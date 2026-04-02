use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rusqlite::params;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::hotspots::load_quality_hotspots;
use super::query::load_rule_violations;
use crate::engine::Engine;
use crate::model::{
    IndexingOptions, QualityDeltaSummary, QualityHotspotAggregation, QualityHotspotBucket,
    QualityHotspotsOptions, QualityHotspotsResult, QualityHotspotsSortBy,
    QualityProjectArtifactPaths,
    QualityProjectDeltaReport, QualityProjectGateStatus, QualityProjectHotspotDelta,
    QualityProjectSnapshotCapture, QualityProjectSnapshotCompareAgainst,
    QualityProjectSnapshotKind, QualityProjectSnapshotOptions, QualityProjectSnapshotReport,
    QualityProjectTopHotFiles, QualityProjectTopHotspotBuckets, QualityStatus,
    RuleViolationsOptions, RuleViolationsResult, RuleViolationsSortBy,
    WorkspaceBrief,
};

const FULL_SCAN_LIMIT: usize = 100_000;
const TOP_PATHS_LIMIT: usize = 10;
const TOP_BUCKETS_LIMIT: usize = 5;
const SNAPSHOT_REPORT_NAME: &str = "snapshot.report.json";
const BASELINE_SUMMARY_NAME: &str = "baseline-summary.json";

