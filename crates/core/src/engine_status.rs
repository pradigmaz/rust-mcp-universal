mod freshness;
mod store;

use anyhow::Result;

use crate::engine::Engine;
use crate::model::{IndexFreshnessStatus, IndexStatus};
use crate::vector_rank::semantic_model_name;

impl Engine {
    pub fn index_status(&self) -> Result<IndexStatus> {
        if !self.db_path.exists() {
            return Ok(zero_index_status(self));
        }
        let mut conn = self.open_db_read_only()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)?;
        let files = store::count_rows(&tx, "files")?;
        let symbols = store::count_rows(&tx, "symbols")?;
        let module_deps = store::count_rows(&tx, "module_deps")?;
        let refs = store::count_rows(&tx, "refs")?;
        let semantic_vectors = store::count_rows(&tx, "semantic_vectors")?;
        let file_chunks = store::count_rows(&tx, "file_chunks")?;
        let chunk_embeddings = store::count_rows(&tx, "chunk_embeddings")?;

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
            last_index_lock_wait_ms: store::meta_u64(&tx, "last_index_lock_wait_ms")?,
            last_embedding_cache_hits: store::meta_u64(&tx, "last_embedding_cache_hits")? as usize,
            last_embedding_cache_misses: store::meta_u64(&tx, "last_embedding_cache_misses")?
                as usize,
            freshness: freshness::scan(&tx, &self.project_root)?,
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
        freshness: IndexFreshnessStatus::default(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use rusqlite::params;

    use super::Engine;

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    fn write_file(root: &Path, relative: &str, content: &str) -> anyhow::Result<()> {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    #[test]
    fn index_status_reports_stale_and_missing_files() -> anyhow::Result<()> {
        let root = temp_dir("rmu-index-freshness");
        fs::create_dir_all(&root)?;
        write_file(&root, "src/stale.rs", "pub fn stale_symbol() {}\n")?;
        write_file(&root, "src/missing.rs", "pub fn missing_symbol() {}\n")?;

        let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
        engine.index_path()?;
        fs::remove_file(root.join("src/missing.rs"))?;

        let conn = engine.open_db()?;
        conn.execute(
            "UPDATE files SET source_mtime_unix_ms = 0 WHERE path = ?1",
            params!["src/stale.rs"],
        )?;
        drop(conn);

        let status = engine.index_status()?;
        assert_eq!(status.freshness.checked_files, 2);
        assert_eq!(status.freshness.stale_files, 1);
        assert_eq!(status.freshness.missing_files, 1);
        assert_eq!(
            status.freshness.hint.as_deref(),
            Some("refresh the index before relying on retrieval results")
        );
        assert!(
            status
                .freshness
                .sample_paths
                .contains(&"src/stale.rs".to_string())
        );
        assert!(
            status
                .freshness
                .sample_paths
                .contains(&"src/missing.rs".to_string())
        );

        let _ = fs::remove_dir_all(root);
        Ok(())
    }
}
