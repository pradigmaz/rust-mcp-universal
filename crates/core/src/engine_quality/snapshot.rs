use anyhow::{Result, bail};
use rusqlite::params;

use super::hotspots::load_quality_hotspots;
use super::rule_violations::load_rule_violations;
use super::snapshot_artifacts::{
    SNAPSHOT_REPORT_NAME, load_latest_wave_before_snapshot, load_self_baseline_snapshot,
    persist_delta_artifact, persist_self_baseline_artifacts, persist_snapshot_artifacts,
    snapshot_exclude_paths, snapshot_output_root,
};
use super::snapshot_delta::{build_delta_report, now_rfc3339};
use crate::engine::Engine;
use crate::model::{
    IndexingOptions, QualityHotspotAggregation, QualityHotspotsOptions, QualityHotspotsResult,
    QualityHotspotsSortBy, QualityProjectArtifactPaths, QualityProjectSnapshotCapture,
    QualityProjectSnapshotCompareAgainst, QualityProjectSnapshotKind,
    QualityProjectSnapshotOptions, QualityProjectSnapshotReport, QualityProjectTopHotFiles,
    QualityProjectTopHotspotBuckets, QualityStatus, RuleViolationsOptions, RuleViolationsResult,
    RuleViolationsSortBy, WorkspaceBrief,
};

const FULL_SCAN_LIMIT: usize = 100_000;
const TOP_PATHS_LIMIT: usize = 10;
const TOP_BUCKETS_LIMIT: usize = 5;

pub(super) fn capture_quality_project_snapshot(
    engine: &Engine,
    options: &QualityProjectSnapshotOptions,
) -> Result<QualityProjectSnapshotCapture> {
    validate_options(options)?;

    if options.auto_index {
        let _ = engine.index_path_with_options(&IndexingOptions {
            exclude_paths: snapshot_exclude_paths(
                &engine.project_root,
                options.output_root.as_deref(),
            ),
            ..IndexingOptions::default()
        })?;
    } else if !engine.db_path.exists() {
        bail!(
            "index is empty; run an indexing flow or enable automatic indexing before capturing quality snapshot"
        );
    }

    let baseline_snapshot = match options.compare_against {
        QualityProjectSnapshotCompareAgainst::None => None,
        QualityProjectSnapshotCompareAgainst::SelfBaseline => {
            Some(load_self_baseline_snapshot(&engine.project_root)?)
        }
        QualityProjectSnapshotCompareAgainst::WaveBefore => {
            let wave_id = options.wave_id.as_deref().ok_or_else(|| {
                anyhow::anyhow!("`wave_id` is required when compare_against=wave_before")
            })?;
            Some(load_latest_wave_before_snapshot(
                &engine.project_root,
                wave_id,
                options.output_root.as_deref(),
            )?)
        }
    };

    let brief_before_refresh = engine.workspace_brief_with_policy(false)?;
    let refresh_performed = brief_before_refresh.quality_summary.status != QualityStatus::Ready;
    engine.refresh_quality_if_needed()?;
    let brief_after_refresh = engine.workspace_brief_with_policy(false)?;

    let snapshot = build_snapshot_report(
        engine,
        options.snapshot_kind,
        options.wave_id.clone(),
        &brief_before_refresh,
        &brief_after_refresh,
        refresh_performed,
    )?;

    let delta = baseline_snapshot
        .as_ref()
        .map(|baseline| build_delta_report(baseline, &snapshot, options.compare_against));

    let mut artifacts = QualityProjectArtifactPaths::default();
    if options.persist_artifacts {
        if let Some(snapshot_root) = snapshot_output_root(
            &engine.project_root,
            options.output_root.as_deref(),
            options.snapshot_kind,
            options.wave_id.as_deref(),
        )? {
            persist_snapshot_artifacts(&snapshot_root, &snapshot)?;
            artifacts.snapshot_root = Some(snapshot_root.display().to_string());
            artifacts.snapshot_report = Some(
                snapshot_root
                    .join(SNAPSHOT_REPORT_NAME)
                    .display()
                    .to_string(),
            );
        }
    }

    if let Some(delta_report) = &delta {
        if options.persist_artifacts {
            if let Some(delta_path) = persist_delta_artifact(
                &engine.project_root,
                options.output_root.as_deref(),
                options.wave_id.as_deref(),
                delta_report,
            )? {
                artifacts.delta_report = Some(delta_path.display().to_string());
            }
        }
    }

    if options.promote_self_baseline
        || options.snapshot_kind == QualityProjectSnapshotKind::Baseline
    {
        let baseline_path = persist_self_baseline_artifacts(&engine.project_root, &snapshot)?;
        artifacts.baseline_summary = Some(baseline_path.display().to_string());
    }

    Ok(QualityProjectSnapshotCapture {
        snapshot,
        delta,
        artifacts,
    })
}

fn validate_options(options: &QualityProjectSnapshotOptions) -> Result<()> {
    if matches!(
        options.snapshot_kind,
        QualityProjectSnapshotKind::Before | QualityProjectSnapshotKind::After
    ) && options.wave_id.as_deref().is_none_or(str::is_empty)
    {
        bail!("`wave_id` is required for before/after quality snapshots");
    }
    if options.compare_against == QualityProjectSnapshotCompareAgainst::WaveBefore
        && options.wave_id.as_deref().is_none_or(str::is_empty)
    {
        bail!("`wave_id` is required when compare_against=wave_before");
    }
    Ok(())
}

