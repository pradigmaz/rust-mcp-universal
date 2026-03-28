use std::path::Path;

use anyhow::Result;
use rmu_core::model::WorkspaceQualityTopMetric;
use rmu_core::{
    PrivacyMode, WorkspaceQualityTopRule, sanitize_path_text, sanitize_value_for_privacy,
};
use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct QualityMatrixAggregateReport {
    pub(crate) manifest_version: u32,
    pub(crate) generated_at_utc: String,
    pub(crate) run_root: String,
    pub(crate) canonical_summary_path: String,
    pub(crate) repos: Vec<QualityMatrixRepoReport>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct QualityMatrixRepoReport {
    pub(crate) repo_id: String,
    pub(crate) role: String,
    pub(crate) profile: String,
    pub(crate) size_class: String,
    pub(crate) expected_languages: Vec<String>,
    pub(crate) observed_languages: Vec<String>,
    pub(crate) pre_refresh_status: String,
    pub(crate) post_refresh_status: String,
    pub(crate) top_rules: Vec<WorkspaceQualityTopRule>,
    pub(crate) top_metrics: Vec<WorkspaceQualityTopMetric>,
    pub(crate) top_hot_files: QualityMatrixTopHotFiles,
    pub(crate) top_hotspot_buckets: QualityMatrixTopHotspotBuckets,
    pub(crate) latency_summary: QualityMatrixLatencySummary,
    pub(crate) resource_summary: QualityMatrixResourceSummary,
    pub(crate) noise_summary: QualityMatrixNoiseSummary,
    pub(crate) new_violations: usize,
    pub(crate) resolved_violations: usize,
    pub(crate) risk_score_delta_total: f64,
    pub(crate) hotspot_score_delta_total: f64,
    pub(crate) validations: QualityMatrixValidations,
    pub(crate) artifacts: QualityMatrixArtifactPaths,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct QualityMatrixTopHotFiles {
    pub(crate) violation_count: Vec<String>,
    pub(crate) size_bytes: Vec<String>,
    pub(crate) non_empty_lines: Vec<String>,
    pub(crate) metric_graph_edge_out_count: Vec<String>,
    pub(crate) metric_max_cognitive_complexity: Vec<String>,
    pub(crate) metric_duplicate_density_bps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct QualityMatrixTopHotspotBuckets {
    pub(crate) file: Vec<String>,
    pub(crate) directory: Vec<String>,
    pub(crate) module: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct QualityMatrixLatencySummary {
    pub(crate) index_ms: u128,
    pub(crate) brief_before_refresh_ms: u128,
    pub(crate) refresh_ms: u128,
    pub(crate) brief_after_refresh_ms: u128,
    pub(crate) violation_count_ms: u128,
    pub(crate) size_bytes_ms: u128,
    pub(crate) non_empty_lines_ms: u128,
    pub(crate) metric_graph_edge_out_count_ms: u128,
    pub(crate) metric_max_cognitive_complexity_ms: u128,
    pub(crate) metric_duplicate_density_bps_ms: u128,
    pub(crate) file_hotspots_ms: u128,
    pub(crate) directory_hotspots_ms: u128,
    pub(crate) module_hotspots_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct QualityMatrixResourceSummary {
    pub(crate) db_size_bytes: Option<u64>,
    pub(crate) wal_size_bytes: Option<u64>,
    pub(crate) total_size_bytes: Option<u64>,
    pub(crate) peak_rss_bytes: Option<u64>,
    pub(crate) peak_rss_note: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct QualityMatrixNoiseSummary {
    pub(crate) violating_file_ratio: f64,
    pub(crate) top_rule_share: f64,
    pub(crate) manual_review_required: bool,
    pub(crate) review_shortlist: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct QualityMatrixValidations {
    pub(crate) languages_match: bool,
    pub(crate) pre_status_match: bool,
    pub(crate) post_status_match: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct QualityMatrixArtifactPaths {
    pub(crate) brief_before_refresh: String,
    pub(crate) brief_after_refresh: String,
    pub(crate) violations_by_violation_count: String,
    pub(crate) violations_by_size_bytes: String,
    pub(crate) violations_by_non_empty_lines: String,
    pub(crate) violations_by_metric_graph_edge_out_count: String,
    pub(crate) violations_by_metric_max_cognitive_complexity: String,
    pub(crate) violations_by_metric_duplicate_density_bps: String,
    pub(crate) file_hotspots: String,
    pub(crate) directory_hotspots: String,
    pub(crate) module_hotspots: String,
    pub(crate) duplication_clone_classes: String,
    pub(crate) notes_markdown: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CanonicalBaselineSummary {
    pub(crate) manifest_version: u32,
    pub(crate) generated_at_utc: String,
    pub(crate) repos: Vec<CanonicalBaselineRepoSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CanonicalBaselineRepoSummary {
    pub(crate) repo_id: String,
    pub(crate) profile: String,
    pub(crate) pre_refresh_status: String,
    pub(crate) post_refresh_status: String,
    pub(crate) evaluated_files: usize,
    pub(crate) violating_files: usize,
    pub(crate) total_violations: usize,
    pub(crate) top_rules: Vec<WorkspaceQualityTopRule>,
    pub(crate) top_metrics: Vec<WorkspaceQualityTopMetric>,
    pub(crate) top_hot_files: QualityMatrixTopHotFiles,
    pub(crate) top_hotspot_buckets: QualityMatrixTopHotspotBuckets,
    pub(crate) latency_summary: QualityMatrixLatencySummary,
    pub(crate) resource_summary: QualityMatrixResourceSummary,
    pub(crate) noise_summary: QualityMatrixNoiseSummary,
    pub(crate) new_violations: usize,
    pub(crate) resolved_violations: usize,
    pub(crate) risk_score_delta_total: f64,
    pub(crate) hotspot_score_delta_total: f64,
}

pub(crate) fn new_aggregate_report(
    manifest_version: u32,
    run_root: &Path,
    canonical_summary_path: &Path,
    repos: Vec<QualityMatrixRepoReport>,
) -> Result<QualityMatrixAggregateReport> {
    Ok(QualityMatrixAggregateReport {
        manifest_version,
        generated_at_utc: now_rfc3339()?,
        run_root: run_root.display().to_string(),
        canonical_summary_path: canonical_summary_path.display().to_string(),
        repos,
    })
}

pub(crate) fn build_canonical_summary(
    aggregate: &QualityMatrixAggregateReport,
    evaluated_files: &[usize],
    violating_files: &[usize],
    total_violations: &[usize],
) -> CanonicalBaselineSummary {
    let repos = aggregate
        .repos
        .iter()
        .zip(evaluated_files.iter())
        .zip(violating_files.iter())
        .zip(total_violations.iter())
        .map(
            |(((repo, &evaluated), &violating), &violations)| CanonicalBaselineRepoSummary {
                repo_id: repo.repo_id.clone(),
                profile: repo.profile.clone(),
                pre_refresh_status: repo.pre_refresh_status.clone(),
                post_refresh_status: repo.post_refresh_status.clone(),
                evaluated_files: evaluated,
                violating_files: violating,
                total_violations: violations,
                top_rules: repo.top_rules.clone(),
                top_metrics: repo.top_metrics.clone(),
                top_hot_files: repo.top_hot_files.clone(),
                top_hotspot_buckets: repo.top_hotspot_buckets.clone(),
                latency_summary: repo.latency_summary.clone(),
                resource_summary: repo.resource_summary.clone(),
                noise_summary: repo.noise_summary.clone(),
                new_violations: repo.new_violations,
                resolved_violations: repo.resolved_violations,
                risk_score_delta_total: repo.risk_score_delta_total,
                hotspot_score_delta_total: repo.hotspot_score_delta_total,
            },
        )
        .collect();
    CanonicalBaselineSummary {
        manifest_version: aggregate.manifest_version,
        generated_at_utc: aggregate.generated_at_utc.clone(),
        repos,
    }
}

pub(crate) fn sanitize_for_output(
    aggregate: &QualityMatrixAggregateReport,
    privacy_mode: PrivacyMode,
) -> Result<serde_json::Value> {
    let mut value = serde_json::to_value(aggregate)?;
    sanitize_value_for_privacy(privacy_mode, &mut value);
    Ok(value)
}

pub(crate) fn text_summary(
    aggregate: &QualityMatrixAggregateReport,
    privacy_mode: PrivacyMode,
) -> String {
    format!(
        "repos={}, run_root={}, canonical_summary={}",
        aggregate.repos.len(),
        sanitize_path_text(privacy_mode, &aggregate.run_root),
        sanitize_path_text(privacy_mode, &aggregate.canonical_summary_path)
    )
}

fn now_rfc3339() -> Result<String> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}
