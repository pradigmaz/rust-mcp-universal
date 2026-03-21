use std::collections::HashMap;

use crate::engine::indexing::run::types::RunSelector;
use crate::engine::storage;
use crate::index_scope::IndexScope;
use crate::model::IndexingOptions;

use super::walk_summary::{WalkSummary, path_under_walk_error};

pub(super) fn collect_deleted_paths(
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