fn build_snapshot_report(
    engine: &Engine,
    snapshot_kind: QualityProjectSnapshotKind,
    wave_id: Option<String>,
    brief_before_refresh: &WorkspaceBrief,
    brief_after_refresh: &WorkspaceBrief,
    refresh_performed: bool,
) -> Result<QualityProjectSnapshotReport> {
    let by_violation_count =
        run_rule_violations(engine, RuleViolationsSortBy::ViolationCount, None)?;
    let by_size_bytes = run_rule_violations(engine, RuleViolationsSortBy::SizeBytes, None)?;
    let by_non_empty_lines =
        run_rule_violations(engine, RuleViolationsSortBy::NonEmptyLines, None)?;
    let by_metric_graph_edge_out_count = run_rule_violations(
        engine,
        RuleViolationsSortBy::MetricValue,
        Some("graph_edge_out_count".to_string()),
    )?;
    let by_metric_max_cognitive_complexity = run_rule_violations(
        engine,
        RuleViolationsSortBy::MetricValue,
        Some("max_cognitive_complexity".to_string()),
    )?;
    let by_metric_duplicate_density_bps = run_rule_violations(
        engine,
        RuleViolationsSortBy::MetricValue,
        Some("duplicate_density_bps".to_string()),
    )?;

    let file_hotspots = run_quality_hotspots(engine, QualityHotspotAggregation::File)?;
    let directory_hotspots = run_quality_hotspots(engine, QualityHotspotAggregation::Directory)?;
    let module_hotspots = run_quality_hotspots(engine, QualityHotspotAggregation::Module)?;
    let (total_non_empty_lines, total_size_bytes) = load_quality_aggregate_totals(engine)?;

    Ok(QualityProjectSnapshotReport {
        generated_at_utc: now_rfc3339()?,
        snapshot_kind,
        wave_id,
        quality_status_before_refresh: brief_before_refresh.quality_summary.status,
        quality_status_after_refresh: brief_after_refresh.quality_summary.status,
        refresh_performed,
        ruleset_id: brief_after_refresh.quality_summary.ruleset_id.clone(),
        evaluated_files: brief_after_refresh.quality_summary.evaluated_files,
        violating_files: brief_after_refresh.quality_summary.violating_files,
        total_violations: brief_after_refresh.quality_summary.total_violations,
        suppressed_violations: brief_after_refresh.quality_summary.suppressed_violations,
        total_non_empty_lines,
        total_size_bytes,
        top_rules: brief_after_refresh.quality_summary.top_rules.clone(),
        top_metrics: brief_after_refresh.quality_summary.top_metrics.clone(),
        top_hot_files: QualityProjectTopHotFiles {
            violation_count: top_paths(&by_violation_count, TOP_PATHS_LIMIT),
            size_bytes: top_paths(&by_size_bytes, TOP_PATHS_LIMIT),
            non_empty_lines: top_paths(&by_non_empty_lines, TOP_PATHS_LIMIT),
            metric_graph_edge_out_count: top_paths(
                &by_metric_graph_edge_out_count,
                TOP_PATHS_LIMIT,
            ),
            metric_max_cognitive_complexity: top_paths(
                &by_metric_max_cognitive_complexity,
                TOP_PATHS_LIMIT,
            ),
            metric_duplicate_density_bps: top_paths(
                &by_metric_duplicate_density_bps,
                TOP_PATHS_LIMIT,
            ),
        },
        top_hotspot_buckets: QualityProjectTopHotspotBuckets {
            file: top_bucket_ids(&file_hotspots, TOP_BUCKETS_LIMIT),
            directory: top_bucket_ids(&directory_hotspots, TOP_BUCKETS_LIMIT),
            module: top_bucket_ids(&module_hotspots, TOP_BUCKETS_LIMIT),
        },
        rule_violations_by_violation_count: by_violation_count,
        rule_violations_by_size_bytes: by_size_bytes,
        rule_violations_by_non_empty_lines: by_non_empty_lines,
        rule_violations_by_metric_graph_edge_out_count: by_metric_graph_edge_out_count,
        rule_violations_by_metric_max_cognitive_complexity: by_metric_max_cognitive_complexity,
        rule_violations_by_metric_duplicate_density_bps: by_metric_duplicate_density_bps,
        file_hotspots,
        directory_hotspots,
        module_hotspots,
    })
}

fn run_rule_violations(
    engine: &Engine,
    sort_by: RuleViolationsSortBy,
    sort_metric_id: Option<String>,
) -> Result<RuleViolationsResult> {
    load_rule_violations(
        engine,
        &RuleViolationsOptions {
            limit: FULL_SCAN_LIMIT,
            sort_by,
            sort_metric_id,
            ..RuleViolationsOptions::default()
        },
    )
}

fn run_quality_hotspots(
    engine: &Engine,
    aggregation: QualityHotspotAggregation,
) -> Result<QualityHotspotsResult> {
    load_quality_hotspots(
        engine,
        &QualityHotspotsOptions {
            limit: FULL_SCAN_LIMIT,
            aggregation,
            sort_by: QualityHotspotsSortBy::HotspotScore,
            ..QualityHotspotsOptions::default()
        },
    )
}

fn load_quality_aggregate_totals(engine: &Engine) -> Result<(i64, i64)> {
    if !engine.db_path.exists() {
        return Ok((0, 0));
    }
    let conn = engine.open_db_read_only()?;
    let totals = conn.query_row(
        "SELECT COALESCE(SUM(COALESCE(non_empty_lines, 0)), 0), COALESCE(SUM(size_bytes), 0) FROM file_quality",
        params![],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )?;
    Ok(totals)
}

fn top_paths(result: &RuleViolationsResult, limit: usize) -> Vec<String> {
    result
        .hits
        .iter()
        .take(limit)
        .map(|hit| hit.path.clone())
        .collect()
}

fn top_bucket_ids(result: &QualityHotspotsResult, limit: usize) -> Vec<String> {
    result
        .buckets
        .iter()
        .take(limit)
        .map(|bucket| bucket.bucket_id.clone())
        .collect()
}
