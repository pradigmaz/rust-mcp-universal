use std::path::{Path, PathBuf};

use anyhow::Result;
use rmu_core::{
    MigrationMode, PrivacyMode, QualityHotspotsResult, RuleViolationsResult, WorkspaceBrief,
};

mod artifacts;
mod baseline;
mod hotspots;
mod manifest;
mod notes;
mod repo;
mod report;
mod run;
mod validate;

pub(crate) struct QualityMatrixArgs {
    pub(crate) manifest: PathBuf,
    pub(crate) override_path: Option<PathBuf>,
    pub(crate) output_root: Option<PathBuf>,
    pub(crate) repo_ids: Vec<String>,
}

pub(super) struct RepoRunOutcome {
    pub(super) report: report::QualityMatrixRepoReport,
    pub(super) notes_markdown: String,
    pub(super) brief_before_refresh: WorkspaceBrief,
    pub(super) brief_after_refresh: WorkspaceBrief,
    pub(super) by_violation_count: RuleViolationsResult,
    pub(super) by_size_bytes: RuleViolationsResult,
    pub(super) by_non_empty_lines: RuleViolationsResult,
    pub(super) by_metric_graph_edge_out_count: RuleViolationsResult,
    pub(super) by_metric_max_cognitive_complexity: RuleViolationsResult,
    pub(super) file_hotspots: QualityHotspotsResult,
    pub(super) directory_hotspots: QualityHotspotsResult,
    pub(super) module_hotspots: QualityHotspotsResult,
    pub(super) evaluated_files: usize,
    pub(super) violating_files: usize,
    pub(super) total_violations: usize,
}

pub(crate) fn run(
    project_root: &Path,
    json: bool,
    privacy_mode: PrivacyMode,
    migration_mode: MigrationMode,
    args: QualityMatrixArgs,
) -> Result<()> {
    run::run(project_root, json, privacy_mode, migration_mode, args)
}
