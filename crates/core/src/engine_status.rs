use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use crate::engine::Engine;
use crate::model::{IndexFreshnessStatus, IndexStatus};
use crate::vector_rank::semantic_model_name;

const FRESHNESS_SAMPLE_LIMIT: usize = 5;

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
            freshness: scan_freshness(&tx, &self.project_root)?,
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

fn scan_freshness(conn: &Connection, project_root: &Path) -> Result<IndexFreshnessStatus> {
    let mut status = IndexFreshnessStatus::default();
    let mut stmt = conn.prepare("SELECT path, source_mtime_unix_ms FROM files ORDER BY path")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
    })?;

    for row in rows {
        let (path, indexed_mtime) = row?;
        status.checked_files += 1;
        let absolute_path = project_root.join(&path);
        let Ok(metadata) = fs::metadata(&absolute_path) else {
            status.missing_files += 1;
            push_sample(&mut status.sample_paths, path);
            continue;
        };
        let Some(indexed_mtime) = indexed_mtime else {
            continue;
        };
        let current_mtime = metadata.modified().ok().map(system_time_to_unix_ms);
        if current_mtime.is_some_and(|current| current > indexed_mtime) {
            status.stale_files += 1;
            push_sample(&mut status.sample_paths, path);
        }
    }

    if status.stale_files > 0 || status.missing_files > 0 {
        status.hint = Some("refresh the index before relying on retrieval results".to_string());
    }
    Ok(status)
}

fn push_sample(sample_paths: &mut Vec<String>, path: String) {
    if sample_paths.len() < FRESHNESS_SAMPLE_LIMIT {
        sample_paths.push(path);
    }
}

fn system_time_to_unix_ms(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
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
