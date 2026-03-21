use std::collections::HashMap;

use anyhow::{Result, anyhow};

use crate::quality::{quality_metrics_hash, violations_hash};

#[derive(Debug, Clone)]
pub(in crate::engine) struct ActualQualityState {
    pub(in crate::engine) metric_count: i64,
    pub(in crate::engine) metric_hash: String,
    pub(in crate::engine) violation_count: i64,
    pub(in crate::engine) violation_hash: String,
}

impl Default for ActualQualityState {
    fn default() -> Self {
        Self {
            metric_count: 0,
            metric_hash: quality_metrics_hash(&[]),
            violation_count: 0,
            violation_hash: violations_hash(&[]),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExistingQualityState {
    pub(in crate::engine) source_mtime_unix_ms: Option<i64>,
    pub(in crate::engine) quality_ruleset_version: i64,
    pub(in crate::engine) quality_metric_count: i64,
    pub(in crate::engine) quality_metric_hash: String,
    pub(in crate::engine) quality_violation_count: i64,
    pub(in crate::engine) quality_violation_hash: String,
    pub(in crate::engine) actual_quality_metric_count: i64,
    pub(in crate::engine) actual_quality_metric_hash: String,
    pub(in crate::engine) actual_quality_violation_count: i64,
    pub(in crate::engine) actual_quality_violation_hash: String,
}

impl ExistingQualityState {
    pub(crate) fn is_complete(&self, expected_ruleset_version: i64) -> bool {
        self.quality_ruleset_version == expected_ruleset_version
            && self.quality_metric_count == self.actual_quality_metric_count
            && self.quality_metric_hash == self.actual_quality_metric_hash
            && self.quality_violation_count == self.actual_quality_violation_count
            && self.quality_violation_hash == self.actual_quality_violation_hash
    }
}

pub(in crate::engine) fn load_actual_quality_state(
    tx: &rusqlite::Transaction<'_>,
) -> Result<HashMap<String, ActualQualityState>> {
    let mut metric_stmt = tx.prepare(
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
            });
    }

    let mut stmt = tx.prepare(
        r#"
        SELECT path, rule_id, actual_value, threshold_value, message
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
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut grouped = HashMap::<String, Vec<crate::model::QualityViolationEntry>>::new();
    for (path, rule_id, actual_value, threshold_value, message) in rows {
        grouped
            .entry(path)
            .or_default()
            .push(crate::model::QualityViolationEntry {
                rule_id,
                actual_value,
                threshold_value,
                message,
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

pub(in crate::engine) fn load_existing_quality_state(
    tx: &rusqlite::Transaction<'_>,
) -> Result<HashMap<String, ExistingQualityState>> {
    let actual_quality_state = load_actual_quality_state(tx)?;
    let mut stmt = tx.prepare(
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
