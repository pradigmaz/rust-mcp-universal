use anyhow::Result;
use time::OffsetDateTime;

use super::super::{Engine, IndexSummary};
use crate::index_scope::IndexScope;
use crate::model::IndexingOptions;
use crate::rebuild_lock::RebuildLockGuard;
use crate::vector_rank::{semantic_model_name, vector_dim};

#[path = "run/commit.rs"]
mod commit;
#[path = "run/pass.rs"]
mod pass;
#[path = "run/prepare.rs"]
mod prepare;
#[path = "run/selector.rs"]
pub(crate) mod selector;
#[path = "run/types.rs"]
pub(crate) mod types;

pub(crate) use pass::filters;

pub(super) fn index_path_with_options_impl(
    engine: &Engine,
    options: &IndexingOptions,
) -> Result<IndexSummary> {
    let scope = IndexScope::new(options)?;
    let rebuild_lock = RebuildLockGuard::acquire(&engine.db_path)?;
    let lock_wait_ms = rebuild_lock.wait_ms();

    let mut conn = engine.open_db()?;
    let tx = conn.transaction()?;
    let semantic_model = semantic_model_name();
    let semantic_dim = i64::try_from(vector_dim()).unwrap_or(i64::MAX);
    let metadata_ts =
        OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?;

    let prepared = prepare::prepare_index_run(
        &tx,
        engine,
        options,
        &scope,
        &semantic_model,
        semantic_dim,
        &metadata_ts,
    )?;

    let pass_result = pass::run_indexing_pass(
        &tx,
        &pass::PassConfig {
            engine,
            options,
            selector: &prepared.selector,
            scope: &scope,
            ignore_matcher: &prepared.ignore_matcher,
            existing_files: &prepared.existing_files,
            semantic_model: &semantic_model,
            semantic_dim,
        },
    )?;

    commit::finalize_and_commit(
        tx,
        commit::FinalizeCommitInput {
            engine,
            options,
            scope: &scope,
            existing_files: &prepared.existing_files,
            selector: &prepared.selector,
            semantic_model: &semantic_model,
            semantic_dim,
            lock_wait_ms,
            rebuild_lock,
            pass_result,
        },
    )
}
