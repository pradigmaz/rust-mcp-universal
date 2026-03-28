use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use rmu_core::{
    DbMaintenanceOptions, Engine, MigrationMode, QualityHotspotAggregation, RuleViolationsOptions,
    RuleViolationsResult, RuleViolationsSortBy,
};

use super::RepoRunOutcome;
use super::baseline;
use super::builder;
use super::hotspots;
use super::manifest;
use super::notes;
use super::validate;

pub(super) fn run_repo(
    matrix_project_root: &Path,
    repo: &manifest::ResolvedQualityMatrixRepository,
    migration_mode: MigrationMode,
) -> Result<RepoRunOutcome> {
    let engine = Engine::new_with_migration_mode(&repo.project_path, None, migration_mode)?;
    let index_ms = maybe_index_repo(&engine, &repo.config)?;
    let (brief_before_refresh, brief_before_refresh_ms) =
        timed(|| engine.workspace_brief_with_policy(false))?;
    let refresh_ms = timed_unit(|| engine.refresh_quality_if_needed())?;
    let (brief_after_refresh, brief_after_refresh_ms) =
        timed(|| engine.workspace_brief_with_policy(false))?;

    let (by_violation_count, violation_count_ms) =
        timed(|| run_rule_violations(&engine, RuleViolationsSortBy::ViolationCount, None))?;
    let (by_size_bytes, size_bytes_ms) =
        timed(|| run_rule_violations(&engine, RuleViolationsSortBy::SizeBytes, None))?;
    let (by_non_empty_lines, non_empty_lines_ms) =
        timed(|| run_rule_violations(&engine, RuleViolationsSortBy::NonEmptyLines, None))?;
    let (by_metric, metric_ms) = timed(|| {
        run_rule_violations(
            &engine,
            RuleViolationsSortBy::MetricValue,
            Some("graph_edge_out_count".to_string()),
        )
    })?;
    let (by_complexity, complexity_ms) = timed(|| {
        run_rule_violations(
            &engine,
            RuleViolationsSortBy::MetricValue,
            Some("max_cognitive_complexity".to_string()),
        )
    })?;
    let (by_duplication, duplication_ms) = timed(|| {
        run_rule_violations(
            &engine,
            RuleViolationsSortBy::MetricValue,
            Some("duplicate_density_bps".to_string()),
        )
    })?;
    let (mut file_hotspots, file_hotspots_ms) =
        timed(|| hotspots::run_quality_hotspots(&engine, QualityHotspotAggregation::File))?;
    let (mut directory_hotspots, directory_hotspots_ms) =
        timed(|| hotspots::run_quality_hotspots(&engine, QualityHotspotAggregation::Directory))?;
    let (mut module_hotspots, module_hotspots_ms) =
        timed(|| hotspots::run_quality_hotspots(&engine, QualityHotspotAggregation::Module))?;

    baseline::apply_baseline_deltas(
        &mut file_hotspots,
        baseline::load_baseline_artifact(
            matrix_project_root,
            &repo.config.id,
            QualityHotspotAggregation::File,
        )?,
    );
    baseline::apply_baseline_deltas(
        &mut directory_hotspots,
        baseline::load_baseline_artifact(
            matrix_project_root,
            &repo.config.id,
            QualityHotspotAggregation::Directory,
        )?,
    );
    baseline::apply_baseline_deltas(
        &mut module_hotspots,
        baseline::load_baseline_artifact(
            matrix_project_root,
            &repo.config.id,
            QualityHotspotAggregation::Module,
        )?,
    );

    let validations =
        validate::validate_repo(repo, &engine, &brief_before_refresh, &brief_after_refresh)?;
    let stats = engine.db_maintenance(DbMaintenanceOptions {
        integrity_check: false,
        checkpoint: false,
        vacuum: false,
        analyze: false,
        stats: true,
        prune: false,
    })?;

    let evaluated = brief_after_refresh.quality_summary.evaluated_files;
    let violating = brief_after_refresh.quality_summary.violating_files;
    let total = brief_after_refresh.quality_summary.total_violations;
    let duplication_clone_classes = builder::load_duplication_artifact(&repo.project_path)?;
    let report = builder::build_repo_report(
        repo,
        &brief_before_refresh,
        &brief_after_refresh,
        &by_violation_count,
        &by_size_bytes,
        &by_non_empty_lines,
        &by_metric,
        &by_complexity,
        &by_duplication,
        &file_hotspots,
        &directory_hotspots,
        &module_hotspots,
        duplication_clone_classes.as_ref(),
        validations,
        index_ms,
        brief_before_refresh_ms,
        refresh_ms,
        brief_after_refresh_ms,
        violation_count_ms,
        size_bytes_ms,
        non_empty_lines_ms,
        metric_ms,
        complexity_ms,
        duplication_ms,
        file_hotspots_ms,
        directory_hotspots_ms,
        module_hotspots_ms,
        stats.stats.as_ref().map(|entry| entry.db_size_bytes),
        stats.stats.as_ref().map(|entry| entry.wal_size_bytes),
        stats.stats.as_ref().map(|entry| entry.total_size_bytes),
    )?;
    let notes_markdown = notes::notes_markdown(
        &report,
        evaluated,
        violating,
        total,
        duplication_clone_classes.as_ref(),
        repo.project_path.join("rmu-quality-policy.json").exists(),
    );

    Ok(RepoRunOutcome {
        report,
        notes_markdown,
        brief_before_refresh,
        brief_after_refresh,
        by_violation_count,
        by_size_bytes,
        by_non_empty_lines,
        by_metric_graph_edge_out_count: by_metric,
        by_metric_max_cognitive_complexity: by_complexity,
        by_metric_duplicate_density_bps: by_duplication,
        file_hotspots,
        directory_hotspots,
        module_hotspots,
        duplication_clone_classes,
        evaluated_files: evaluated,
        violating_files: violating,
        total_violations: total,
    })
}

fn maybe_index_repo(engine: &Engine, repo: &manifest::QualityMatrixRepository) -> Result<u128> {
    let status = engine.index_status()?;
    let needs_reindex = if status.files == 0 {
        false
    } else {
        engine
            .workspace_brief_with_policy(false)?
            .repair_hint
            .is_some()
    };
    if status.files > 0 && !needs_reindex {
        return Ok(0);
    }
    timed_unit(|| {
        engine.index_path_with_options(&rmu_core::IndexingOptions {
            profile: Some(repo.profile),
            include_paths: repo.include_paths.clone(),
            exclude_paths: repo.exclude_paths.clone(),
            reindex: needs_reindex,
            ..rmu_core::IndexingOptions::default()
        })
    })
}

fn run_rule_violations(
    engine: &Engine,
    sort_by: RuleViolationsSortBy,
    sort_metric_id: Option<String>,
) -> Result<RuleViolationsResult> {
    engine.rule_violations(&RuleViolationsOptions {
        limit: 20,
        sort_by,
        sort_metric_id,
        ..RuleViolationsOptions::default()
    })
}

fn timed<T>(f: impl FnOnce() -> Result<T>) -> Result<(T, u128)> {
    let started = Instant::now();
    let value = f()?;
    Ok((value, started.elapsed().as_millis()))
}

fn timed_unit<T>(f: impl FnOnce() -> Result<T>) -> Result<u128> {
    let started = Instant::now();
    let _ = f()?;
    Ok(started.elapsed().as_millis())
}
