use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::model::{
    QualityDeltaSummary, QualityHotspotBucket, QualityHotspotsResult, QualityProjectDeltaReport,
    QualityProjectGateStatus, QualityProjectHotspotDelta, QualityProjectSnapshotCompareAgainst,
    QualityProjectSnapshotReport, QualityStatus, RuleViolationsResult,
};

pub(super) fn build_delta_report(
    baseline: &QualityProjectSnapshotReport,
    candidate: &QualityProjectSnapshotReport,
    compare_against: QualityProjectSnapshotCompareAgainst,
) -> QualityProjectDeltaReport {
    let (new_violations, resolved_violations) = compare_violation_multisets(
        &baseline.rule_violations_by_violation_count,
        &candidate.rule_violations_by_violation_count,
    );
    let mut regression_reasons = Vec::new();
    if candidate.quality_status_after_refresh != QualityStatus::Ready {
        regression_reasons.push(format!(
            "post_refresh_status={}",
            candidate.quality_status_after_refresh.as_str()
        ));
    }
    if new_violations > 0 {
        regression_reasons.push(format!("new_violations={new_violations}"));
    }

    QualityProjectDeltaReport {
        generated_at_utc: now_rfc3339().unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string()),
        compare_against,
        baseline_generated_at_utc: baseline.generated_at_utc.clone(),
        candidate_generated_at_utc: candidate.generated_at_utc.clone(),
        total_violations_delta: candidate.total_violations as i64
            - baseline.total_violations as i64,
        violating_files_delta: candidate.violating_files as i64 - baseline.violating_files as i64,
        suppressed_violations_delta: candidate.suppressed_violations as i64
            - baseline.suppressed_violations as i64,
        total_non_empty_lines_delta: candidate.total_non_empty_lines
            - baseline.total_non_empty_lines,
        total_size_bytes_delta: candidate.total_size_bytes - baseline.total_size_bytes,
        new_violations,
        resolved_violations,
        file_hotspots: compare_hotspot_results(&baseline.file_hotspots, &candidate.file_hotspots),
        directory_hotspots: compare_hotspot_results(
            &baseline.directory_hotspots,
            &candidate.directory_hotspots,
        ),
        module_hotspots: compare_hotspot_results(
            &baseline.module_hotspots,
            &candidate.module_hotspots,
        ),
        gate_status: if regression_reasons.is_empty() {
            QualityProjectGateStatus::Ok
        } else {
            QualityProjectGateStatus::Regression
        },
        regression_reasons,
    }
}

pub(super) fn now_rfc3339() -> Result<String> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

fn compare_violation_multisets(
    baseline: &RuleViolationsResult,
    candidate: &RuleViolationsResult,
) -> (usize, usize) {
    let mut counts = BTreeMap::<String, i64>::new();

    for hit in &candidate.hits {
        for violation in &hit.violations {
            *counts
                .entry(violation_fingerprint(&hit.path, violation))
                .or_default() += 1;
        }
    }
    for hit in &baseline.hits {
        for violation in &hit.violations {
            *counts
                .entry(violation_fingerprint(&hit.path, violation))
                .or_default() -= 1;
        }
    }

    let mut new_violations = 0usize;
    let mut resolved_violations = 0usize;
    for delta in counts.into_values() {
        if delta > 0 {
            new_violations += delta as usize;
        } else if delta < 0 {
            resolved_violations += (-delta) as usize;
        }
    }
    (new_violations, resolved_violations)
}

fn compare_hotspot_results(
    baseline: &QualityHotspotsResult,
    candidate: &QualityHotspotsResult,
) -> QualityProjectHotspotDelta {
    let mut baseline_buckets = baseline
        .buckets
        .iter()
        .cloned()
        .map(|bucket| (bucket.bucket_id.clone(), bucket))
        .collect::<HashMap<_, _>>();

    let mut delta = QualityProjectHotspotDelta::default();
    for bucket in &candidate.buckets {
        let current_delta =
            build_bucket_delta(bucket, baseline_buckets.remove(&bucket.bucket_id).as_ref());
        delta.new_violations += current_delta.new_violations;
        delta.resolved_violations += current_delta.resolved_violations;
        delta.risk_score_delta_total += current_delta.risk_score_delta;
        delta.hotspot_score_delta_total += current_delta.hotspot_score_delta;
    }

    for previous in baseline_buckets.into_values() {
        delta.resolved_violations += previous.active_violation_count;
        delta.hotspot_score_delta_total -= previous.hotspot_score;
        delta.risk_score_delta_total -= previous.risk_score.map(|risk| risk.score).unwrap_or(0.0);
    }

    delta
}

fn build_bucket_delta(
    current: &QualityHotspotBucket,
    previous: Option<&QualityHotspotBucket>,
) -> QualityDeltaSummary {
    let previous_rule_counts = previous
        .map(|bucket| {
            bucket
                .rule_counts
                .iter()
                .map(|entry| (entry.rule_id.as_str(), entry.violations))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let current_rule_counts = current
        .rule_counts
        .iter()
        .map(|entry| (entry.rule_id.as_str(), entry.violations))
        .collect::<HashMap<_, _>>();

    let mut new_violations = 0usize;
    let mut resolved_violations = 0usize;
    let mut regressed_rules = Vec::new();

    for (rule_id, current_count) in &current_rule_counts {
        let previous_count = previous_rule_counts.get(rule_id).copied().unwrap_or(0);
        if *current_count > previous_count {
            new_violations += current_count - previous_count;
            regressed_rules.push((*rule_id).to_string());
        }
    }
    for (rule_id, previous_count) in previous_rule_counts {
        let current_count = current_rule_counts.get(rule_id).copied().unwrap_or(0);
        if previous_count > current_count {
            resolved_violations += previous_count - current_count;
        }
    }
    regressed_rules.sort();

    let previous_risk_score = previous.and_then(|bucket| bucket.risk_score.map(|risk| risk.score));
    let current_risk_score = current.risk_score.map(|risk| risk.score);

    QualityDeltaSummary {
        new_violations,
        resolved_violations,
        risk_score_delta: match (current_risk_score, previous_risk_score) {
            (Some(current_score), Some(previous_score)) => current_score - previous_score,
            (Some(current_score), None) => current_score,
            _ => 0.0,
        },
        hotspot_score_delta: previous
            .map(|bucket| current.hotspot_score - bucket.hotspot_score)
            .unwrap_or(current.hotspot_score),
        new_hotspot: previous.is_none()
            && (current.active_violation_count > 0 || current.hotspot_score > 0.0),
        regressed_rules,
    }
}

fn violation_fingerprint(path: &str, violation: &crate::model::QualityViolationEntry) -> String {
    let location = violation
        .location
        .as_ref()
        .map(|entry| {
            format!(
                "{}:{}:{}:{}",
                entry.start_line, entry.start_column, entry.end_line, entry.end_column
            )
        })
        .unwrap_or_default();
    let source = violation.source.map(|entry| entry.as_str()).unwrap_or("");
    format!(
        "{path}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{location}\u{001f}{source}",
        violation.rule_id,
        violation.actual_value,
        violation.threshold_value,
        violation.message,
        violation.severity.as_str(),
        violation.category.as_str(),
        violation.location.is_some()
    )
}
