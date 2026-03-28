use std::path::Path;

use anyhow::{Context, Result, bail};
use rmu_core::{MigrationMode, PrivacyMode};

use crate::output::{print_json, print_line};

use super::QualityMatrixArgs;
use super::artifacts;
use super::baseline;
use super::manifest;
use super::repo;
use super::report;

pub(super) fn run(
    project_root: &Path,
    json: bool,
    privacy_mode: PrivacyMode,
    migration_mode: MigrationMode,
    args: QualityMatrixArgs,
) -> Result<()> {
    let manifest_path = manifest::resolve_from_project_root(project_root, &args.manifest);
    let override_path = args
        .override_path
        .as_deref()
        .map(|path| manifest::resolve_from_project_root(project_root, path))
        .unwrap_or_else(|| manifest::default_override_path(project_root));
    let explicit_output_root = args.output_root.is_some();
    let output_root = args
        .output_root
        .as_deref()
        .map(|path| manifest::resolve_from_project_root(project_root, path))
        .unwrap_or_else(|| manifest::default_output_root(project_root));
    let canonical_summary_path = manifest::canonical_summary_path(&manifest_path)?;
    let manifest = manifest::load_manifest(&manifest_path)?;
    let overrides = manifest::load_local_overrides(&override_path)?;
    let repos = manifest::select_repositories(&manifest, &overrides, &args.repo_ids)?;
    if repos.is_empty() {
        bail!("quality-matrix selected zero repositories");
    }

    let run_root = prepare_run_root(project_root, &output_root, explicit_output_root)?;

    let mut repo_reports = Vec::new();
    let mut evaluated_files = Vec::new();
    let mut violating_files = Vec::new();
    let mut total_violations = Vec::new();

    for repo in repos {
        let repo_output_root = run_root.join(&repo.config.id);
        artifacts::create_directory(&repo_output_root)?;
        let outcome = repo::run_repo(project_root, &repo, migration_mode)?;
        artifacts::write_repo_artifacts(&repo_output_root, &outcome)?;
        baseline::write_baseline_artifact(
            project_root,
            &repo.config.id,
            rmu_core::QualityHotspotAggregation::File,
            &outcome.file_hotspots,
        )?;
        baseline::write_baseline_artifact(
            project_root,
            &repo.config.id,
            rmu_core::QualityHotspotAggregation::Directory,
            &outcome.directory_hotspots,
        )?;
        baseline::write_baseline_artifact(
            project_root,
            &repo.config.id,
            rmu_core::QualityHotspotAggregation::Module,
            &outcome.module_hotspots,
        )?;
        evaluated_files.push(outcome.evaluated_files);
        violating_files.push(outcome.violating_files);
        total_violations.push(outcome.total_violations);
        repo_reports.push(outcome.report);
    }

    let aggregate = report::new_aggregate_report(
        manifest.version,
        &run_root,
        &canonical_summary_path,
        repo_reports,
    )?;
    artifacts::write_json_file(&run_root.join("aggregate.json"), &aggregate)?;
    let canonical = report::build_canonical_summary(
        &aggregate,
        &evaluated_files,
        &violating_files,
        &total_violations,
    );
    if let Some(parent) = canonical_summary_path.parent() {
        artifacts::create_directory(parent)?;
    }
    artifacts::write_json_file(&canonical_summary_path, &canonical)?;

    if json {
        let value = report::sanitize_for_output(&aggregate, privacy_mode)?;
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        print_line(report::text_summary(&aggregate, privacy_mode));
    }
    Ok(())
}

fn prepare_run_root(
    project_root: &Path,
    output_root: &Path,
    explicit_output_root: bool,
) -> Result<std::path::PathBuf> {
    let run_stamp = artifacts::run_stamp()?;
    let preferred = output_root.join(&run_stamp);
    if artifacts::create_directory(&preferred).is_ok() {
        return Ok(preferred);
    }
    if explicit_output_root {
        artifacts::create_directory(&preferred)?;
        return Ok(preferred);
    }

    let fallback_root = project_root.join("target/quality-matrix-runs");
    let fallback = fallback_root.join(&run_stamp);
    artifacts::create_directory(&fallback).with_context(|| {
        format!(
            "failed to create preferred quality-matrix output root `{}` and fallback `{}`",
            preferred.display(),
            fallback.display()
        )
    })?;
    Ok(fallback)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::prepare_run_root;

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    #[test]
    fn default_output_root_falls_back_to_target_when_preferred_root_is_unwritable() {
        let root = temp_dir("rmu-quality-matrix-run-fallback");
        fs::create_dir_all(root.join(".codex/quality-matrix")).expect("create quality-matrix dir");
        fs::create_dir_all(root.join("target")).expect("create target dir");
        fs::write(root.join(".codex/quality-matrix/runs"), "occupied-by-file")
            .expect("create blocking file");

        let run_root = prepare_run_root(&root, &root.join(".codex/quality-matrix/runs"), false)
            .expect("fallback run root");

        assert!(run_root.starts_with(root.join("target/quality-matrix-runs")));
        assert!(run_root.exists());

        let _ = fs::remove_dir_all(root);
    }
}
