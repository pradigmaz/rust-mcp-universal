use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use rmu_core::{
    DbMaintenanceOptions, Engine, MigrationMode, QualityHotspotAggregation, QualityHotspotsResult,
    RuleViolationsOptions, RuleViolationsResult, RuleViolationsSortBy, WorkspaceBrief,
};

use super::RepoRunOutcome;
use super::baseline;
use super::hotspots;
use super::manifest;
use super::notes;
use super::report;
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
    let report = build_repo_report(
        repo,
        &brief_before_refresh,
        &brief_after_refresh,
        &by_violation_count,
        &by_size_bytes,
        &by_non_empty_lines,
        &by_metric,
        &file_hotspots,
        &directory_hotspots,
        &module_hotspots,
        validations,
        index_ms,
        brief_before_refresh_ms,
        refresh_ms,
        brief_after_refresh_ms,
        violation_count_ms,
        size_bytes_ms,
        non_empty_lines_ms,
        metric_ms,
        file_hotspots_ms,
        directory_hotspots_ms,
        module_hotspots_ms,
        stats.stats.as_ref().map(|entry| entry.db_size_bytes),
        stats.stats.as_ref().map(|entry| entry.wal_size_bytes),
        stats.stats.as_ref().map(|entry| entry.total_size_bytes),
    );
    let notes_markdown = notes::notes_markdown(&report, evaluated, violating, total);

    Ok(RepoRunOutcome {
        report,
        notes_markdown,
        brief_before_refresh,
        brief_after_refresh,
        by_violation_count,
        by_size_bytes,
        by_non_empty_lines,
        by_metric_graph_edge_out_count: by_metric,
        file_hotspots,
        directory_hotspots,
        module_hotspots,
        evaluated_files: evaluated,
        violating_files: violating,
        total_violations: total,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_repo_report(
    repo: &manifest::ResolvedQualityMatrixRepository,
    brief_before_refresh: &WorkspaceBrief,
    brief_after_refresh: &WorkspaceBrief,
    by_violation_count: &RuleViolationsResult,
    by_size_bytes: &RuleViolationsResult,
    by_non_empty_lines: &RuleViolationsResult,
    by_metric: &RuleViolationsResult,
    file_hotspots: &QualityHotspotsResult,
    directory_hotspots: &QualityHotspotsResult,
    module_hotspots: &QualityHotspotsResult,
    validations: report::QualityMatrixValidations,
    index_ms: u128,
    brief_before_refresh_ms: u128,
    refresh_ms: u128,
    brief_after_refresh_ms: u128,
    violation_count_ms: u128,
    size_bytes_ms: u128,
    non_empty_lines_ms: u128,
    metric_ms: u128,
    file_hotspots_ms: u128,
    directory_hotspots_ms: u128,
    module_hotspots_ms: u128,
    db_size_bytes: Option<u64>,
    wal_size_bytes: Option<u64>,
    total_size_bytes: Option<u64>,
) -> report::QualityMatrixRepoReport {
    let evaluated = brief_after_refresh.quality_summary.evaluated_files;
    let violating = brief_after_refresh.quality_summary.violating_files;
    let total = brief_after_refresh.quality_summary.total_violations;
    let top_rule_share = brief_after_refresh
        .quality_summary
        .top_rules
        .first()
        .map(|rule| rule.violations as f64 / total.max(1) as f64)
        .unwrap_or(0.0);

    report::QualityMatrixRepoReport {
        repo_id: repo.config.id.clone(),
        role: repo.config.role.clone(),
        profile: repo.config.profile.as_str().to_string(),
        size_class: repo.config.size_class.clone(),
        expected_languages: repo.config.expected_languages.clone(),
        observed_languages: observed_languages(brief_after_refresh),
        pre_refresh_status: brief_before_refresh
            .quality_summary
            .status
            .as_str()
            .to_string(),
        post_refresh_status: brief_after_refresh
            .quality_summary
            .status
            .as_str()
            .to_string(),
        top_rules: brief_after_refresh.quality_summary.top_rules.clone(),
        top_metrics: brief_after_refresh.quality_summary.top_metrics.clone(),
        top_hot_files: report::QualityMatrixTopHotFiles {
            violation_count: top_paths(by_violation_count, 10),
            size_bytes: top_paths(by_size_bytes, 10),
            non_empty_lines: top_paths(by_non_empty_lines, 10),
            metric_graph_edge_out_count: top_paths(by_metric, 10),
        },
        top_hotspot_buckets: report::QualityMatrixTopHotspotBuckets {
            file: hotspots::top_hotspot_bucket_ids(file_hotspots, 5),
            directory: hotspots::top_hotspot_bucket_ids(directory_hotspots, 5),
            module: hotspots::top_hotspot_bucket_ids(module_hotspots, 5),
        },
        latency_summary: report::QualityMatrixLatencySummary {
            index_ms,
            brief_before_refresh_ms,
            refresh_ms,
            brief_after_refresh_ms,
            violation_count_ms,
            size_bytes_ms,
            non_empty_lines_ms,
            metric_graph_edge_out_count_ms: metric_ms,
            file_hotspots_ms,
            directory_hotspots_ms,
            module_hotspots_ms,
        },
        resource_summary: report::QualityMatrixResourceSummary {
            db_size_bytes,
            wal_size_bytes,
            total_size_bytes,
            peak_rss_bytes: None,
            peak_rss_note: "unsupported_on_host".to_string(),
        },
        noise_summary: report::QualityMatrixNoiseSummary {
            violating_file_ratio: violating as f64 / evaluated.max(1) as f64,
            top_rule_share,
            manual_review_required: true,
            review_shortlist: top_paths(by_violation_count, 5),
        },
        new_violations: file_hotspots.summary.new_violations,
        resolved_violations: file_hotspots.summary.resolved_violations,
        risk_score_delta_total: file_hotspots.summary.risk_score_delta_total,
        hotspot_score_delta_total: file_hotspots.summary.hotspot_score_delta_total,
        validations,
        artifacts: report::QualityMatrixArtifactPaths {
            brief_before_refresh: "brief.before_refresh.json".to_string(),
            brief_after_refresh: "brief.after_refresh.json".to_string(),
            violations_by_violation_count: "violations.by_violation_count.json".to_string(),
            violations_by_size_bytes: "violations.by_size_bytes.json".to_string(),
            violations_by_non_empty_lines: "violations.by_non_empty_lines.json".to_string(),
            violations_by_metric_graph_edge_out_count:
                "violations.by_metric_graph_edge_out_count.json".to_string(),
            file_hotspots: "hotspots.file.json".to_string(),
            directory_hotspots: "hotspots.directory.json".to_string(),
            module_hotspots: "hotspots.module.json".to_string(),
            notes_markdown: "notes.md".to_string(),
        },
    }
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

fn observed_languages(brief_after_refresh: &WorkspaceBrief) -> Vec<String> {
    brief_after_refresh
        .languages
        .iter()
        .map(|entry| entry.language.to_ascii_lowercase())
        .collect()
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

fn top_paths(result: &RuleViolationsResult, limit: usize) -> Vec<String> {
    result
        .hits
        .iter()
        .take(limit)
        .map(|hit| hit.path.clone())
        .collect()
}
