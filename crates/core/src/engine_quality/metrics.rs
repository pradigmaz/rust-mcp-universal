use std::collections::HashMap;

use anyhow::Result;
use rusqlite::params;

use crate::model::{
    QualityMetricValue, QualitySource, RuleViolationsOptions, WorkspaceQualityTopMetric,
};

pub(super) fn load_top_metrics(
    conn: &rusqlite::Connection,
) -> Result<Vec<WorkspaceQualityTopMetric>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT metric_id, COUNT(DISTINCT path) AS file_count, MAX(metric_value) AS max_value
        FROM file_quality_metrics
        GROUP BY metric_id
        ORDER BY file_count DESC, max_value DESC, metric_id ASC
        LIMIT 5
        "#,
    )?;
    Ok(stmt
        .query_map([], |row| {
            Ok(WorkspaceQualityTopMetric {
                metric_id: row.get(0)?,
                files: usize::try_from(row.get::<_, i64>(1)?).unwrap_or(usize::MAX),
                max_value: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

pub(super) fn load_metrics_by_path(
    conn: &rusqlite::Connection,
    options: &RuleViolationsOptions,
) -> Result<HashMap<String, Vec<QualityMetricValue>>> {
    let path_like = options
        .path_prefix
        .as_ref()
        .map(|prefix| format!("{prefix}%"));
    let metric_filter = options
        .metric_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut stmt = conn.prepare(
        r#"
        SELECT
            q.path,
            m.metric_id,
            m.metric_value,
            m.source,
            m.start_line,
            m.start_column,
            m.end_line,
            m.end_column
        FROM file_quality q
        JOIN file_quality_metrics m ON m.path = q.path
        WHERE (?1 IS NULL OR q.path LIKE ?1)
          AND (?2 IS NULL OR q.language = ?2)
        ORDER BY q.path ASC, m.metric_id ASC
        "#,
    )?;
    let rows = stmt
        .query_map(params![path_like, options.language.as_ref()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                QualityMetricValue {
                    metric_id: row.get(1)?,
                    metric_value: row.get(2)?,
                    source: row
                        .get::<_, Option<String>>(3)?
                        .and_then(|value| QualitySource::parse(&value)),
                    location: build_location(
                        row.get::<_, Option<i64>>(4)?,
                        row.get::<_, Option<i64>>(5)?,
                        row.get::<_, Option<i64>>(6)?,
                        row.get::<_, Option<i64>>(7)?,
                    ),
                },
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut metrics_by_path = HashMap::<String, Vec<QualityMetricValue>>::new();
    for (path, metric) in rows {
        if !metric_filter.is_empty() && !metric_filter.contains(&metric.metric_id.as_str()) {
            continue;
        }
        metrics_by_path.entry(path).or_default().push(metric);
    }
    Ok(metrics_by_path)
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
