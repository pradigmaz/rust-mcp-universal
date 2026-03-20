use anyhow::Result;
use rusqlite::params;

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
