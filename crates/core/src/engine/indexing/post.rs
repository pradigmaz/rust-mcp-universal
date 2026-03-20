use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use rusqlite::Transaction;
use time::OffsetDateTime;

use super::super::{compatibility, storage};
use super::run::types::RunSelector;
use super::util::path_under_walk_error;
use crate::index_scope::IndexScope;
use crate::model::IndexingOptions;

pub(super) struct FinalizeMetrics<'a> {
    pub lock_wait_ms: u64,
    pub embedding_cache_hits: usize,
    pub embedding_cache_misses: usize,
    pub added: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub deleted: usize,
    pub project_root: &'a Path,
    pub semantic_model: &'a str,
    pub semantic_dim: u32,
}

pub(super) struct PruneDeletedPathsInput<'a, 'conn> {
    pub tx: &'a Transaction<'conn>,
    pub existing_files: &'a HashMap<String, storage::ExistingFileState>,
    pub options: &'a IndexingOptions,
    pub scope: &'a IndexScope,
    pub selector: &'a RunSelector,
    pub present_paths: &'a HashSet<String>,
    pub failed_paths: &'a HashSet<String>,
    pub authoritative_deleted_paths: &'a HashSet<String>,
    pub failed_walk_prefixes: &'a [String],
}

pub(super) fn finalize_index_metadata(
    tx: &Transaction<'_>,
    metrics: &FinalizeMetrics<'_>,
) -> Result<()> {
    tx.execute(
        r#"
        DELETE FROM chunk_embeddings
        WHERE NOT EXISTS (
            SELECT 1 FROM file_chunks
            WHERE file_chunks.chunk_hash = chunk_embeddings.chunk_hash
        );
        "#,
        [],
    )?;
    storage::upsert_meta(
        tx,
        "last_index_lock_wait_ms",
        &metrics.lock_wait_ms.to_string(),
    )?;
    storage::upsert_meta(
        tx,
        "last_embedding_cache_hits",
        &metrics.embedding_cache_hits.to_string(),
    )?;
    storage::upsert_meta(
        tx,
        "last_embedding_cache_misses",
        &metrics.embedding_cache_misses.to_string(),
    )?;
    storage::upsert_meta(tx, "last_index_added", &metrics.added.to_string())?;
    storage::upsert_meta(tx, "last_index_changed", &metrics.changed.to_string())?;
    storage::upsert_meta(tx, "last_index_unchanged", &metrics.unchanged.to_string())?;
    storage::upsert_meta(tx, "last_index_deleted", &metrics.deleted.to_string())?;
    storage::upsert_meta(
        tx,
        "project_root",
        &metrics.project_root.display().to_string(),
    )?;
    compatibility::write_index_identity_meta(tx, metrics.semantic_model, metrics.semantic_dim)?;

    let access_at =
        OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?;
    storage::upsert_meta(tx, "last_access_utc", &access_at)?;
    Ok(())
}

pub(super) fn prune_deleted_paths(input: PruneDeletedPathsInput<'_, '_>) -> Result<usize> {
    let mut deleted = 0;
    for path in input.existing_files.keys() {
        if !input.options.reindex && input.scope.has_rules() && !input.scope.allows(path) {
            storage::remove_path_index(input.tx, path)?;
            deleted += 1;
            continue;
        }
        if matches!(input.selector, RunSelector::Commit(_)) {
            if input.authoritative_deleted_paths.contains(path) {
                storage::remove_path_index(input.tx, path)?;
                deleted += 1;
            }
            continue;
        }
        if input.present_paths.contains(path) {
            continue;
        }
        if input.failed_paths.contains(path)
            || path_under_walk_error(path, input.failed_walk_prefixes)
        {
            continue;
        }
        storage::remove_path_index(input.tx, path)?;
        deleted += 1;
    }
    Ok(deleted)
}
