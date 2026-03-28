use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;
use rmu_core::{QualityHotspotsResult, RuleViolationsResult, WorkspaceBrief};
use serde::Deserialize;
use serde_json::Value;

use super::hotspots;
use super::manifest;
use super::report;

#[allow(clippy::too_many_arguments)]
pub(super) fn build_repo_report(
    repo: &manifest::ResolvedQualityMatrixRepository,
    brief_before_refresh: &WorkspaceBrief,
    brief_after_refresh: &WorkspaceBrief,
    by_violation_count: &RuleViolationsResult,
    by_size_bytes: &RuleViolationsResult,
    by_non_empty_lines: &RuleViolationsResult,
    by_metric: &RuleViolationsResult,
    by_complexity: &RuleViolationsResult,
    by_duplication: &RuleViolationsResult,
    file_hotspots: &QualityHotspotsResult,
    directory_hotspots: &QualityHotspotsResult,
    module_hotspots: &QualityHotspotsResult,
    duplication_clone_classes: Option<&Value>,
    validations: report::QualityMatrixValidations,
    index_ms: u128,
    brief_before_refresh_ms: u128,
    refresh_ms: u128,
    brief_after_refresh_ms: u128,
    violation_count_ms: u128,
    size_bytes_ms: u128,
    non_empty_lines_ms: u128,
    metric_ms: u128,
    complexity_ms: u128,
    duplication_ms: u128,
    file_hotspots_ms: u128,
    directory_hotspots_ms: u128,
    module_hotspots_ms: u128,
    db_size_bytes: Option<u64>,
    wal_size_bytes: Option<u64>,
    total_size_bytes: Option<u64>,
) -> Result<report::QualityMatrixRepoReport> {
    let evaluated = brief_after_refresh.quality_summary.evaluated_files;
    let violating = brief_after_refresh.quality_summary.violating_files;
    let total = brief_after_refresh.quality_summary.total_violations;
    let top_rule_share = brief_after_refresh
        .quality_summary
        .top_rules
        .first()
        .map(|rule| rule.violations as f64 / total.max(1) as f64)
        .unwrap_or(0.0);
    let review_shortlist = derive_duplication_review_shortlist(duplication_clone_classes, 5)?;

    Ok(report::QualityMatrixRepoReport {
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
            metric_max_cognitive_complexity: top_paths(by_complexity, 10),
            metric_duplicate_density_bps: top_paths(by_duplication, 10),
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
            metric_max_cognitive_complexity_ms: complexity_ms,
            metric_duplicate_density_bps_ms: duplication_ms,
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
            manual_review_required: !review_shortlist.is_empty(),
            review_shortlist,
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
            violations_by_metric_max_cognitive_complexity:
                "violations.by_metric_max_cognitive_complexity.json".to_string(),
            violations_by_metric_duplicate_density_bps:
                "violations.by_metric_duplicate_density_bps.json".to_string(),
            file_hotspots: "hotspots.file.json".to_string(),
            directory_hotspots: "hotspots.directory.json".to_string(),
            module_hotspots: "hotspots.module.json".to_string(),
            duplication_clone_classes: "duplication.clone_classes.json".to_string(),
            notes_markdown: "notes.md".to_string(),
        },
    })
}

pub(super) fn load_duplication_artifact(project_path: &Path) -> Result<Option<Value>> {
    let artifact_path = project_path.join(".rmu/quality/duplication.clone_classes.json");
    if !artifact_path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&artifact_path)?;
    Ok(Some(serde_json::from_str(&raw)?))
}

fn observed_languages(brief_after_refresh: &WorkspaceBrief) -> Vec<String> {
    brief_after_refresh
        .languages
        .iter()
        .map(|entry| entry.language.to_ascii_lowercase())
        .collect()
}

fn top_paths(result: &RuleViolationsResult, limit: usize) -> Vec<String> {
    result
        .hits
        .iter()
        .take(limit)
        .map(|hit| hit.path.clone())
        .collect()
}

fn derive_duplication_review_shortlist(
    duplication_clone_classes: Option<&Value>,
    limit: usize,
) -> Result<Vec<String>> {
    let Some(artifact) = duplication_clone_classes else {
        return Ok(Vec::new());
    };
    let artifact = serde_json::from_value::<DuplicationArtifactView>(artifact.clone())?;
    let mut clone_classes = artifact
        .clone_classes
        .into_iter()
        .filter(|class| {
            class.cross_file
                && class.corpus_class == "production"
                && class.signal_role.as_deref() != Some("boilerplate")
        })
        .collect::<Vec<_>>();
    clone_classes.sort_by(|left, right| {
        right
            .normalized_token_count
            .cmp(&left.normalized_token_count)
            .then_with(|| right.similarity_percent.cmp(&left.similarity_percent))
            .then_with(|| right.members.len().cmp(&left.members.len()))
            .then_with(|| left.clone_class_id.cmp(&right.clone_class_id))
    });

    let mut seen = BTreeSet::new();
    let mut shortlist = Vec::new();
    for class in clone_classes {
        for member in class.members {
            if seen.insert(member.path.clone()) {
                shortlist.push(member.path);
                if shortlist.len() == limit {
                    return Ok(shortlist);
                }
            }
        }
    }

    Ok(shortlist)
}

#[derive(Debug, Default, Deserialize)]
struct DuplicationArtifactView {
    #[serde(default)]
    clone_classes: Vec<DuplicationCloneClassView>,
}

#[derive(Debug, Deserialize)]
struct DuplicationCloneClassView {
    clone_class_id: String,
    corpus_class: String,
    normalized_token_count: usize,
    similarity_percent: i64,
    cross_file: bool,
    #[serde(default)]
    signal_role: Option<String>,
    #[serde(default)]
    members: Vec<DuplicationCloneMemberView>,
}

#[derive(Debug, Deserialize)]
struct DuplicationCloneMemberView {
    path: String,
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
