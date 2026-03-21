use std::path::PathBuf;

use crate::model::MigrationMode;
use crate::model::{DbPruneResult, DeleteIndexResult};
use anyhow::Result;
use rusqlite::Connection;
use time::OffsetDateTime;

#[path = "engine/benchmark.rs"]
mod benchmark;
#[path = "engine/chunks.rs"]
mod chunks;
#[path = "engine/compatibility.rs"]
pub(crate) mod compatibility;
#[path = "engine/context.rs"]
mod context;
#[path = "engine/indexing.rs"]
mod indexing;
#[path = "engine/lifecycle.rs"]
mod lifecycle;
#[path = "engine/maintenance.rs"]
mod maintenance;
#[path = "engine/navigation.rs"]
mod navigation;
#[path = "engine/preview.rs"]
mod preview;
#[path = "engine/query.rs"]
mod query;
#[path = "engine/schema.rs"]
mod schema;
#[path = "engine/storage.rs"]
mod storage;

#[derive(Debug, Clone)]
pub struct Engine {
    pub project_root: PathBuf,
    pub db_path: PathBuf,
    pub migration_mode: MigrationMode,
}

#[derive(Debug, Clone)]
pub struct IndexSummary {
    pub scanned: usize,
    pub indexed: usize,
    pub skipped_binary_or_large: usize,
    pub skipped_before_changed_since: usize,
    pub added: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub deleted: usize,
    pub changed_since: Option<OffsetDateTime>,
    pub changed_since_commit: Option<String>,
    pub resolved_merge_base_commit: Option<String>,
    pub lock_wait_ms: u64,
    pub embedding_cache_hits: usize,
    pub embedding_cache_misses: usize,
}

impl Engine {
    pub fn new(project_root: impl Into<PathBuf>, db_path: Option<PathBuf>) -> Result<Self> {
        Self::new_with_migration_mode(project_root, db_path, MigrationMode::Auto)
    }

    pub fn new_read_only(
        project_root: impl Into<PathBuf>,
        db_path: Option<PathBuf>,
    ) -> Result<Self> {
        Self::new_read_only_with_migration_mode(project_root, db_path, MigrationMode::Auto)
    }

    pub fn new_with_migration_mode(
        project_root: impl Into<PathBuf>,
        db_path: Option<PathBuf>,
        migration_mode: MigrationMode,
    ) -> Result<Self> {
        let (project_root, db_path, uses_default_store) =
            lifecycle::resolve_paths(project_root.into(), db_path)?;

        let engine = Self {
            project_root,
            db_path,
            migration_mode,
        };
        engine.init_db()?;
        lifecycle::touch_access(&engine)?;
        if uses_default_store {
            let _ = engine.cleanup_stale_indexes();
        }
        Ok(engine)
    }

    pub fn new_read_only_with_migration_mode(
        project_root: impl Into<PathBuf>,
        db_path: Option<PathBuf>,
        migration_mode: MigrationMode,
    ) -> Result<Self> {
        let (project_root, db_path) =
            lifecycle::resolve_paths_read_only(project_root.into(), db_path)?;

        Ok(Self {
            project_root,
            db_path,
            migration_mode,
        })
    }

    pub fn init_db(&self) -> Result<()> {
        lifecycle::init_db(self)
    }

    pub(crate) fn open_db(&self) -> Result<Connection> {
        lifecycle::open_db(self)
    }

    pub(crate) fn open_db_read_only(&self) -> Result<Connection> {
        lifecycle::open_db_read_only(self)
    }

    pub fn cleanup_stale_indexes(&self) -> Result<DbPruneResult> {
        lifecycle::cleanup_stale_indexes(self)
    }

    pub fn delete_index_storage(&self) -> Result<DeleteIndexResult> {
        lifecycle::delete_index_storage(self)
    }
}

#[cfg(test)]
#[path = "engine/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "engine/tests_quality.rs"]
mod quality_tests;
