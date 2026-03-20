use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::Result;
use walkdir::WalkDir;

use super::indexing::run::filters;
use super::indexing::run::selector;
use super::indexing::run::types::RunSelector;
use super::{Engine, storage};
use crate::index_scope::IndexScope;
use crate::model::{IndexingOptions, ScopePreviewResult};
use crate::utils::{ProjectIgnoreMatcher, is_probably_ignored, normalize_path};
use crate::vector_rank::semantic_model_name;

impl Engine {
    pub fn scope_preview_with_options(
        &self,
        options: &IndexingOptions,
    ) -> Result<ScopePreviewResult> {
        let scope = IndexScope::new(options)?;
        let ignore_matcher = ProjectIgnoreMatcher::new(&self.project_root)?;
        let semantic_model = semantic_model_name();
        let existing_files = load_existing_file_state_read_only(self, &semantic_model)?;
        let selector = selector::resolve_run_selector(
            self,
            options,
            &scope,
            &existing_files,
            &ignore_matcher,
        )?;
        let walk_summary =
            collect_walk_summary(self, &scope, &selector, &existing_files, &ignore_matcher)?;
        let deleted_paths =
            collect_deleted_paths(options, &scope, &selector, &existing_files, &walk_summary);

        let mut candidate_paths = walk_summary.candidate_paths;
        let mut excluded_by_scope_paths = walk_summary.excluded_by_scope_paths;
        let mut ignored_paths = walk_summary.ignored_paths;
        let mut skipped_before_changed_since_paths =
            walk_summary.skipped_before_changed_since_paths;
        let mut repair_backfill_paths = walk_summary.repair_backfill_paths;
        let mut deleted_paths = deleted_paths;

        sort_and_dedup(&mut candidate_paths);
        sort_and_dedup(&mut excluded_by_scope_paths);
        sort_and_dedup(&mut ignored_paths);
        sort_and_dedup(&mut skipped_before_changed_since_paths);
        sort_and_dedup(&mut repair_backfill_paths);
        sort_and_dedup(&mut deleted_paths);

        let resolved_merge_base_commit = match &selector {
            RunSelector::Commit(commit_selector) => {
                Some(commit_selector.resolved_merge_base_commit.clone())
            }
            RunSelector::Full | RunSelector::Timestamp { .. } => None,
        };

        Ok(ScopePreviewResult {
            profile: options.profile,
            changed_since: options.changed_since,
            changed_since_commit: options.changed_since_commit.clone(),
            resolved_merge_base_commit,
            reindex: options.reindex,
            include_paths: options.include_paths.clone(),
            exclude_paths: options.exclude_paths.clone(),
            scanned_files: walk_summary.scanned_files,
            candidate_count: candidate_paths.len(),
            excluded_by_scope_count: excluded_by_scope_paths.len(),
            ignored_count: ignored_paths.len(),
            skipped_before_changed_since_count: skipped_before_changed_since_paths.len(),
            repair_backfill_count: repair_backfill_paths.len(),
            deleted_count: deleted_paths.len(),
            candidate_paths,
            excluded_by_scope_paths,
            ignored_paths,
            skipped_before_changed_since_paths,
            repair_backfill_paths,
            deleted_paths,
        })
    }
}

fn load_existing_file_state_read_only(
    engine: &Engine,
    semantic_model: &str,
) -> Result<HashMap<String, storage::ExistingFileState>> {
    if !engine.db_path.exists() {
        return Ok(HashMap::new());
    }

    let mut conn = engine.open_db_read_only()?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)?;
    let existing_files = storage::load_existing_file_state(&tx, semantic_model)?;
    tx.commit()?;
    Ok(existing_files)
}

#[derive(Default)]
struct WalkSummary {
    scanned_files: usize,
    candidate_paths: Vec<String>,
    excluded_by_scope_paths: Vec<String>,
    ignored_paths: Vec<String>,
    skipped_before_changed_since_paths: Vec<String>,
    repair_backfill_paths: Vec<String>,
    present_paths: HashSet<String>,
    failed_paths: HashSet<String>,
    failed_walk_prefixes: Vec<String>,
}

fn collect_walk_summary(
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

        if metadata.len() > crate::utils::INDEX_FILE_LIMIT {
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

fn collect_deleted_paths(
    options: &IndexingOptions,
    scope: &IndexScope,
    selector: &RunSelector,
    existing_files: &HashMap<String, storage::ExistingFileState>,
    walk_summary: &WalkSummary,
) -> Vec<String> {
    let mut deleted_paths = Vec::new();

    for path in existing_files.keys() {
        if !options.reindex && scope.has_rules() && !scope.allows(path) {
            deleted_paths.push(path.clone());
            continue;
        }

        if let RunSelector::Commit(commit_selector) = selector {
            if commit_selector.deleted_paths.contains(path) {
                deleted_paths.push(path.clone());
            }
            continue;
        }

        if walk_summary.present_paths.contains(path) {
            continue;
        }
        if walk_summary.failed_paths.contains(path)
            || path_under_walk_error(path, &walk_summary.failed_walk_prefixes)
        {
            continue;
        }
        deleted_paths.push(path.clone());
    }

    deleted_paths
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

fn path_under_walk_error(path: &str, error_prefixes: &[String]) -> bool {
    error_prefixes.iter().any(|prefix| {
        path == prefix
            || path.starts_with(&format!("{prefix}/"))
            || prefix.starts_with(&format!("{path}/"))
    })
}

fn sort_and_dedup(paths: &mut Vec<String>) {
    paths.sort();
    paths.dedup();
}
