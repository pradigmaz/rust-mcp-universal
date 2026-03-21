#[path = "preview/deleted_paths.rs"]
mod deleted_paths;
#[path = "preview/state_load.rs"]
mod state_load;
#[path = "preview/walk_summary.rs"]
mod walk_summary;

use anyhow::Result;

use super::Engine;
use super::indexing::run::selector;
use super::indexing::run::types::RunSelector;
use crate::index_scope::IndexScope;
use crate::model::{IndexingOptions, ScopePreviewResult};
use crate::utils::ProjectIgnoreMatcher;
use crate::vector_rank::semantic_model_name;

impl Engine {
    pub fn scope_preview_with_options(
        &self,
        options: &IndexingOptions,
    ) -> Result<ScopePreviewResult> {
        let options = self.resolve_indexing_options(options);
        let scope = IndexScope::new(&options)?;
        let ignore_matcher = ProjectIgnoreMatcher::new(&self.project_root)?;
        let semantic_model = semantic_model_name();
        let existing_files = state_load::load_existing_file_state_read_only(self, &semantic_model)?;
        let selector = selector::resolve_run_selector(
            self,
            &options,
            &scope,
            &existing_files,
            &ignore_matcher,
        )?;
        let walk_summary = walk_summary::collect_walk_summary(
            self,
            &scope,
            &selector,
            &existing_files,
            &ignore_matcher,
        )?;
        let mut deleted_paths = deleted_paths::collect_deleted_paths(
            &options,
            &scope,
            &selector,
            &existing_files,
            &walk_summary,
        );
        let mut candidate_paths = walk_summary.candidate_paths;
        let mut excluded_by_scope_paths = walk_summary.excluded_by_scope_paths;
        let mut ignored_paths = walk_summary.ignored_paths;
        let mut skipped_before_changed_since_paths =
            walk_summary.skipped_before_changed_since_paths;
        let mut repair_backfill_paths = walk_summary.repair_backfill_paths;

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

fn sort_and_dedup(paths: &mut Vec<String>) {
    paths.sort();
    paths.dedup();
}