pub(super) fn capture_quality_project_snapshot(
    engine: &Engine,
    options: &QualityProjectSnapshotOptions,
) -> Result<QualityProjectSnapshotCapture> {
    validate_options(options)?;

    if options.auto_index {
        let _ = engine.index_path_with_options(&IndexingOptions {
            exclude_paths: snapshot_exclude_paths(&engine.project_root, options.output_root.as_deref()),
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

    let delta = baseline_snapshot.as_ref().map(|baseline| {
        build_delta_report(baseline, &snapshot, options.compare_against)
    });

    let mut artifacts = QualityProjectArtifactPaths::default();
    if options.persist_artifacts {
        if let Some(snapshot_root) =
            snapshot_output_root(
                &engine.project_root,
                options.output_root.as_deref(),
                options.snapshot_kind,
                options.wave_id.as_deref(),
            )?
        {
            persist_snapshot_artifacts(&snapshot_root, &snapshot)?;
            artifacts.snapshot_root = Some(snapshot_root.display().to_string());
            artifacts.snapshot_report =
                Some(snapshot_root.join(SNAPSHOT_REPORT_NAME).display().to_string());
        }
    }

    if let Some(delta_report) = &delta {
        if options.persist_artifacts {
            if let Some(delta_path) =
                persist_delta_artifact(
                    &engine.project_root,
                    options.output_root.as_deref(),
                    options.wave_id.as_deref(),
                    delta_report,
                )?
            {
                artifacts.delta_report = Some(delta_path.display().to_string());
            }
        }
    }

    if options.promote_self_baseline || options.snapshot_kind == QualityProjectSnapshotKind::Baseline
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

fn build_delta_report(
    baseline: &QualityProjectSnapshotReport,
    candidate: &QualityProjectSnapshotReport,
    compare_against: QualityProjectSnapshotCompareAgainst,
) -> QualityProjectDeltaReport {
    let (new_violations, resolved_violations) = compare_violation_multisets(
        &baseline.rule_violations_by_violation_count,
        &candidate.rule_violations_by_violation_count,
    );
    let mut regression_reasons = Vec::new();
    if candidate.quality_status_after_refresh != QualityStatus::Ready {
        regression_reasons.push(format!(
            "post_refresh_status={}",
            candidate.quality_status_after_refresh.as_str()
        ));
    }
    if new_violations > 0 {
        regression_reasons.push(format!("new_violations={new_violations}"));
    }

    QualityProjectDeltaReport {
        generated_at_utc: now_rfc3339().unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string()),
        compare_against,
        baseline_generated_at_utc: baseline.generated_at_utc.clone(),
        candidate_generated_at_utc: candidate.generated_at_utc.clone(),
        total_violations_delta: candidate.total_violations as i64 - baseline.total_violations as i64,
        violating_files_delta: candidate.violating_files as i64 - baseline.violating_files as i64,
        suppressed_violations_delta: candidate.suppressed_violations as i64
            - baseline.suppressed_violations as i64,
        total_non_empty_lines_delta: candidate.total_non_empty_lines - baseline.total_non_empty_lines,
        total_size_bytes_delta: candidate.total_size_bytes - baseline.total_size_bytes,
        new_violations,
        resolved_violations,
        file_hotspots: compare_hotspot_results(&baseline.file_hotspots, &candidate.file_hotspots),
        directory_hotspots: compare_hotspot_results(
            &baseline.directory_hotspots,
            &candidate.directory_hotspots,
        ),
        module_hotspots: compare_hotspot_results(&baseline.module_hotspots, &candidate.module_hotspots),
        gate_status: if regression_reasons.is_empty() {
            QualityProjectGateStatus::Ok
        } else {
            QualityProjectGateStatus::Regression
        },
        regression_reasons,
    }
}

fn compare_violation_multisets(
    baseline: &RuleViolationsResult,
    candidate: &RuleViolationsResult,
) -> (usize, usize) {
    let mut counts = BTreeMap::<String, i64>::new();

    for hit in &candidate.hits {
        for violation in &hit.violations {
            *counts
                .entry(violation_fingerprint(&hit.path, violation))
                .or_default() += 1;
        }
    }
    for hit in &baseline.hits {
        for violation in &hit.violations {
            *counts
                .entry(violation_fingerprint(&hit.path, violation))
                .or_default() -= 1;
        }
    }

    let mut new_violations = 0usize;
    let mut resolved_violations = 0usize;
    for delta in counts.into_values() {
        if delta > 0 {
            new_violations += delta as usize;
        } else if delta < 0 {
            resolved_violations += (-delta) as usize;
        }
    }
    (new_violations, resolved_violations)
}

fn compare_hotspot_results(
    baseline: &QualityHotspotsResult,
    candidate: &QualityHotspotsResult,
) -> QualityProjectHotspotDelta {
    let mut baseline_buckets = baseline
        .buckets
        .iter()
        .cloned()
        .map(|bucket| (bucket.bucket_id.clone(), bucket))
        .collect::<HashMap<_, _>>();

    let mut delta = QualityProjectHotspotDelta::default();
    for bucket in &candidate.buckets {
        let current_delta = build_bucket_delta(bucket, baseline_buckets.remove(&bucket.bucket_id).as_ref());
        delta.new_violations += current_delta.new_violations;
        delta.resolved_violations += current_delta.resolved_violations;
        delta.risk_score_delta_total += current_delta.risk_score_delta;
        delta.hotspot_score_delta_total += current_delta.hotspot_score_delta;
    }

    for previous in baseline_buckets.into_values() {
        delta.resolved_violations += previous.active_violation_count;
        delta.hotspot_score_delta_total -= previous.hotspot_score;
        delta.risk_score_delta_total -= previous.risk_score.map(|risk| risk.score).unwrap_or(0.0);
    }

    delta
}

fn build_bucket_delta(
    current: &QualityHotspotBucket,
    previous: Option<&QualityHotspotBucket>,
) -> QualityDeltaSummary {
    let previous_rule_counts = previous
        .map(|bucket| {
            bucket
                .rule_counts
                .iter()
                .map(|entry| (entry.rule_id.as_str(), entry.violations))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let current_rule_counts = current
        .rule_counts
        .iter()
        .map(|entry| (entry.rule_id.as_str(), entry.violations))
        .collect::<HashMap<_, _>>();

    let mut new_violations = 0usize;
    let mut resolved_violations = 0usize;
    let mut regressed_rules = Vec::new();

    for (rule_id, current_count) in &current_rule_counts {
        let previous_count = previous_rule_counts.get(rule_id).copied().unwrap_or(0);
        if *current_count > previous_count {
            new_violations += current_count - previous_count;
            regressed_rules.push((*rule_id).to_string());
        }
    }
    for (rule_id, previous_count) in previous_rule_counts {
        let current_count = current_rule_counts.get(rule_id).copied().unwrap_or(0);
        if previous_count > current_count {
            resolved_violations += previous_count - current_count;
        }
    }
    regressed_rules.sort();

    let previous_risk_score = previous.and_then(|bucket| bucket.risk_score.map(|risk| risk.score));
    let current_risk_score = current.risk_score.map(|risk| risk.score);

    QualityDeltaSummary {
        new_violations,
        resolved_violations,
        risk_score_delta: match (current_risk_score, previous_risk_score) {
            (Some(current_score), Some(previous_score)) => current_score - previous_score,
            (Some(current_score), None) => current_score,
            _ => 0.0,
        },
        hotspot_score_delta: previous
            .map(|bucket| current.hotspot_score - bucket.hotspot_score)
            .unwrap_or(current.hotspot_score),
        new_hotspot: previous.is_none()
            && (current.active_violation_count > 0 || current.hotspot_score > 0.0),
        regressed_rules,
    }
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

fn persist_snapshot_artifacts(root: &Path, snapshot: &QualityProjectSnapshotReport) -> Result<()> {
    fs::create_dir_all(root)
        .with_context(|| format!("failed to create snapshot directory `{}`", root.display()))?;
    write_json(root.join(SNAPSHOT_REPORT_NAME), snapshot)?;
    write_json(
        root.join("violations.by_violation_count.json"),
        &snapshot.rule_violations_by_violation_count,
    )?;
    write_json(
        root.join("violations.by_size_bytes.json"),
        &snapshot.rule_violations_by_size_bytes,
    )?;
    write_json(
        root.join("violations.by_non_empty_lines.json"),
        &snapshot.rule_violations_by_non_empty_lines,
    )?;
    write_json(
        root.join("violations.by_metric_graph_edge_out_count.json"),
        &snapshot.rule_violations_by_metric_graph_edge_out_count,
    )?;
    write_json(
        root.join("violations.by_metric_max_cognitive_complexity.json"),
        &snapshot.rule_violations_by_metric_max_cognitive_complexity,
    )?;
    write_json(
        root.join("violations.by_metric_duplicate_density_bps.json"),
        &snapshot.rule_violations_by_metric_duplicate_density_bps,
    )?;
    write_json(root.join("hotspots.file.json"), &snapshot.file_hotspots)?;
    write_json(root.join("hotspots.directory.json"), &snapshot.directory_hotspots)?;
    write_json(root.join("hotspots.module.json"), &snapshot.module_hotspots)?;
    fs::write(root.join("notes.md"), snapshot_notes(snapshot))
        .with_context(|| format!("failed to write notes `{}`", root.join("notes.md").display()))?;
    Ok(())
}

fn persist_delta_artifact(
    project_root: &Path,
    output_root: Option<&str>,
    wave_id: Option<&str>,
    delta: &QualityProjectDeltaReport,
) -> Result<Option<PathBuf>> {
    let Some(wave_id) = wave_id else {
        return Ok(None);
    };
    let path = resolve_quality_artifact_root(project_root, output_root)
        .join("quality-waves")
        .join(safe_segment(wave_id))
        .join("delta")
        .join(format!("{}.json", run_stamp()?));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create delta directory `{}`", parent.display()))?;
    }
    write_json(&path, delta)?;
    Ok(Some(path))
}

fn persist_self_baseline_artifacts(
    project_root: &Path,
    snapshot: &QualityProjectSnapshotReport,
) -> Result<PathBuf> {
    let baseline_root = project_root.join("baseline/quality/self");
    fs::create_dir_all(&baseline_root).with_context(|| {
        format!(
            "failed to create self baseline directory `{}`",
            baseline_root.display()
        )
    })?;
    let baseline_summary = baseline_root.join(BASELINE_SUMMARY_NAME);
    write_json(&baseline_summary, snapshot)?;
    write_json(baseline_root.join("file-hotspots.json"), &snapshot.file_hotspots)?;
    write_json(
        baseline_root.join("directory-hotspots.json"),
        &snapshot.directory_hotspots,
    )?;
    write_json(baseline_root.join("module-hotspots.json"), &snapshot.module_hotspots)?;
    Ok(baseline_summary)
}

fn load_self_baseline_snapshot(project_root: &Path) -> Result<QualityProjectSnapshotReport> {
    load_snapshot_from_path(project_root.join("baseline/quality/self").join(BASELINE_SUMMARY_NAME))
}

fn load_latest_wave_before_snapshot(
    project_root: &Path,
    wave_id: &str,
    output_root: Option<&str>,
) -> Result<QualityProjectSnapshotReport> {
    let before_root = resolve_quality_artifact_root(project_root, output_root)
        .join("quality-waves")
        .join(safe_segment(wave_id))
        .join("before");
    let latest = latest_snapshot_path(&before_root)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no before snapshot found for wave `{wave_id}` under `{}`",
            before_root.display()
        )
    })?;
    load_snapshot_from_path(latest)
}

fn load_snapshot_from_path(path: PathBuf) -> Result<QualityProjectSnapshotReport> {
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read snapshot `{}`", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse snapshot `{}`", path.display()))
}

