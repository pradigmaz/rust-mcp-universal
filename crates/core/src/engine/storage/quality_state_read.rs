use std::collections::HashMap;

use anyhow::{Result, anyhow};

use super::quality_state::{ActualQualityState, ExistingQualityState};
use crate::quality::{quality_metrics_hash, violations_hash};

pub(crate) fn load_existing_quality_state_conn(
    conn: &rusqlite::Connection,
) -> Result<HashMap<String, ExistingQualityState>> {
    let actual_quality_state = load_actual_quality_state_conn(conn)?;
    let mut stmt = conn.prepare(
        r#"
        SELECT
            path,
            source_mtime_unix_ms,
            quality_mode,
            quality_ruleset_version,
            quality_metric_count,
            quality_metric_hash,
            quality_violation_count,
            quality_violation_hash
        FROM file_quality
        "#,
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut out = HashMap::with_capacity(rows.len());
    for (
        path,
        source_mtime_unix_ms,
        quality_mode_raw,
        quality_ruleset_version,
        quality_metric_count,
        quality_metric_hash,
        quality_violation_count,
        quality_violation_hash,
    ) in rows
    {
        if quality_mode_raw != "indexed" && quality_mode_raw != "quality-only-oversize" {
            return Err(anyhow!(
                "file_quality contains unknown quality_mode `{quality_mode_raw}`"
            ));
        }
        let actual_state = actual_quality_state.get(&path).cloned().unwrap_or_default();
        out.insert(
            path,
            ExistingQualityState {
                source_mtime_unix_ms,
                quality_ruleset_version,
                quality_metric_count,
                quality_metric_hash,
                quality_violation_count,
                quality_violation_hash,
                actual_quality_metric_count: actual_state.metric_count,
                actual_quality_metric_hash: actual_state.metric_hash,
                actual_quality_violation_count: actual_state.violation_count,
                actual_quality_violation_hash: actual_state.violation_hash,
            },
        );
    }
    Ok(out)
}

fn load_actual_quality_state_conn(
    conn: &rusqlite::Connection,
) -> Result<HashMap<String, ActualQualityState>> {
    let mut metric_stmt = conn.prepare(
        r#"
        SELECT path, metric_id, metric_value
        FROM file_quality_metrics
        ORDER BY path ASC, metric_id ASC
        "#,
    )?;
    let metric_rows = metric_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut metrics_grouped = HashMap::<String, Vec<crate::quality::QualityMetricEntry>>::new();
    for (path, metric_id, metric_value) in metric_rows {
        metrics_grouped
            .entry(path)
            .or_default()
            .push(crate::quality::QualityMetricEntry {
                metric_id,
                metric_value,
                location: None,
            });
    }

    let mut stmt = conn.prepare(
        r#"
        SELECT
            path,
            rule_id,
            actual_value,
            threshold_value,
            message,
            start_line,
            start_column,
            end_line,
            end_column
        FROM file_rule_violations
        ORDER BY path ASC, rule_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<i64>>(8)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut grouped = HashMap::<String, Vec<crate::model::QualityViolationEntry>>::new();
    for (
        path,
        rule_id,
        actual_value,
        threshold_value,
        message,
        start_line,
        start_column,
        end_line,
        end_column,
    ) in rows
    {
        grouped
            .entry(path)
            .or_default()
            .push(crate::model::QualityViolationEntry {
                rule_id,
                actual_value,
                threshold_value,
                message,
                location: build_location(start_line, start_column, end_line, end_column),
            });
    }

    let mut out = HashMap::with_capacity(grouped.len());
    for path in metrics_grouped.keys().chain(grouped.keys()) {
        if out.contains_key(path) {
            continue;
        }
        let metrics = metrics_grouped.get(path).cloned().unwrap_or_default();
        let violations = grouped.get(path).cloned().unwrap_or_default();
        out.insert(
            path.clone(),
            ActualQualityState {
                metric_count: i64::try_from(metrics.len()).unwrap_or(i64::MAX),
                metric_hash: quality_metrics_hash(&metrics),
                violation_count: i64::try_from(violations.len()).unwrap_or(i64::MAX),
                violation_hash: violations_hash(&violations),
            },
        );
    }
    Ok(out)
}

fn build_location(
    start_line: Option<i64>,
    start_column: Option<i64>,
    end_line: Option<i64>,
    end_column: Option<i64>,
) -> Option<crate::model::QualityLocation> {
    Some(crate::model::QualityLocation {
        start_line: usize::try_from(start_line?).ok()?,
        start_column: usize::try_from(start_column?).ok()?,
        end_line: usize::try_from(end_line?).ok()?,
        end_column: usize::try_from(end_column?).ok()?,
    })
}
