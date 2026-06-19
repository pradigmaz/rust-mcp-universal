use std::collections::HashMap;

use anyhow::Result;
use rusqlite::params;
use rusqlite::params_from_iter;
use rusqlite::types::Value as SqlValue;

use crate::model::{
    QualityCategory, QualityLocation, QualityMetricValue, QualityMode, QualitySeverity,
    QualitySource, QualityViolationEntry, RuleViolationFileHit, RuleViolationsOptions,
    SuppressedQualityViolationEntry,
};

pub(super) fn load_quality_candidates(
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

pub(super) fn attach_and_filter_violations(
    conn: &rusqlite::Connection,
    candidates: Vec<RuleViolationFileHit>,
    options: &RuleViolationsOptions,
    metrics_by_path: &HashMap<String, Vec<QualityMetricValue>>,
    suppressed_by_path: &HashMap<String, Vec<SuppressedQualityViolationEntry>>,
) -> Result<Vec<RuleViolationFileHit>> {
    let path_like = options
        .path_prefix
        .as_ref()
        .map(|prefix| format!("{prefix}%"));
    let mut sql = r#"
        SELECT
            q.path,
            v.rule_id,
            v.actual_value,
            v.threshold_value,
            v.message,
            v.severity,
            v.category,
            v.source,
            v.finding_family,
            v.confidence,
            v.manual_review_required,
            v.noise_reason,
            v.recommended_followups_json,
            v.start_line,
            v.start_column,
            v.end_line,
            v.end_column
        FROM file_quality q
        JOIN file_rule_violations v ON v.path = q.path
        WHERE (?1 IS NULL OR q.path LIKE ?1)
          AND (?2 IS NULL OR q.language = ?2)
        "#
    .to_string();
    let mut query_params = vec![
        optional_text_value(path_like),
        optional_text_value(options.language.clone()),
    ];
    if !options.rule_ids.is_empty() {
        let placeholders = (0..options.rule_ids.len())
            .map(|index| format!("?{}", index + 3))
            .collect::<Vec<_>>()
            .join(", ");
        sql.push_str(" AND v.rule_id IN (");
        sql.push_str(&placeholders);
        sql.push(')');
        query_params.extend(options.rule_ids.iter().cloned().map(SqlValue::Text));
    }
    sql.push_str(" ORDER BY q.path ASC, v.rule_id ASC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(query_params), |row| {
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
                    finding_family: row
                        .get::<_, Option<String>>(8)?
                        .and_then(|value| crate::model::FindingFamily::parse(&value)),
                    confidence: row
                        .get::<_, Option<String>>(9)?
                        .and_then(|value| crate::model::FindingConfidence::parse(&value)),
                    manual_review_required: row.get::<_, i64>(10).unwrap_or_default() != 0,
                    noise_reason: row.get(11)?,
                    recommended_followups: serde_json::from_str(
                        &row.get::<_, String>(12)
                            .unwrap_or_else(|_| "[]".to_string()),
                    )
                    .unwrap_or_default(),
                    signal_key: None,
                    memory_status: None,
                    location: violation_location(
                        row.get::<_, Option<i64>>(13)?,
                        row.get::<_, Option<i64>>(14)?,
                        row.get::<_, Option<i64>>(15)?,
                        row.get::<_, Option<i64>>(16)?,
                    ),
                },
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut violations_by_path = HashMap::<String, Vec<QualityViolationEntry>>::new();
    for (path, violation) in rows {
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

pub(super) fn load_suppressed_violations_by_path(
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

fn optional_text_value(value: Option<String>) -> SqlValue {
    value.map_or(SqlValue::Null, SqlValue::Text)
}