fn latest_snapshot_path(root: &Path) -> Result<Option<PathBuf>> {
    if !root.exists() {
        return Ok(None);
    }
    let mut entries = fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().ok().is_some_and(|kind| kind.is_dir()))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries
        .pop()
        .map(|entry| entry.path().join(SNAPSHOT_REPORT_NAME))
        .filter(|path| path.exists()))
}

fn snapshot_output_root(
    project_root: &Path,
    output_root: Option<&str>,
    snapshot_kind: QualityProjectSnapshotKind,
    wave_id: Option<&str>,
) -> Result<Option<PathBuf>> {
    let stamp = run_stamp()?;
    let artifact_root = resolve_quality_artifact_root(project_root, output_root);
    let root = match (snapshot_kind, wave_id) {
        (QualityProjectSnapshotKind::Before, Some(wave)) => artifact_root
            .join("quality-waves")
            .join(safe_segment(wave))
            .join("before")
            .join(stamp),
        (QualityProjectSnapshotKind::After, Some(wave)) => artifact_root
            .join("quality-waves")
            .join(safe_segment(wave))
            .join("after")
            .join(stamp),
        (kind, _) => artifact_root
            .join("quality-snapshots")
            .join(snapshot_kind_label(kind))
            .join(stamp),
    };
    Ok(Some(root))
}

