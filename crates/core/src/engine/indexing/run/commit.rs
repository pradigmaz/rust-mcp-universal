use std::collections::HashMap;

use anyhow::Result;

use super::super::post::{
    FinalizeMetrics, PruneDeletedPathsInput, finalize_index_metadata, prune_deleted_paths,
    prune_deleted_quality_paths,
};
use super::types::{PassResult, RunSelector};
use crate::engine::{Engine, IndexSummary, storage};
use crate::index_scope::IndexScope;
use crate::model::IndexingOptions;
use crate::rebuild_lock::RebuildLockGuard;

pub(super) struct FinalizeCommitInput<'a> {
    pub(super) engine: &'a Engine,
    pub(super) options: &'a IndexingOptions,
    pub(super) scope: &'a IndexScope,
    pub(super) existing_files: &'a HashMap<String, storage::ExistingFileState>,
    pub(super) existing_quality: &'a HashMap<String, storage::ExistingQualityState>,
    pub(super) selector: &'a RunSelector,
    pub(super) semantic_model: &'a str,
    pub(super) semantic_dim: i64,
    pub(super) lock_wait_ms: u64,
    pub(super) rebuild_lock: RebuildLockGuard,
    pub(super) pass_result: PassResult,
}

pub(super) fn finalize_and_commit(
    tx: rusqlite::Transaction<'_>,
    input: FinalizeCommitInput<'_>,
) -> Result<IndexSummary> {
    let FinalizeCommitInput {
        engine,
        options,
        scope,
        existing_files,
        existing_quality,
        selector,
        semantic_model,
        semantic_dim,
        lock_wait_ms,
        rebuild_lock,
        mut pass_result,
    } = input;

    pass_result.stats.deleted += prune_deleted_paths(PruneDeletedPathsInput {
        tx: &tx,
        existing_files,
        options,
        scope,
        selector,
        present_paths: &pass_result.present_paths,
        failed_paths: &pass_result.failed_paths,
        authoritative_deleted_paths: &pass_result.authoritative_deleted_paths,
        failed_walk_prefixes: &pass_result.failed_walk_prefixes,
    })?;
    prune_deleted_quality_paths(
        &tx,
        existing_quality,
        options,
        scope,
        selector,
        &pass_result.present_paths,
        &pass_result.failed_paths,
        &pass_result.authoritative_deleted_paths,
        &pass_result.failed_walk_prefixes,
    )?;
    storage::rebuild_file_graph_edges(&tx)?;

    let finalize_metrics = FinalizeMetrics {
        lock_wait_ms,
        embedding_cache_hits: pass_result.stats.embedding_cache_hits,
        embedding_cache_misses: pass_result.stats.embedding_cache_misses,
        added: pass_result.stats.added,
        changed: pass_result.stats.changed,
        unchanged: pass_result.stats.unchanged,
        deleted: pass_result.stats.deleted,
        project_root: &engine.project_root,
        semantic_model,
        semantic_dim: usize::try_from(semantic_dim)
            .ok()
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(u32::MAX),
    };
    finalize_index_metadata(&tx, &finalize_metrics)?;

    tx.commit()?;
    drop(rebuild_lock);
    Ok(IndexSummary {
        scanned: pass_result.stats.scanned,
        indexed: pass_result.stats.indexed,
        skipped_binary_or_large: pass_result.stats.skipped,
        skipped_before_changed_since: pass_result.stats.skipped_before_changed_since,
        added: pass_result.stats.added,
        changed: pass_result.stats.changed,
        unchanged: pass_result.stats.unchanged,
        deleted: pass_result.stats.deleted,
        changed_since: options.changed_since,
        changed_since_commit: options.changed_since_commit.clone(),
        resolved_merge_base_commit: match selector {
            RunSelector::Commit(commit_selector) => {
                Some(commit_selector.resolved_merge_base_commit.clone())
            }
            RunSelector::Full | RunSelector::Timestamp { .. } => None,
        },
        lock_wait_ms,
        embedding_cache_hits: pass_result.stats.embedding_cache_hits,
        embedding_cache_misses: pass_result.stats.embedding_cache_misses,
    })
}
