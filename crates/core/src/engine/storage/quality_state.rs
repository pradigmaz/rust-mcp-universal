use std::collections::HashMap;

use anyhow::{Result, anyhow};

use crate::quality::{quality_metrics_hash, suppressed_violations_hash, violations_hash};

#[derive(Debug, Clone)]
pub(in crate::engine) struct ActualQualityState {
    pub(in crate::engine) metric_count: i64,
    pub(in crate::engine) metric_hash: String,
    pub(in crate::engine) violation_count: i64,
    pub(in crate::engine) violation_hash: String,
    pub(in crate::engine) suppressed_violation_count: i64,
    pub(in crate::engine) suppressed_violation_hash: String,
}

impl Default for ActualQualityState {
    fn default() -> Self {
        Self {
            metric_count: 0,
            metric_hash: quality_metrics_hash(&[]),
            violation_count: 0,
            violation_hash: violations_hash(&[]),
            suppressed_violation_count: 0,
            suppressed_violation_hash: suppressed_violations_hash(&[]),
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
    pub(in crate::engine) quality_suppressed_violation_count: i64,
    pub(in crate::engine) quality_suppressed_violation_hash: String,
    pub(in crate::engine) actual_quality_metric_count: i64,
    pub(in crate::engine) actual_quality_metric_hash: String,
    pub(in crate::engine) actual_quality_violation_count: i64,
    pub(in crate::engine) actual_quality_violation_hash: String,
    pub(in crate::engine) actual_quality_suppressed_violation_count: i64,
    pub(in crate::engine) actual_quality_suppressed_violation_hash: String,
}

impl ExistingQualityState {
    pub(crate) fn is_complete(&self, expected_ruleset_version: i64) -> bool {
        self.quality_ruleset_version == expected_ruleset_version
            && self.quality_metric_count == self.actual_quality_metric_count
            && self.quality_metric_hash == self.actual_quality_metric_hash
            && self.quality_violation_count == self.actual_quality_violation_count
            && self.quality_violation_hash == self.actual_quality_violation_hash
            && self.quality_suppressed_violation_count
                == self.actual_quality_suppressed_violation_count
            && self.quality_suppressed_violation_hash
                == self.actual_quality_suppressed_violation_hash
    }
}

pub(in crate::engine) fn load_actual_quality_state(
    tx: &rusqlite::Transaction<'_>,
) -> Result<HashMap<String, ActualQualityState>> {
    let mut metric_stmt = tx.prepare(
        r#"
        SELECT
            path,
            metric_id,
            metric_value,
            source,
            start_line,
            start_column,
            end_line,
            end_column
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
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<i64>>(7)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut metrics_grouped = HashMap::<String, Vec<crate::quality::QualityMetricEntry>>::new();
    for (path, metric_id, metric_value, source, start_line, start_column, end_line, end_column) in
        metric_rows
    {
        metrics_grouped
            .entry(path)
            .or_default()
            .push(crate::quality::QualityMetricEntry {
                metric_id,
                metric_value,
                location: build_location(start_line, start_column, end_line, end_column),
                source: source
                    .as_deref()
                    .and_then(crate::model::QualitySource::parse),
            });
    }

    let mut stmt = tx.prepare(
        r#"
        SELECT
            path,
            rule_id,
            actual_value,
            threshold_value,
            message,
            severity,
            category,
            source,
            finding_family,
            confidence,
            manual_review_required,
            noise_reason,
            recommended_followups_json,
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
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, Option<i64>>(13)?,
                row.get::<_, Option<i64>>(14)?,
                row.get::<_, Option<i64>>(15)?,
                row.get::<_, Option<i64>>(16)?,
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
        severity,
        category,
        source,
        finding_family,
        confidence,
        manual_review_required,
        noise_reason,
        recommended_followups_json,
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
                severity: crate::model::QualitySeverity::parse(&severity)
                    .unwrap_or(crate::model::QualitySeverity::Medium),
                category: crate::model::QualityCategory::parse(&category)
                    .unwrap_or(crate::model::QualityCategory::Maintainability),
                location: build_location(start_line, start_column, end_line, end_column),
                source: source
                    .as_deref()
                    .and_then(crate::model::QualitySource::parse),
                finding_family: finding_family
                    .as_deref()
                    .and_then(crate::model::FindingFamily::parse),
                confidence: confidence
                    .as_deref()
                    .and_then(crate::model::FindingConfidence::parse),
                manual_review_required: manual_review_required != 0,
                noise_reason,
                recommended_followups: serde_json::from_str(&recommended_followups_json)
                    .unwrap_or_default(),
                signal_key: None,
                memory_status: None,
            });
    }

    let mut suppressed_grouped =
        HashMap::<String, Vec<crate::model::SuppressedQualityViolationEntry>>::new();
    let mut suppressed_stmt = tx.prepare(
        r#"
        SELECT path, suppressed_violations_json
        FROM file_quality
        ORDER BY path ASC
        "#,
    )?;
    let suppressed_rows = suppressed_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (path, payload) in suppressed_rows {
        suppressed_grouped.insert(path, parse_suppressed_violations_json(&payload)?);
    }

    let mut out = HashMap::with_capacity(grouped.len());
    for path in metrics_grouped
        .keys()
        .chain(grouped.keys())
        .chain(suppressed_grouped.keys())
    {
        if out.contains_key(path) {
            continue;
        }
        let metrics = metrics_grouped.get(path).cloned().unwrap_or_default();
        let violations = grouped.get(path).cloned().unwrap_or_default();
        let suppressed = suppressed_grouped.get(path).cloned().unwrap_or_default();
        out.insert(
            path.clone(),
            ActualQualityState {
                metric_count: i64::try_from(metrics.len()).unwrap_or(i64::MAX),
                metric_hash: quality_metrics_hash(&metrics),
                violation_count: i64::try_from(violations.len()).unwrap_or(i64::MAX),
                violation_hash: violations_hash(&violations),
                suppressed_violation_count: i64::try_from(suppressed.len()).unwrap_or(i64::MAX),
                suppressed_violation_hash: suppressed_violations_hash(&suppressed),
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
            quality_violation_hash,
            quality_suppressed_violation_count,
            quality_suppressed_violation_hash
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
                row.get::<_, i64>(8)?,
                row.get::<_, String>(9)?,
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
        quality_suppressed_violation_count,
        quality_suppressed_violation_hash,
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
                quality_suppressed_violation_count,
                quality_suppressed_violation_hash,
                actual_quality_metric_count: actual_state.metric_count,
                actual_quality_metric_hash: actual_state.metric_hash,
                actual_quality_violation_count: actual_state.violation_count,
                actual_quality_violation_hash: actual_state.violation_hash,
                actual_quality_suppressed_violation_count: actual_state.suppressed_violation_count,
                actual_quality_suppressed_violation_hash: actual_state.suppressed_violation_hash,
            },
        );
    }
    Ok(out)
}

fn parse_suppressed_violations_json(
    payload: &str,
) -> Result<Vec<crate::model::SuppressedQualityViolationEntry>> {
    if payload.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(serde_json::from_str(payload)?)
}
