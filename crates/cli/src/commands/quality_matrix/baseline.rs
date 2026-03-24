use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use rmu_core::{
    QualityDeltaSummary, QualityHotspotAggregation, QualityHotspotBucket, QualityHotspotsResult,
};
use serde::{Deserialize, Serialize};

use super::artifacts;

const HOTSPOTS_BASELINE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct QualityHotspotsBaselineArtifact {
    pub(super) version: u32,
    pub(super) repo_id: String,
    pub(super) aggregation: QualityHotspotAggregation,
    pub(super) buckets: Vec<QualityHotspotBucket>,
}

pub(super) fn apply_baseline_deltas(
    result: &mut QualityHotspotsResult,
    baseline: Option<QualityHotspotsBaselineArtifact>,
) {
    let mut baseline_buckets = baseline
        .map(|artifact| {
            artifact
                .buckets
                .into_iter()
                .map(|bucket| (bucket.bucket_id.clone(), bucket))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    let mut new_violations_total = 0usize;
    let mut resolved_violations_total = 0usize;
    let mut risk_score_delta_total = 0.0f64;
    let mut hotspot_score_delta_total = 0.0f64;

    for bucket in &mut result.buckets {
        let previous = baseline_buckets.remove(&bucket.bucket_id);
        bucket.delta = build_bucket_delta(bucket, previous.as_ref());
        new_violations_total += bucket.delta.new_violations;
        resolved_violations_total += bucket.delta.resolved_violations;
        risk_score_delta_total += bucket.delta.risk_score_delta;
        hotspot_score_delta_total += bucket.delta.hotspot_score_delta;
    }

    for previous in baseline_buckets.into_values() {
        resolved_violations_total += previous.active_violation_count;
        hotspot_score_delta_total -= previous.hotspot_score;
        risk_score_delta_total -= previous.risk_score.map(|risk| risk.score).unwrap_or(0.0);
    }

    result.summary.new_violations = new_violations_total;
    result.summary.resolved_violations = resolved_violations_total;
    result.summary.risk_score_delta_total = risk_score_delta_total;
    result.summary.hotspot_score_delta_total = hotspot_score_delta_total;
}

pub(super) fn load_baseline_artifact(
    project_root: &Path,
    repo_id: &str,
    aggregation: QualityHotspotAggregation,
) -> Result<Option<QualityHotspotsBaselineArtifact>> {
    let artifact_path = baseline_artifact_path(project_root, repo_id, aggregation);
    if !artifact_path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&artifact_path)?;
    Ok(Some(serde_json::from_str(&raw)?))
}

pub(super) fn write_baseline_artifact(
    project_root: &Path,
    repo_id: &str,
    aggregation: QualityHotspotAggregation,
    result: &QualityHotspotsResult,
) -> Result<()> {
    let artifact = QualityHotspotsBaselineArtifact {
        version: HOTSPOTS_BASELINE_VERSION,
        repo_id: repo_id.to_string(),
        aggregation,
        buckets: result
            .buckets
            .iter()
            .cloned()
            .map(|mut bucket| {
                bucket.delta = QualityDeltaSummary::default();
                bucket
            })
            .collect(),
    };
    let path = baseline_artifact_path(project_root, repo_id, aggregation);
    if let Some(parent) = path.parent() {
        artifacts::create_directory(parent)?;
    }
    artifacts::write_json_file(&path, &artifact)
}

pub(super) fn baseline_artifact_path(
    project_root: &Path,
    repo_id: &str,
    aggregation: QualityHotspotAggregation,
) -> PathBuf {
    project_root
        .join("baseline/quality/repos")
        .join(repo_id)
        .join(match aggregation {
            QualityHotspotAggregation::File => "file-hotspots.json",
            QualityHotspotAggregation::Directory => "directory-hotspots.json",
            QualityHotspotAggregation::Module => "module-hotspots.json",
        })
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

#[cfg(test)]
mod tests {
    use rmu_core::model::{
        QualityHotspotBucket, QualityHotspotRuleCount, QualityRiskScoreBreakdown,
        QualityRiskScoreComponents, QualityRiskScoreWeights,
    };

    use super::build_bucket_delta;

    fn risk(score: f64) -> QualityRiskScoreBreakdown {
        QualityRiskScoreBreakdown {
            score,
            components: QualityRiskScoreComponents::default(),
            weights: QualityRiskScoreWeights::default(),
        }
    }

    #[test]
    fn bucket_delta_counts_new_rule_violations_and_regressed_rules() {
        let previous = QualityHotspotBucket {
            bucket_id: "src/lib.rs".to_string(),
            hotspot_score: 1.0,
            active_violation_count: 0,
            suppressed_violation_count: 0,
            risk_score: Some(risk(1.0)),
            rule_counts: Vec::new(),
            structural_signals: Default::default(),
            top_files: vec!["src/lib.rs".to_string()],
            delta: Default::default(),
        };
        let current = QualityHotspotBucket {
            bucket_id: "src/lib.rs".to_string(),
            hotspot_score: 5.0,
            active_violation_count: 2,
            suppressed_violation_count: 0,
            risk_score: Some(risk(5.0)),
            rule_counts: vec![
                QualityHotspotRuleCount {
                    rule_id: "max_non_empty_lines_default".to_string(),
                    violations: 1,
                },
                QualityHotspotRuleCount {
                    rule_id: "max_line_length".to_string(),
                    violations: 1,
                },
            ],
            structural_signals: Default::default(),
            top_files: vec!["src/lib.rs".to_string()],
            delta: Default::default(),
        };

        let delta = build_bucket_delta(&current, Some(&previous));
        assert_eq!(delta.new_violations, 2);
        assert_eq!(delta.resolved_violations, 0);
        assert_eq!(delta.risk_score_delta, 4.0);
        assert_eq!(delta.hotspot_score_delta, 4.0);
        assert_eq!(
            delta.regressed_rules,
            vec![
                "max_line_length".to_string(),
                "max_non_empty_lines_default".to_string()
            ]
        );
    }
}
