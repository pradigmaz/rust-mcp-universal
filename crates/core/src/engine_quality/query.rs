use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::Result;
use rusqlite::params;

use super::compute_quality_status;
use super::metrics::{load_metrics_by_path, load_top_metrics};
use crate::engine::Engine;
use crate::model::{
    QualityCategory, QualityLocation, QualityMode, QualitySeverity, QualitySource, QualityStatus,
    QualityViolationEntry, RuleViolationFileHit, RuleViolationsOptions, RuleViolationsResult,
    RuleViolationsSortBy, RuleViolationsSummary, SuppressedQualityViolationEntry,
    WorkspaceQualityCategoryCount, WorkspaceQualitySeverityCount, WorkspaceQualitySummary,
    WorkspaceQualityTopRule,
};
use crate::quality::QUALITY_RULESET_ID;
use crate::quality::compute_hit_risk_score;

pub(super) fn load_quality_summary(engine: &Engine) -> Result<WorkspaceQualitySummary> {
    if !engine.db_path.exists() {
        return Ok(empty_quality_summary(QualityStatus::Unavailable));
    }
    let status = compute_quality_status(engine)?;
    if status == QualityStatus::Unavailable {
        return Ok(empty_quality_summary(status));
    }
    let conn = engine.open_db_read_only()?;

    let summary = try_load_quality_summary(&conn)
        .unwrap_or_else(|_| empty_quality_summary(QualityStatus::Degraded));
    Ok(WorkspaceQualitySummary { status, ..summary })
}

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
        try_load_rule_violations(&conn, options).unwrap_or_else(|_| RuleViolationsResult {
            summary: empty_rule_violations_summary(QualityStatus::Degraded),
            hits: Vec::new(),
        });
    Ok(RuleViolationsResult {
        summary: RuleViolationsSummary {
            status,
            ..result.summary
        },
        hits: result.hits,
    })
}

