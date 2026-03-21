use anyhow::Result;
use rusqlite::params;

use crate::model::{QualityMode, QualityViolationEntry};

pub(in crate::engine) fn clear_index_tables(tx: &rusqlite::Transaction<'_>) -> Result<()> {
    tx.execute_batch(
        r#"
        DELETE FROM files_fts;
        DELETE FROM files;
        DELETE FROM symbols;
        DELETE FROM module_deps;
        DELETE FROM refs;
        DELETE FROM file_graph_edges;
        DELETE FROM semantic_vectors;
        DELETE FROM semantic_ann_buckets;
        DELETE FROM file_chunks;
        DELETE FROM chunk_embeddings;
        DELETE FROM file_rule_violations;
        DELETE FROM file_quality;
        DELETE FROM model_metadata;
        DELETE FROM meta;
        "#,
    )?;
    Ok(())
}

pub(in crate::engine) fn upsert_meta(
    tx: &rusqlite::Transaction<'_>,
    key: &str,
    value: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub(in crate::engine) fn remove_path_index(
    tx: &rusqlite::Transaction<'_>,
    path: &str,
) -> Result<()> {
    tx.execute("DELETE FROM files_fts WHERE path = ?1", [path])?;
    tx.execute("DELETE FROM symbols WHERE path = ?1", [path])?;
    tx.execute("DELETE FROM module_deps WHERE path = ?1", [path])?;
    tx.execute("DELETE FROM refs WHERE path = ?1", [path])?;
    tx.execute(
        "DELETE FROM file_graph_edges WHERE src_path = ?1 OR dst_path = ?1",
        [path],
    )?;
    tx.execute("DELETE FROM file_chunks WHERE path = ?1", [path])?;
    tx.execute("DELETE FROM semantic_vectors WHERE path = ?1", [path])?;
    tx.execute("DELETE FROM semantic_ann_buckets WHERE path = ?1", [path])?;
    tx.execute("DELETE FROM files WHERE path = ?1", [path])?;
    Ok(())
}

pub(in crate::engine) fn remove_path_quality(
    tx: &rusqlite::Transaction<'_>,
    path: &str,
) -> Result<()> {
    tx.execute("DELETE FROM file_rule_violations WHERE path = ?1", [path])?;
    tx.execute("DELETE FROM file_quality WHERE path = ?1", [path])?;
    Ok(())
}

pub(in crate::engine) fn update_path_source_mtime(
    tx: &rusqlite::Transaction<'_>,
    path: &str,
    source_mtime_unix_ms: Option<i64>,
) -> Result<()> {
    let Some(source_mtime_unix_ms) = source_mtime_unix_ms else {
        return Ok(());
    };
    tx.execute(
        "UPDATE files SET source_mtime_unix_ms = ?2 WHERE path = ?1",
        params![path, source_mtime_unix_ms],
    )?;
    Ok(())
}

pub(in crate::engine) fn update_path_quality_mtime(
    tx: &rusqlite::Transaction<'_>,
    path: &str,
    source_mtime_unix_ms: Option<i64>,
) -> Result<()> {
    let Some(source_mtime_unix_ms) = source_mtime_unix_ms else {
        return Ok(());
    };
    tx.execute(
        "UPDATE file_quality SET source_mtime_unix_ms = ?2 WHERE path = ?1",
        params![path, source_mtime_unix_ms],
    )?;
    Ok(())
}

pub(in crate::engine) struct UpsertQualitySnapshotInput<'a> {
    pub(in crate::engine) path: &'a str,
    pub(in crate::engine) language: &'a str,
    pub(in crate::engine) size_bytes: i64,
    pub(in crate::engine) total_lines: Option<i64>,
    pub(in crate::engine) non_empty_lines: Option<i64>,
    pub(in crate::engine) import_count: Option<i64>,
    pub(in crate::engine) quality_mode: QualityMode,
    pub(in crate::engine) source_mtime_unix_ms: Option<i64>,
    pub(in crate::engine) quality_ruleset_version: i64,
    pub(in crate::engine) quality_violation_hash: &'a str,
    pub(in crate::engine) quality_indexed_at_utc: &'a str,
    pub(in crate::engine) violations: &'a [QualityViolationEntry],
}

pub(in crate::engine) fn upsert_quality_snapshot(
    tx: &rusqlite::Transaction<'_>,
    input: UpsertQualitySnapshotInput<'_>,
) -> Result<()> {
    remove_path_quality(tx, input.path)?;

    for violation in input.violations {
        tx.execute(
            "INSERT INTO file_rule_violations(path, rule_id, actual_value, threshold_value, message)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                input.path,
                &violation.rule_id,
                violation.actual_value,
                violation.threshold_value,
                &violation.message
            ],
        )?;
    }

    tx.execute(
        "INSERT INTO file_quality(
                path,
                language,
                size_bytes,
                total_lines,
                non_empty_lines,
                import_count,
                quality_mode,
                source_mtime_unix_ms,
                quality_ruleset_version,
                quality_violation_count,
                quality_violation_hash,
                quality_indexed_at_utc
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            input.path,
            input.language,
            input.size_bytes,
            input.total_lines,
            input.non_empty_lines,
            input.import_count,
            input.quality_mode.as_str(),
            input.source_mtime_unix_ms,
            input.quality_ruleset_version,
            i64::try_from(input.violations.len()).unwrap_or(i64::MAX),
            input.quality_violation_hash,
            input.quality_indexed_at_utc
        ],
    )?;
    Ok(())
}
