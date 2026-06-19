use std::collections::HashMap;

use anyhow::{Result, anyhow};
use rusqlite::Statement;

use super::quality_state::ExistingQualityState;
use super::quality_state_actual::load_actual_quality_state;

pub(crate) trait QualityStateSource {
    fn prepare_quality_state<'a>(&'a self, sql: &str) -> rusqlite::Result<Statement<'a>>;
}

impl QualityStateSource for rusqlite::Connection {
    fn prepare_quality_state<'a>(&'a self, sql: &str) -> rusqlite::Result<Statement<'a>> {
        self.prepare(sql)
    }
}

impl QualityStateSource for rusqlite::Transaction<'_> {
    fn prepare_quality_state<'a>(&'a self, sql: &str) -> rusqlite::Result<Statement<'a>> {
        self.prepare(sql)
    }
}

pub(crate) fn load_existing_quality_state<S>(
    source: &S,
) -> Result<HashMap<String, ExistingQualityState>>
where
    S: QualityStateSource + ?Sized,
{
    let actual_quality_state = load_actual_quality_state(source)?;
    let mut stmt = source.prepare_quality_state(
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
