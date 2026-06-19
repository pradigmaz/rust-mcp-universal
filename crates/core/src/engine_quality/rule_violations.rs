use std::cmp::Ordering;
use std::collections::HashMap;

use anyhow::Result;

use super::breakdown::{build_category_breakdown, build_severity_breakdown};
use super::compute_quality_status;
use super::metrics::load_metrics_by_path;
use super::rule_violation_rows::{
    attach_and_filter_violations, load_quality_candidates, load_suppressed_violations_by_path,
};
use crate::engine::Engine;
use crate::model::{
    QualityMetricValue, QualityStatus, RuleViolationFileHit, RuleViolationsOptions,
    RuleViolationsResult, RuleViolationsSortBy, RuleViolationsSummary,
    SuppressedQualityViolationEntry,
};
use crate::quality::QUALITY_RULESET_ID;
use crate::quality::compute_hit_risk_score;
use crate::text_utils::is_low_priority_path;

pub(super) fn load_rule_violations(
    engine: &Engine,
    options: &RuleViolationsOptions,
) -> Result<RuleViolationsResult> {
    if !engine.db_path.exists() {
        return Ok(RuleViolationsResult {
            summary: empty_rule_violations_summary(QualityStatus::Unavailable),
            hits: Vec::new(),
        });
    }

    let status = compute_quality_status(engine)?;
    if status == QualityStatus::Unavailable {
        return Ok(RuleViolationsResult {
            summary: empty_rule_violations_summary(status),
            hits: Vec::new(),
        });
    }
    let conn = engine.open_db_read_only()?;

    let result =
        try_load_rule_violations(&engine.project_root, &conn, options).unwrap_or_else(|_| {
            RuleViolationsResult {
                summary: empty_rule_violations_summary(QualityStatus::Degraded),
                hits: Vec::new(),
            }
        });
    Ok(RuleViolationsResult {
        summary: RuleViolationsSummary {
            status,
            ..result.summary
        },
        hits: result.hits,
    })
}

fn try_load_rule_violations(
    project_root: &std::path::Path,
    conn: &rusqlite::Connection,
    options: &RuleViolationsOptions,
) -> Result<RuleViolationsResult> {
    let candidates = load_quality_candidates(conn, options)?;
    let evaluated_files = candidates.len();
    let metrics_by_path = load_metrics_by_path(conn, options)?;
    let suppressed_by_path = load_suppressed_violations_by_path(conn, options)?;
    let filtered = attach_and_filter_violations(
        conn,
        candidates,
        options,
        &metrics_by_path,
        &suppressed_by_path,
    )?;
    let mut hits = attach_metrics_and_suppressed(filtered, metrics_by_path, suppressed_by_path);
    attach_signal_memory(project_root, &mut hits);
    attach_risk_scores(&mut hits);
    let sort_metric_id = options
        .sort_metric_id
        .as_deref()
        .or_else(|| options.metric_ids.first().map(String::as_str));
    hits.sort_by(|left, right| compare_hits(left, right, options.sort_by, sort_metric_id));
    hits.truncate(options.limit);
    let suppressed_violations = hits.iter().map(|hit| hit.suppressed_violations.len()).sum();
    let severity_breakdown = build_severity_breakdown(&hits);
    let category_breakdown = build_category_breakdown(&hits);

    Ok(RuleViolationsResult {
        summary: RuleViolationsSummary {
            ruleset_id: QUALITY_RULESET_ID.to_string(),
            status: QualityStatus::Ready,
            evaluated_files,
            violating_files: hits.iter().filter(|hit| !hit.violations.is_empty()).count(),
            total_violations: hits.iter().map(|hit| hit.violations.len()).sum(),
            suppressed_violations,
            severity_breakdown,
            category_breakdown,
        },
        hits,
    })
}

