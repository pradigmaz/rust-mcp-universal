use anyhow::Result;

use super::breakdown::{load_category_breakdown, load_severity_breakdown};
use super::compute_quality_status;
use super::metrics::load_top_metrics;
use crate::engine::Engine;
use crate::model::{QualityStatus, WorkspaceQualitySummary, WorkspaceQualityTopRule};
use crate::quality::QUALITY_RULESET_ID;

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
