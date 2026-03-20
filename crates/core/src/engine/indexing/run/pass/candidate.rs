use std::path::Path;

use anyhow::Result;
use walkdir::WalkDir;

use super::super::types::PassResult;
use super::{PassConfig, files, filters, graph, source, stats, walk};
use crate::engine::storage;
use crate::utils::INDEX_FILE_LIMIT;

pub(super) fn run_walking_pass(
    tx: &rusqlite::Transaction<'_>,
    config: &PassConfig<'_>,
    changed_since_unix_ms: Option<i64>,
    pass_result: &mut PassResult,
) -> Result<()> {
    for walk_entry in WalkDir::new(&config.engine.project_root).into_iter() {
        let Some(entry) =
            walk::resolve_walk_entry(walk_entry, &config.engine.project_root, pass_result)
        else {
            continue;
        };
        if !entry.file_type().is_file() {
            continue;
        }
        pass_result.stats.scanned += 1;
        let path = entry.path();
        let Some(rel_text) = filters::resolve_scoped_path(
            path,
            &config.engine.project_root,
            config.scope,
            config.ignore_matcher,
            pass_result,
        ) else {
            continue;
        };
        index_candidate_path(
            tx,
            config,
            path,
            &rel_text,
            changed_since_unix_ms,
            pass_result,
        )?;
    }
    Ok(())
}

pub(super) fn index_candidate_path(
    tx: &rusqlite::Transaction<'_>,
    config: &PassConfig<'_>,
    path: &Path,
    rel_text: &str,
    changed_since_unix_ms: Option<i64>,
    pass_result: &mut PassResult,
) -> Result<()> {
    let metadata = match source::read_source_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) => {
            handle_source_io_failure(tx, config, rel_text, pass_result, &err)?;
            return Ok(());
        }
    };
    if metadata.size_bytes > INDEX_FILE_LIMIT {
        stats::mark_authoritative_removed_path_as_skipped(
            tx,
            config.existing_files,
            rel_text,
            &mut pass_result.stats,
        )?;
        return Ok(());
    }

    if !filters::should_refresh_candidate(
        changed_since_unix_ms,
        config.existing_files.get(rel_text),
        metadata.current_mtime_unix_ms,
    ) {
        stats::mark_skipped_before_changed_since(&mut pass_result.stats);
        return Ok(());
    }

    let source_snapshot = match source::read_source_snapshot(path) {
        Ok(source_snapshot) => source_snapshot,
        Err(err) => {
            handle_source_io_failure(tx, config, rel_text, pass_result, &err)?;
            return Ok(());
        }
    };
    if source_snapshot.is_binary {
        stats::mark_authoritative_removed_path_as_skipped(
            tx,
            config.existing_files,
            rel_text,
            &mut pass_result.stats,
        )?;
        return Ok(());
    }

    if !config.options.reindex
        && filters::is_unchanged(config.existing_files, rel_text, &source_snapshot.sha256)
    {
        storage::update_path_source_mtime(tx, rel_text, metadata.current_mtime_unix_ms)?;
        stats::mark_unchanged(&mut pass_result.stats);
        return Ok(());
    }

    let existed_before = config.existing_files.contains_key(rel_text);
    let indexed_at = source::now_indexed_at()?;
    let graph_artifacts =
        graph::build_graph_artifacts(path, &source_snapshot.language, &source_snapshot.full_text);

    files::persist_indexed_file(
        tx,
        files::PersistFileInput {
            rel_text,
            source: &source_snapshot,
            metadata: &metadata,
            indexed_at: &indexed_at,
            semantic_model: config.semantic_model,
            semantic_dim: config.semantic_dim,
            graph: &graph_artifacts,
        },
        &mut pass_result.stats,
    )?;

    stats::mark_indexed(&mut pass_result.stats, existed_before);
    Ok(())
}

fn handle_source_io_failure(
    tx: &rusqlite::Transaction<'_>,
    config: &PassConfig<'_>,
    rel_text: &str,
    pass_result: &mut PassResult,
    err: &std::io::Error,
) -> Result<()> {
    match stats::classify_io_failure(err) {
        stats::IoFailurePolicy::AuthoritativeRemoval => {
            stats::mark_authoritative_removed_path_as_skipped(
                tx,
                config.existing_files,
                rel_text,
                &mut pass_result.stats,
            )?;
        }
        stats::IoFailurePolicy::PreservePriorSnapshot => {
            pass_result.failed_paths.insert(rel_text.to_string());
            stats::mark_preserved_path_on_failure(&mut pass_result.stats);
        }
    }
    Ok(())
}
