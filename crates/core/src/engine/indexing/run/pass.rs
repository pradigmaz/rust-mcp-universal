use std::collections::HashMap;

use anyhow::Result;

use super::types::{PassResult, RunSelector};
use crate::engine::{Engine, storage};
use crate::index_scope::IndexScope;
use crate::utils::ProjectIgnoreMatcher;

#[path = "pass/candidate.rs"]
mod candidate;
#[path = "pass/chunks.rs"]
mod chunks;
#[path = "pass/files.rs"]
mod files;
#[path = "pass/filters.rs"]
pub(crate) mod filters;
#[path = "pass/graph.rs"]
mod graph;
#[path = "pass/quality.rs"]
mod quality;
#[path = "pass/source.rs"]
mod source;
#[path = "pass/stats.rs"]
mod stats;
#[path = "pass/vectors.rs"]
mod vectors;
#[path = "pass/walk.rs"]
mod walk;

pub(super) struct PassConfig<'a> {
    pub(super) engine: &'a Engine,
    pub(super) options: &'a crate::model::IndexingOptions,
    pub(super) selector: &'a RunSelector,
    pub(super) scope: &'a IndexScope,
    pub(super) ignore_matcher: &'a ProjectIgnoreMatcher,
    pub(super) existing_files: &'a HashMap<String, storage::ExistingFileState>,
    pub(super) existing_quality: &'a HashMap<String, storage::ExistingQualityState>,
    pub(super) semantic_model: &'a str,
    pub(super) semantic_dim: i64,
}

pub(super) fn run_indexing_pass(
    tx: &rusqlite::Transaction<'_>,
    config: &PassConfig<'_>,
) -> Result<PassResult> {
    let mut pass_result = PassResult::default();
    match config.selector {
        RunSelector::Full => candidate::run_walking_pass(tx, config, None, &mut pass_result)?,
        RunSelector::Timestamp {
            changed_since_unix_ms,
        } => {
            candidate::run_walking_pass(tx, config, Some(*changed_since_unix_ms), &mut pass_result)?
        }
        RunSelector::Commit(commit_selector) => {
            pass_result.authoritative_deleted_paths = commit_selector.deleted_paths.clone();
            let mut candidate_paths = commit_selector
                .candidate_paths
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            candidate_paths.sort();
            for rel_text in candidate_paths {
                pass_result.stats.scanned += 1;
                let path = config.engine.project_root.join(&rel_text);
                candidate::index_candidate_path(
                    tx,
                    config,
                    &path,
                    &rel_text,
                    None,
                    &mut pass_result,
                )?;
            }
        }
    }

    Ok(pass_result)
}
