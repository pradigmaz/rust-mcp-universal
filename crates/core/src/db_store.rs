use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::Connection;

use crate::model::DbPruneResult;

mod cleanup;
mod config;
mod metadata;

#[derive(Debug, Clone)]
pub(crate) struct DbStoreConfig {
    pub root_dir: PathBuf,
    pub ttl_days: i64,
    pub shared_store: bool,
}

pub(crate) fn default_db_path_for_project(project_root: &Path) -> Result<PathBuf> {
    config::default_db_path_for_project_impl(project_root)
}

pub(crate) fn store_config(project_root: &Path) -> DbStoreConfig {
    config::store_config_impl(project_root)
}

pub(crate) fn touch_database_metadata(conn: &Connection, project_root: &Path) -> Result<()> {
    metadata::touch_database_metadata_impl(conn, project_root)
}

pub(crate) fn cleanup_stale_databases(
    config: &DbStoreConfig,
    active_db_path: &Path,
) -> Result<DbPruneResult> {
    cleanup::cleanup_stale_databases_impl(config, active_db_path)
}

pub(crate) fn sqlite_sidecar_paths(db_path: &Path) -> [PathBuf; 2] {
    metadata::sqlite_sidecar_paths_impl(db_path)
}

fn project_key(project_root: &Path) -> Result<String> {
    config::project_key_impl(project_root)
}

#[cfg(test)]
mod tests;