fn try_load_quality_summary(conn: &rusqlite::Connection) -> Result<WorkspaceQualitySummary> {
    let evaluated_files: i64 =
        conn.query_row("SELECT COUNT(1) FROM file_quality", [], |row| row.get(0))?;
    let violating_files: i64 = conn.query_row(
        "SELECT COUNT(1) FROM file_quality WHERE quality_violation_count > 0",
        [],
        |row| row.get(0),
    )?;
    let total_violations: i64 = conn.query_row(
        "SELECT COALESCE(SUM(quality_violation_count), 0) FROM file_quality",
        [],
        |row| row.get(0),
    )?;
    let suppressed_violations: i64 = conn.query_row(
        "SELECT COALESCE(SUM(quality_suppressed_violation_count), 0) FROM file_quality",
        [],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        r#"
        SELECT rule_id, COUNT(DISTINCT path) AS file_count, COUNT(1) AS violation_count
        FROM file_rule_violations
        GROUP BY rule_id
        ORDER BY violation_count DESC, rule_id ASC
        LIMIT 5
        "#,
    )?;
    let top_rules = stmt
        .query_map([], |row| {
            Ok(WorkspaceQualityTopRule {
                rule_id: row.get(0)?,
                files: usize::try_from(row.get::<_, i64>(1)?).unwrap_or(usize::MAX),
                violations: usize::try_from(row.get::<_, i64>(2)?).unwrap_or(usize::MAX),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let top_metrics = load_top_metrics(conn)?;
    let severity_breakdown = load_severity_breakdown(conn)?;
    let category_breakdown = load_category_breakdown(conn)?;

    Ok(WorkspaceQualitySummary {
        ruleset_id: QUALITY_RULESET_ID.to_string(),
        status: QualityStatus::Ready,
        evaluated_files: usize::try_from(evaluated_files).unwrap_or(usize::MAX),
        violating_files: usize::try_from(violating_files).unwrap_or(usize::MAX),
        total_violations: usize::try_from(total_violations).unwrap_or(usize::MAX),
        suppressed_violations: usize::try_from(suppressed_violations).unwrap_or(usize::MAX),
        top_rules,
        top_metrics,
        severity_breakdown,
        category_breakdown,
    })
}

fn try_load_rule_violations(
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
    attach_risk_scores(&mut hits);
    let sort_metric_id = options
        .sort_metric_id
        .as_deref()
        .or_else(|| options.metric_ids.first().map(String::as_str));
    let suppressed_violations = hits.iter().map(|hit| hit.suppressed_violations.len()).sum();
    let severity_breakdown = build_severity_breakdown(&hits);
    let category_breakdown = build_category_breakdown(&hits);
    hits.sort_by(|left, right| compare_hits(left, right, options.sort_by, sort_metric_id));
    hits.truncate(options.limit);

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

fn load_quality_candidates(
    conn: &rusqlite::Connection,
    options: &RuleViolationsOptions,
) -> Result<Vec<RuleViolationFileHit>> {
    let path_like = options
        .path_prefix
        .as_ref()
        .map(|prefix| format!("{prefix}%"));
    let mut stmt = conn.prepare(
        r#"
        SELECT path, language, size_bytes, total_lines, non_empty_lines, import_count, quality_mode
        FROM file_quality
        WHERE (?1 IS NULL OR path LIKE ?1)
          AND (?2 IS NULL OR language = ?2)
        "#,
    )?;
    Ok(stmt
        .query_map(params![path_like, options.language.as_ref()], |row| {
            let quality_mode_raw: String = row.get(6)?;
            Ok(RuleViolationFileHit {
                path: row.get(0)?,
                language: row.get(1)?,
                size_bytes: row.get(2)?,
                total_lines: row.get(3)?,
                non_empty_lines: row.get(4)?,
                import_count: row.get(5)?,
                quality_mode: QualityMode::parse(&quality_mode_raw).unwrap_or(QualityMode::Indexed),
                violations: Vec::new(),
                metrics: Vec::new(),
                suppressed_violations: Vec::new(),
                risk_score: None,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn attach_and_filter_violations(
    conn: &rusqlite::Connection,
    candidates: Vec<RuleViolationFileHit>,
    options: &RuleViolationsOptions,
    metrics_by_path: &HashMap<String, Vec<crate::model::QualityMetricValue>>,
    suppressed_by_path: &HashMap<String, Vec<SuppressedQualityViolationEntry>>,
) -> Result<Vec<RuleViolationFileHit>> {
    let path_like = options
        .path_prefix
        .as_ref()
        .map(|prefix| format!("{prefix}%"));
    let rule_filter = options
        .rule_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut stmt = conn.prepare(
        r#"
        SELECT
            q.path,
            v.rule_id,
            v.actual_value,
            v.threshold_value,
            v.message,
            v.severity,
            v.category,
            v.source,
            v.start_line,
            v.start_column,
            v.end_line,
            v.end_column
        FROM file_quality q
        JOIN file_rule_violations v ON v.path = q.path
        WHERE (?1 IS NULL OR q.path LIKE ?1)
          AND (?2 IS NULL OR q.language = ?2)
        ORDER BY q.path ASC, v.rule_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map(params![path_like, options.language.as_ref()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                QualityViolationEntry {
                    rule_id: row.get(1)?,
                    actual_value: row.get(2)?,
                    threshold_value: row.get(3)?,
                    message: row.get(4)?,
                    severity: row
                        .get::<_, String>(5)
                        .ok()
                        .and_then(|value| QualitySeverity::parse(&value))
                        .unwrap_or(QualitySeverity::Medium),
                    category: row
                        .get::<_, String>(6)
                        .ok()
                        .and_then(|value| QualityCategory::parse(&value))
                        .unwrap_or(QualityCategory::Maintainability),
                    source: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|value| QualitySource::parse(&value)),
                    location: violation_location(
                        row.get::<_, Option<i64>>(8)?,
                        row.get::<_, Option<i64>>(9)?,
                        row.get::<_, Option<i64>>(10)?,
                        row.get::<_, Option<i64>>(11)?,
                    ),
                },
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut violations_by_path = HashMap::<String, Vec<QualityViolationEntry>>::new();
    for (path, violation) in rows {
        if !rule_filter.is_empty() && !rule_filter.contains(&violation.rule_id.as_str()) {
            continue;
        }
        violations_by_path.entry(path).or_default().push(violation);
    }

    let mut filtered = Vec::new();
    for mut candidate in candidates {
        if let Some(violations) = violations_by_path.remove(&candidate.path) {
            candidate.violations = violations;
            filtered.push(candidate);
            continue;
        }
        let has_metrics = metrics_by_path.contains_key(&candidate.path);
        let has_suppressed = suppressed_by_path.contains_key(&candidate.path);
        if has_metrics || has_suppressed {
            filtered.push(candidate);
        }
    }
    Ok(filtered)
}

fn attach_metrics_and_suppressed(
    mut candidates: Vec<RuleViolationFileHit>,
    mut metrics_by_path: HashMap<String, Vec<crate::model::QualityMetricValue>>,
    mut suppressed_by_path: HashMap<String, Vec<SuppressedQualityViolationEntry>>,
) -> Vec<RuleViolationFileHit> {
    for candidate in &mut candidates {
        candidate.metrics = metrics_by_path.remove(&candidate.path).unwrap_or_default();
        candidate.suppressed_violations =
            suppressed_by_path.remove(&candidate.path).unwrap_or_default();
    }
    candidates
}

fn attach_risk_scores(hits: &mut [RuleViolationFileHit]) {
    for hit in hits {
        hit.risk_score = Some(compute_hit_risk_score(hit));
    }
}

fn load_suppressed_violations_by_path(
    conn: &rusqlite::Connection,
    options: &RuleViolationsOptions,
) -> Result<HashMap<String, Vec<SuppressedQualityViolationEntry>>> {
    let path_like = options
        .path_prefix
        .as_ref()
        .map(|prefix| format!("{prefix}%"));
    let rule_filter = options
        .rule_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut stmt = conn.prepare(
        r#"
        SELECT q.path, q.suppressed_violations_json
        FROM file_quality q
        WHERE (?1 IS NULL OR q.path LIKE ?1)
          AND (?2 IS NULL OR q.language = ?2)
        "#,
    )?;
    let rows = stmt
        .query_map(params![path_like, options.language.as_ref()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut out = HashMap::with_capacity(rows.len());
    for (path, payload) in rows {
        let mut suppressed = parse_suppressed_violations_json(&payload)?;
        if !rule_filter.is_empty() {
            suppressed.retain(|entry| rule_filter.contains(&entry.violation.rule_id.as_str()));
        }
        if !suppressed.is_empty() {
            out.insert(path, suppressed);
        }
    }
    Ok(out)
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
    primary
        .then_with(|| right.size_bytes.cmp(&left.size_bytes))
        .then_with(|| left.path.cmp(&right.path))
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

fn violation_location(
    start_line: Option<i64>,
    start_column: Option<i64>,
    end_line: Option<i64>,
    end_column: Option<i64>,
) -> Option<QualityLocation> {
    Some(QualityLocation {
        start_line: usize::try_from(start_line?).ok()?,
        start_column: usize::try_from(start_column?).ok()?,
        end_line: usize::try_from(end_line?).ok()?,
        end_column: usize::try_from(end_column?).ok()?,
    })
}

fn parse_suppressed_violations_json(payload: &str) -> Result<Vec<SuppressedQualityViolationEntry>> {
    if payload.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(serde_json::from_str(payload)?)
}

fn load_severity_breakdown(
    conn: &rusqlite::Connection,
) -> Result<Vec<WorkspaceQualitySeverityCount>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT severity, COUNT(DISTINCT path) AS file_count, COUNT(1) AS violation_count
        FROM file_rule_violations
        GROUP BY severity
        ORDER BY violation_count DESC, severity ASC
        "#,
    )?;
    stmt.query_map([], |row| {
        Ok(WorkspaceQualitySeverityCount {
            severity: row
                .get::<_, String>(0)
                .ok()
                .and_then(|value| QualitySeverity::parse(&value))
                .unwrap_or(QualitySeverity::Medium),
            files: usize::try_from(row.get::<_, i64>(1)?).unwrap_or(usize::MAX),
            violations: usize::try_from(row.get::<_, i64>(2)?).unwrap_or(usize::MAX),
        })
    })?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(Into::into)
}

fn load_category_breakdown(
    conn: &rusqlite::Connection,
) -> Result<Vec<WorkspaceQualityCategoryCount>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT category, COUNT(DISTINCT path) AS file_count, COUNT(1) AS violation_count
        FROM file_rule_violations
        GROUP BY category
        ORDER BY violation_count DESC, category ASC
        "#,
    )?;
    stmt.query_map([], |row| {
        Ok(WorkspaceQualityCategoryCount {
            category: row
                .get::<_, String>(0)
                .ok()
                .and_then(|value| QualityCategory::parse(&value))
                .unwrap_or(QualityCategory::Maintainability),
            files: usize::try_from(row.get::<_, i64>(1)?).unwrap_or(usize::MAX),
            violations: usize::try_from(row.get::<_, i64>(2)?).unwrap_or(usize::MAX),
        })
    })?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(Into::into)
}

fn build_severity_breakdown(hits: &[RuleViolationFileHit]) -> Vec<WorkspaceQualitySeverityCount> {
    let mut counts = BTreeMap::<QualitySeverity, (usize, BTreeSet<String>)>::new();
    for hit in hits {
        for violation in &hit.violations {
            let entry = counts
                .entry(violation.severity)
                .or_insert_with(|| (0, BTreeSet::new()));
            entry.0 += 1;
            entry.1.insert(hit.path.clone());
        }
    }
    counts
        .into_iter()
        .map(|(severity, (violations, files))| WorkspaceQualitySeverityCount {
            severity,
            files: files.len(),
            violations,
        })
        .collect()
}

fn build_category_breakdown(hits: &[RuleViolationFileHit]) -> Vec<WorkspaceQualityCategoryCount> {
    let mut counts = BTreeMap::<QualityCategory, (usize, BTreeSet<String>)>::new();
    for hit in hits {
        for violation in &hit.violations {
            let entry = counts
                .entry(violation.category)
                .or_insert_with(|| (0, BTreeSet::new()));
            entry.0 += 1;
            entry.1.insert(hit.path.clone());
        }
    }
    counts
        .into_iter()
        .map(|(category, (violations, files))| WorkspaceQualityCategoryCount {
            category,
            files: files.len(),
            violations,
        })
        .collect()
}

fn empty_quality_summary(status: QualityStatus) -> WorkspaceQualitySummary {
    WorkspaceQualitySummary {
        ruleset_id: QUALITY_RULESET_ID.to_string(),
        status,
        evaluated_files: 0,
        violating_files: 0,
        total_violations: 0,
        suppressed_violations: 0,
        top_rules: Vec::new(),
        top_metrics: Vec::new(),
        severity_breakdown: Vec::new(),
        category_breakdown: Vec::new(),
    }
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
