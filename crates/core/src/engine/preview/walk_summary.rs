use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::Result;
use walkdir::WalkDir;

use crate::engine::Engine;
use crate::engine::indexing::run::filters;
use crate::engine::indexing::run::types::RunSelector;
use crate::engine::storage;
use crate::index_scope::IndexScope;
use crate::utils::{INDEX_FILE_LIMIT, ProjectIgnoreMatcher, is_probably_ignored, normalize_path};

#[derive(Default)]
pub(super) struct WalkSummary {
    pub(super) scanned_files: usize,
    pub(super) candidate_paths: Vec<String>,
    pub(super) excluded_by_scope_paths: Vec<String>,
    pub(super) ignored_paths: Vec<String>,
    pub(super) skipped_before_changed_since_paths: Vec<String>,
    pub(super) repair_backfill_paths: Vec<String>,
    pub(super) present_paths: HashSet<String>,
    pub(super) failed_paths: HashSet<String>,
    pub(super) failed_walk_prefixes: Vec<String>,
}

pub(super) fn collect_walk_summary(
    engine: &Engine,
    scope: &IndexScope,
    selector: &RunSelector,
    existing_files: &HashMap<String, storage::ExistingFileState>,
    ignore_matcher: &ProjectIgnoreMatcher,
) -> Result<WalkSummary> {
    let mut summary = WalkSummary::default();

    if let RunSelector::Commit(commit_selector) = selector {
        let mut candidate_paths = commit_selector
            .candidate_paths
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        candidate_paths.sort();
        summary.candidate_paths = candidate_paths;
        summary.repair_backfill_paths = commit_selector
            .candidate_paths
            .iter()
            .filter(|path| {
                existing_files
                    .get(*path)
                    .is_some_and(|state| !filters::is_state_complete(state))
            })
            .cloned()
            .collect();
    }

    let changed_since_unix_ms = match selector {
        RunSelector::Timestamp {
            changed_since_unix_ms,
        } => Some(*changed_since_unix_ms),
        RunSelector::Full | RunSelector::Commit(_) => None,
    };

    for walk_entry in WalkDir::new(&engine.project_root).into_iter() {
        let entry = match walk_entry {
            Ok(entry) => entry,
            Err(err) => {
                record_walk_error(
                    engine.project_root.as_path(),
                    err,
                    &mut summary.failed_walk_prefixes,
                );
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        summary.scanned_files += 1;
        let path = entry.path();
        let Some(relative) = path.strip_prefix(&engine.project_root).ok() else {
            continue;
        };

        if is_probably_ignored(relative)
            || ignore_matcher.is_ignored(relative, entry.file_type().is_dir())
        {
            summary.ignored_paths.push(normalize_path(relative));
            continue;
        }

        let rel_text = normalize_path(relative);
        if !scope.allows(&rel_text) {
            summary.excluded_by_scope_paths.push(rel_text);
            continue;
        }

        summary.present_paths.insert(rel_text.clone());

        if matches!(selector, RunSelector::Commit(_)) {
            continue;
        }

        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(err) => {
                if should_preserve_snapshot(&err) {
                    summary.failed_paths.insert(rel_text);
                }
                continue;
            }
        };

        if metadata.len() > INDEX_FILE_LIMIT {
            continue;
        }

        let current_mtime_unix_ms = metadata
            .modified()
            .ok()
            .map(filters::system_time_to_unix_ms);
        let existing_state = existing_files.get(&rel_text);
        if filters::should_refresh_candidate(
            changed_since_unix_ms,
            existing_state,
            current_mtime_unix_ms,
        ) {
            summary.candidate_paths.push(rel_text.clone());
            if existing_state.is_some_and(|state| !filters::is_state_complete(state)) {
                summary.repair_backfill_paths.push(rel_text);
            }
        } else {
            summary.skipped_before_changed_since_paths.push(rel_text);
        }
    }

    Ok(summary)
}

pub(super) fn path_under_walk_error(path: &str, error_prefixes: &[String]) -> bool {
    error_prefixes.iter().any(|prefix| {
        path == prefix
            || path.starts_with(&format!("{prefix}/"))
            || prefix.starts_with(&format!("{path}/"))
    })
}

fn record_walk_error(
    project_root: &Path,
    err: walkdir::Error,
    failed_walk_prefixes: &mut Vec<String>,
) {
    if err
        .io_error()
        .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound)
    {
        return;
    }

    let Some(raw_path) = err.path() else {
        return;
    };
    let Ok(relative) = raw_path.strip_prefix(project_root) else {
        return;
    };
    let rel_text = normalize_path(relative);
    if !rel_text.is_empty() {
        failed_walk_prefixes.push(rel_text);
    }
}

fn should_preserve_snapshot(err: &std::io::Error) -> bool {
    err.kind() != std::io::ErrorKind::NotFound
}
