use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use crate::engine::Engine;
use crate::model::IndexStatus;
use crate::vector_rank::semantic_model_name;

impl Engine {
    pub fn index_status(&self) -> Result<IndexStatus> {
        if !self.db_path.exists() {
            return Ok(zero_index_status(self));
        }
        let mut conn = self.open_db_read_only()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)?;
        let files = count_rows(&tx, "files")?;
        let symbols = count_rows(&tx, "symbols")?;
        let module_deps = count_rows(&tx, "module_deps")?;
        let refs = count_rows(&tx, "refs")?;
        let semantic_vectors = count_rows(&tx, "semantic_vectors")?;
        let file_chunks = count_rows(&tx, "file_chunks")?;
        let chunk_embeddings = count_rows(&tx, "chunk_embeddings")?;

        let status = IndexStatus {
            project_root: self.project_root.display().to_string(),
            db_path: self.db_path.display().to_string(),
            files,
            symbols,
            module_deps,
            refs,
            semantic_vectors,
            file_chunks,
            chunk_embeddings,
            semantic_model: semantic_model_name(),
            last_index_lock_wait_ms: meta_u64(&tx, "last_index_lock_wait_ms")?,
            last_embedding_cache_hits: meta_u64(&tx, "last_embedding_cache_hits")? as usize,
            last_embedding_cache_misses: meta_u64(&tx, "last_embedding_cache_misses")? as usize,
        };
        tx.commit()?;
        Ok(status)
    }
}

fn zero_index_status(engine: &Engine) -> IndexStatus {
    IndexStatus {
        project_root: engine.project_root.display().to_string(),
        db_path: engine.db_path.display().to_string(),
        files: 0,
        symbols: 0,
        module_deps: 0,
        refs: 0,
        semantic_vectors: 0,
        file_chunks: 0,
        chunk_embeddings: 0,
        semantic_model: semantic_model_name(),
        last_index_lock_wait_ms: 0,
        last_embedding_cache_hits: 0,
        last_embedding_cache_misses: 0,
    }
}

fn count_rows(conn: &Connection, table: &str) -> Result<usize> {
    let sql = format!("SELECT COUNT(1) FROM {table}");
    let count: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
    Ok(usize::try_from(count).unwrap_or(usize::MAX))
}

fn meta_u64(conn: &Connection, key: &str) -> Result<u64> {
    let value = conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get::<_, String>(0)
        })
        .optional()
        .with_context(|| format!("failed to read meta key `{key}`"))?;

    match value {
        Some(raw) => raw
            .parse::<u64>()
            .with_context(|| format!("meta key `{key}` contains non-u64 value `{raw}`")),
        None => Ok(0),
    }
}