fn attach_metrics_and_suppressed(
    mut candidates: Vec<RuleViolationFileHit>,
    mut metrics_by_path: HashMap<String, Vec<QualityMetricValue>>,
    mut suppressed_by_path: HashMap<String, Vec<SuppressedQualityViolationEntry>>,
) -> Vec<RuleViolationFileHit> {
    for candidate in &mut candidates {
        candidate.metrics = metrics_by_path.remove(&candidate.path).unwrap_or_default();
        candidate.suppressed_violations = suppressed_by_path
            .remove(&candidate.path)
            .unwrap_or_default();
    }
    candidates
}

fn attach_risk_scores(hits: &mut [RuleViolationFileHit]) {
    for hit in hits {
        hit.risk_score = Some(compute_hit_risk_score(hit));
    }
}

fn attach_signal_memory(project_root: &std::path::Path, hits: &mut [RuleViolationFileHit]) {
    let memory = crate::signal_memory::load_signal_memory(project_root).unwrap_or_default();
    for hit in hits {
        for violation in &mut hit.violations {
            let signal_key = crate::signal_memory::build_quality_signal_key(&hit.path, violation);
            violation.memory_status =
                crate::signal_memory::signal_memory_status(&memory, &signal_key);
            violation.signal_key = Some(signal_key);
        }
        for suppressed in &mut hit.suppressed_violations {
            let signal_key =
                crate::signal_memory::build_quality_signal_key(&hit.path, &suppressed.violation);
            suppressed.violation.memory_status =
                crate::signal_memory::signal_memory_status(&memory, &signal_key);
            suppressed.violation.signal_key = Some(signal_key);
        }
    }
}

fn compare_hits(
    left: &RuleViolationFileHit,
    right: &RuleViolationFileHit,
    sort_by: RuleViolationsSortBy,
    sort_metric_id: Option<&str>,
) -> Ordering {
    let primary = match sort_by {
        RuleViolationsSortBy::ViolationCount => right.violations.len().cmp(&left.violations.len()),
        RuleViolationsSortBy::SizeBytes => right.size_bytes.cmp(&left.size_bytes),
        RuleViolationsSortBy::NonEmptyLines => right
            .non_empty_lines
            .unwrap_or(i64::MIN)
            .cmp(&left.non_empty_lines.unwrap_or(i64::MIN)),
        RuleViolationsSortBy::MetricValue => {
            metric_value_for(right, sort_metric_id).cmp(&metric_value_for(left, sort_metric_id))
        }
    };
    is_low_priority_path(&left.path)
        .cmp(&is_low_priority_path(&right.path))
        .then(primary)
        .then_with(|| warning_prominence(right).cmp(&warning_prominence(left)))
        .then_with(|| right.size_bytes.cmp(&left.size_bytes))
        .then_with(|| left.path.cmp(&right.path))
}

fn warning_prominence(hit: &RuleViolationFileHit) -> usize {
    hit.violations
        .iter()
        .map(|violation| match violation.memory_status {
            Some(crate::model::SignalMemoryStatus::RememberedUseful) => 3,
            Some(crate::model::SignalMemoryStatus::RememberedNoisy) => {
                if violation.confidence == Some(crate::model::FindingConfidence::High) {
                    2
                } else {
                    0
                }
            }
            None | Some(crate::model::SignalMemoryStatus::Unknown) => 2,
        })
        .sum()
}

fn metric_value_for(hit: &RuleViolationFileHit, metric_id: Option<&str>) -> i64 {
    if let Some(metric_id) = metric_id {
        return hit
            .metrics
            .iter()
            .find(|metric| metric.metric_id == metric_id)
            .map(|metric| metric.metric_value)
            .unwrap_or(i64::MIN);
    }

    hit.metrics
        .iter()
        .map(|metric| metric.metric_value)
        .max()
        .unwrap_or(i64::MIN)
}

fn empty_rule_violations_summary(status: QualityStatus) -> RuleViolationsSummary {
    RuleViolationsSummary {
        ruleset_id: QUALITY_RULESET_ID.to_string(),
        status,
        evaluated_files: 0,
        violating_files: 0,
        total_violations: 0,
        suppressed_violations: 0,
        severity_breakdown: Vec::new(),
        category_breakdown: Vec::new(),
    }
}
