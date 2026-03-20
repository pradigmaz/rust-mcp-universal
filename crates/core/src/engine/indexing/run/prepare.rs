use std::collections::HashMap;

use anyhow::Result;
use rusqlite::params;

use super::selector;
use super::types::PreparedIndexRun;
use crate::engine::Engine;
use crate::engine::storage;
use crate::index_scope::IndexScope;
use crate::model::IndexingOptions;
use crate::utils::ProjectIgnoreMatcher;

pub(super) fn prepare_index_run(
    tx: &rusqlite::Transaction<'_>,
    engine: &Engine,
    options: &IndexingOptions,
    scope: &IndexScope,
    semantic_model: &str,
    semantic_dim: i64,
    metadata_ts: &str,
) -> Result<PreparedIndexRun> {
    let full_reindex = options.reindex
        && options.changed_since.is_none()
        && options.changed_since_commit.is_none();
    if full_reindex {
        storage::clear_index_tables(tx)?;
    }
    tx.execute(
        "INSERT INTO model_metadata(model, dim, updated_at_utc) VALUES (?1, ?2, ?3)
             ON CONFLICT(model) DO UPDATE SET dim = excluded.dim, updated_at_utc = excluded.updated_at_utc",
        params![semantic_model, semantic_dim, metadata_ts],
    )?;

    let existing_files = if full_reindex {
        HashMap::new()
    } else {
        storage::load_existing_file_state(tx, semantic_model)?
    };
    let ignore_matcher = ProjectIgnoreMatcher::new(&engine.project_root)?;
    let selector =
        selector::resolve_run_selector(engine, options, scope, &existing_files, &ignore_matcher)?;
    Ok(PreparedIndexRun {
        existing_files,
        selector,
        ignore_matcher,
    })
}
