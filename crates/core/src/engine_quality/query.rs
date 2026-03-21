use std::cmp::Ordering;
use std::collections::HashMap;

use anyhow::Result;
use rusqlite::params;

use super::status::read_quality_status;
use crate::engine::Engine;
use crate::model::{
    QualityMode, QualityStatus, QualityViolationEntry, RuleViolationFileHit, RuleViolationsOptions,
    RuleViolationsResult, RuleViolationsSortBy, RuleViolationsSummary, WorkspaceQualitySummary,
    WorkspaceQualityTopRule,
};
use crate::quality::QUALITY_RULESET_ID;

pub(super) fn load_quality_summary(engine: &Engine) -> Result<WorkspaceQualitySummary> {
    if !engine.db_path.exists() {
        return Ok(empty_quality_summary(QualityStatus::Unavailable));
    }
    let conn = engine.open_db_read_only()?;
    let status = read_quality_status(&conn)?;
    if status == QualityStatus::Unavailable {
        return Ok(empty_quality_summary(status));
    }

    let summary = try_load_quality_summary(&conn).unwrap_or_else(|_| empty_quality_summary(QualityStatus::Degraded));
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

    let conn = engine.open_db_read_only()?;
    let status = read_quality_status(&conn)?;
    if status == QualityStatus::Unavailable {
        return Ok(RuleViolationsResult {
            summary: empty_rule_violations_summary(status),
            hits: Vec::new(),
        });
    }

    let result = try_load_rule_violations(&conn, options)
        .unwrap_or_else(|_| RuleViolationsResult {
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

    Ok(WorkspaceQualitySummary {
        ruleset_id: QUALITY_RULESET_ID.to_string(),
        status: QualityStatus::Ready,
        evaluated_files: usize::try_from(evaluated_files).unwrap_or(usize::MAX),
        violating_files: usize::try_from(violating_files).unwrap_or(usize::MAX),
        total_violations: usize::try_from(total_violations).unwrap_or(usize::MAX),
        top_rules,
    })
}

fn try_load_rule_violations(
    conn: &rusqlite::Connection,
    options: &RuleViolationsOptions,
) -> Result<RuleViolationsResult> {
    let candidates = load_quality_candidates(conn, options)?;
    let evaluated_files = candidates.len();
    let filtered = attach_and_filter_violations(conn, candidates, options)?;
    let mut hits = filtered;
    hits.sort_by(|left, right| compare_hits(left, right, options.sort_by));
    hits.truncate(options.limit);

    Ok(RuleViolationsResult {
        summary: RuleViolationsSummary {
            ruleset_id: QUALITY_RULESET_ID.to_string(),
            status: QualityStatus::Ready,
            evaluated_files,
            violating_files: hits.len(),
            total_violations: hits.iter().map(|hit| hit.violations.len()).sum(),
        },
        hits,
    })
}

fn load_quality_candidates(
    conn: &rusqlite::Connection,
    options: &RuleViolationsOptions,
) -> Result<Vec<RuleViolationFileHit>> {
    let path_like = options.path_prefix.as_ref().map(|prefix| format!("{prefix}%"));
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
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn attach_and_filter_violations(
    conn: &rusqlite::Connection,
    candidates: Vec<RuleViolationFileHit>,
    options: &RuleViolationsOptions,
) -> Result<Vec<RuleViolationFileHit>> {
    let path_like = options.path_prefix.as_ref().map(|prefix| format!("{prefix}%"));
    let rule_filter = options.rule_ids.iter().map(String::as_str).collect::<Vec<_>>();
    let mut stmt = conn.prepare(
        r#"
        SELECT q.path, v.rule_id, v.actual_value, v.threshold_value, v.message
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
        } else if rule_filter.is_empty() {
            continue;
        }
    }
    Ok(filtered)
}

fn compare_hits(
    left: &RuleViolationFileHit,
    right: &RuleViolationFileHit,
    sort_by: RuleViolationsSortBy,
) -> Ordering {
    let primary = match sort_by {
        RuleViolationsSortBy::ViolationCount => right.violations.len().cmp(&left.violations.len()),
        RuleViolationsSortBy::SizeBytes => right.size_bytes.cmp(&left.size_bytes),
        RuleViolationsSortBy::NonEmptyLines => right
            .non_empty_lines
            .unwrap_or(i64::MIN)
            .cmp(&left.non_empty_lines.unwrap_or(i64::MIN)),
    };
    primary
        .then_with(|| right.size_bytes.cmp(&left.size_bytes))
        .then_with(|| left.path.cmp(&right.path))
}

fn empty_quality_summary(status: QualityStatus) -> WorkspaceQualitySummary {
    WorkspaceQualitySummary {
        ruleset_id: QUALITY_RULESET_ID.to_string(),
        status,
        evaluated_files: 0,
        violating_files: 0,
        total_violations: 0,
        top_rules: Vec::new(),
    }
}

fn empty_rule_violations_summary(status: QualityStatus) -> RuleViolationsSummary {
    RuleViolationsSummary {
        ruleset_id: QUALITY_RULESET_ID.to_string(),
        status,
        evaluated_files: 0,
        violating_files: 0,
        total_violations: 0,
    }
}