fn resolve_quality_artifact_root(project_root: &Path, output_root: Option<&str>) -> PathBuf {
    match output_root.map(str::trim).filter(|value| !value.is_empty()) {
        Some(path) => {
            let candidate = Path::new(path);
            if candidate.is_absolute() {
                candidate.to_path_buf()
            } else {
                project_root.join(candidate)
            }
        }
        None => project_root.join(".codex"),
    }
}

fn snapshot_exclude_paths(project_root: &Path, output_root: Option<&str>) -> Vec<String> {
    let mut excludes = Vec::new();
    if let Some(path) = output_root {
        let resolved = resolve_quality_artifact_root(project_root, Some(path));
        if let Ok(relative) = resolved.strip_prefix(project_root) {
            let normalized = relative.to_string_lossy().replace('\\', "/");
            if !normalized.is_empty() && normalized != "." {
                excludes.push(normalized);
            }
        }
    }
    excludes
}

fn snapshot_kind_label(kind: QualityProjectSnapshotKind) -> &'static str {
    match kind {
        QualityProjectSnapshotKind::AdHoc => "ad_hoc",
        QualityProjectSnapshotKind::Before => "before",
        QualityProjectSnapshotKind::After => "after",
        QualityProjectSnapshotKind::Baseline => "baseline",
    }
}

fn snapshot_notes(snapshot: &QualityProjectSnapshotReport) -> String {
    format!(
        concat!(
            "snapshot_kind={}\n",
            "generated_at_utc={}\n",
            "quality_status_before_refresh={}\n",
            "quality_status_after_refresh={}\n",
            "refresh_performed={}\n",
            "evaluated_files={}\n",
            "violating_files={}\n",
            "total_violations={}\n",
            "suppressed_violations={}\n",
            "total_non_empty_lines={}\n",
            "total_size_bytes={}\n"
        ),
        snapshot_kind_label(snapshot.snapshot_kind),
        snapshot.generated_at_utc,
        snapshot.quality_status_before_refresh.as_str(),
        snapshot.quality_status_after_refresh.as_str(),
        snapshot.refresh_performed,
        snapshot.evaluated_files,
        snapshot.violating_files,
        snapshot.total_violations,
        snapshot.suppressed_violations,
        snapshot.total_non_empty_lines,
        snapshot.total_size_bytes
    )
}

fn violation_fingerprint(
    path: &str,
    violation: &crate::model::QualityViolationEntry,
) -> String {
    let location = violation
        .location
        .as_ref()
        .map(|entry| {
            format!(
                "{}:{}:{}:{}",
                entry.start_line, entry.start_column, entry.end_line, entry.end_column
            )
        })
        .unwrap_or_default();
    let source = violation.source.map(|entry| entry.as_str()).unwrap_or("");
    format!(
        "{path}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{location}\u{001f}{source}",
        violation.rule_id,
        violation.actual_value,
        violation.threshold_value,
        violation.message,
        violation.severity.as_str(),
        violation.category.as_str(),
        violation.location.is_some()
    )
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

fn write_json(path: impl AsRef<Path>, value: &impl serde::Serialize) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory `{}`", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{serialized}\n"))
        .with_context(|| format!("failed to write json file `{}`", path.display()))?;
    Ok(())
}

fn safe_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

fn run_stamp() -> Result<String> {
    Ok(
        OffsetDateTime::now_utc().format(&time::macros::format_description!(
            "[year][month][day]T[hour][minute][second]Z"
        ))?,
    )
}

fn now_rfc3339() -> Result<String> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}
