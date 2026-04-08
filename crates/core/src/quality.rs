use crate::model::{
    FindingConfidence, QualityLocation, QualityMode, QualitySource, QualityViolationEntry,
    SuppressedQualityViolationEntry,
};

#[path = "quality/complexity.rs"]
mod complexity;
#[path = "quality/dead_code.rs"]
mod dead_code;
#[path = "quality/duplication.rs"]
mod duplication;
#[path = "quality/evaluate.rs"]
mod evaluate;
#[path = "quality/git_risk.rs"]
mod git_risk;
#[path = "quality/layering.rs"]
mod layering;
#[path = "quality/location.rs"]
mod location;
#[path = "quality/metrics.rs"]
mod metrics;
#[path = "quality/policy.rs"]
mod policy;
#[path = "quality/policy_schema.rs"]
mod policy_schema;
#[path = "quality/rule_metadata.rs"]
mod rule_metadata;
#[path = "quality/rules/mod.rs"]
mod rules;
#[path = "quality/scoring.rs"]
mod scoring;
#[path = "quality/security_smells.rs"]
mod security_smells;
#[path = "quality/test_risk.rs"]
mod test_risk;

pub(crate) const QUALITY_RULESET_ID: &str = "quality-core-v13";
pub(crate) const CURRENT_QUALITY_RULESET_VERSION: i64 = 13;

#[derive(Debug, Clone)]
pub(crate) struct QualityMetricEntry {
    pub(crate) metric_id: String,
    pub(crate) metric_value: i64,
    pub(crate) location: Option<QualityLocation>,
    pub(crate) source: Option<QualitySource>,
}

#[derive(Debug, Clone)]
pub(crate) struct ObservedMetric {
    pub(crate) metric_value: i64,
    pub(crate) location: Option<QualityLocation>,
    pub(crate) source: QualitySource,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HotspotFacts {
    pub(crate) max_function_lines: Option<ObservedMetric>,
    pub(crate) max_nesting_depth: Option<ObservedMetric>,
    pub(crate) max_parameters_per_function: Option<ObservedMetric>,
    pub(crate) max_export_count_per_file: Option<ObservedMetric>,
    pub(crate) max_class_member_count: Option<ObservedMetric>,
    pub(crate) max_todo_count_per_file: Option<ObservedMetric>,
    pub(crate) max_cyclomatic_complexity: Option<ObservedMetric>,
    pub(crate) max_cognitive_complexity: Option<ObservedMetric>,
    pub(crate) max_branch_count: Option<ObservedMetric>,
    pub(crate) max_early_return_count: Option<ObservedMetric>,
}

impl HotspotFacts {
    pub(crate) fn merge_from(&mut self, other: Self) {
        merge_observed_metric(&mut self.max_function_lines, other.max_function_lines);
        merge_observed_metric(&mut self.max_nesting_depth, other.max_nesting_depth);
        merge_observed_metric(
            &mut self.max_parameters_per_function,
            other.max_parameters_per_function,
        );
        merge_observed_metric(
            &mut self.max_export_count_per_file,
            other.max_export_count_per_file,
        );
        merge_observed_metric(
            &mut self.max_class_member_count,
            other.max_class_member_count,
        );
        merge_observed_metric(
            &mut self.max_todo_count_per_file,
            other.max_todo_count_per_file,
        );
        merge_observed_metric(
            &mut self.max_cyclomatic_complexity,
            other.max_cyclomatic_complexity,
        );
        merge_observed_metric(
            &mut self.max_cognitive_complexity,
            other.max_cognitive_complexity,
        );
        merge_observed_metric(&mut self.max_branch_count, other.max_branch_count);
        merge_observed_metric(
            &mut self.max_early_return_count,
            other.max_early_return_count,
        );
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StructuralFacts {
    pub(crate) fan_in_count: Option<i64>,
    pub(crate) fan_out_count: Option<i64>,
    pub(crate) cycle_member: bool,
    pub(crate) orphan_module: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LayeringFacts {
    pub(crate) zone_id: Option<String>,
    pub(crate) forbidden_edge_count: i64,
    pub(crate) out_of_direction_edge_count: i64,
    pub(crate) unmatched_edge_count: i64,
    pub(crate) primary_message: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct GitRiskFacts {
    pub(crate) recent_commit_count: i64,
    pub(crate) recent_author_count: i64,
    pub(crate) recent_churn_lines: i64,
    pub(crate) primary_author_share_bps: i64,
    pub(crate) cochange_neighbor_count: i64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TestRiskFacts {
    pub(crate) nearby_test_file_count: i64,
    pub(crate) nearby_integration_test_file_count: i64,
    pub(crate) has_public_surface: bool,
    pub(crate) is_hotspot_candidate: bool,
    pub(crate) is_integration_entry: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DuplicationFacts {
    pub(crate) duplicate_block_count: i64,
    pub(crate) duplicate_peer_count: i64,
    pub(crate) duplicate_lines: i64,
    pub(crate) max_duplicate_block_tokens: i64,
    pub(crate) max_duplicate_similarity_percent: i64,
    pub(crate) duplicate_density_bps: i64,
    pub(crate) primary_location: Option<crate::model::QualityLocation>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DeadCodeFacts {
    pub(crate) exported_symbol_count: i64,
    pub(crate) candidate: bool,
    pub(crate) location: Option<crate::model::QualityLocation>,
    pub(crate) confidence: Option<FindingConfidence>,
    pub(crate) noise_reason: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SecuritySmellMatch {
    pub(crate) match_count: i64,
    pub(crate) location: Option<crate::model::QualityLocation>,
    pub(crate) confidence: Option<FindingConfidence>,
    pub(crate) noise_reason: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SecuritySmellFacts {
    pub(crate) shell_exec: SecuritySmellMatch,
    pub(crate) path_traversal: SecuritySmellMatch,
    pub(crate) raw_sql: SecuritySmellMatch,
    pub(crate) unsafe_deserialize: SecuritySmellMatch,
}

#[derive(Debug, Clone)]
pub(crate) struct QualitySnapshot {
    pub(crate) size_bytes: i64,
    pub(crate) total_lines: Option<i64>,
    pub(crate) non_empty_lines: Option<i64>,
    pub(crate) import_count: Option<i64>,
    pub(crate) quality_mode: QualityMode,
    pub(crate) metrics: Vec<QualityMetricEntry>,
    pub(crate) violations: Vec<QualityViolationEntry>,
    pub(crate) suppressed_violations: Vec<SuppressedQualityViolationEntry>,
}

#[derive(Debug, Clone)]
pub(crate) struct QualityEvaluation {
    pub(crate) snapshot: QualitySnapshot,
    pub(crate) had_rule_errors: bool,
    pub(crate) last_error_rule_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct QualityCandidateFacts {
    pub(crate) rel_path: String,
    pub(crate) size_bytes: i64,
    pub(crate) total_lines: Option<i64>,
    pub(crate) non_empty_lines: Option<i64>,
    pub(crate) import_count: Option<i64>,
    pub(crate) max_line_length: Option<i64>,
    pub(crate) import_region: Option<crate::model::QualityLocation>,
    pub(crate) max_line_length_location: Option<crate::model::QualityLocation>,
    pub(crate) quality_mode: QualityMode,
    pub(crate) file_kind: metrics::FileKind,
    pub(crate) hotspots: HotspotFacts,
    pub(crate) structural: StructuralFacts,
    pub(crate) layering: LayeringFacts,
    pub(crate) git_risk: GitRiskFacts,
    pub(crate) test_risk: TestRiskFacts,
    pub(crate) duplication: DuplicationFacts,
    pub(crate) dead_code: DeadCodeFacts,
    pub(crate) security_smells: SecuritySmellFacts,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct IndexedQualityMetrics {
    pub(crate) symbol_count: Option<i64>,
    pub(crate) ref_count: Option<i64>,
    pub(crate) module_dep_count: Option<i64>,
    pub(crate) graph_edge_out_count: Option<i64>,
}

pub(crate) use duplication::{
    DuplicationCandidate, analyze_duplication, write_duplication_artifact,
};
pub(crate) use evaluate::{
    build_indexed_quality_facts, build_oversize_quality_facts, evaluate_quality,
};
pub(crate) use git_risk::load_git_risk_facts;
pub(crate) use metrics::quality_metrics_hash;
pub(crate) use policy::{
    EffectiveQualityPolicy, GitRiskPolicy, QualityPolicy, QualityThresholds, StructuralPolicy,
    TestRiskPolicy, default_quality_policy, load_quality_policy, load_quality_policy_digest,
};
pub(crate) use policy_schema::StructuralUnmatchedBehavior;
pub(crate) use rule_metadata::is_known_rule_id;
pub(crate) use scoring::compute_hit_risk_score;
pub(crate) use test_risk::load_test_risk_facts;

pub(crate) fn violations_hash(violations: &[QualityViolationEntry]) -> String {
    let mut bytes = Vec::new();
    for violation in violations {
        bytes.extend_from_slice(violation.rule_id.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(violation.actual_value.to_string().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(violation.threshold_value.to_string().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(violation.message.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(violation.severity.as_str().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(violation.category.as_str().as_bytes());
        bytes.push(0);
        if let Some(location) = &violation.location {
            bytes.extend_from_slice(location.start_line.to_string().as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(location.start_column.to_string().as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(location.end_line.to_string().as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(location.end_column.to_string().as_bytes());
        }
        bytes.push(0);
        if let Some(source) = violation.source {
            bytes.extend_from_slice(source.as_str().as_bytes());
        }
        bytes.push(0);
        if let Some(finding_family) = violation.finding_family {
            bytes.extend_from_slice(finding_family.as_str().as_bytes());
        }
        bytes.push(0);
        if let Some(confidence) = violation.confidence {
            bytes.extend_from_slice(confidence.as_str().as_bytes());
        }
        bytes.push(0);
        bytes.extend_from_slice(if violation.manual_review_required {
            b"1"
        } else {
            b"0"
        });
        bytes.push(0);
        if let Some(noise_reason) = &violation.noise_reason {
            bytes.extend_from_slice(noise_reason.as_bytes());
        }
        bytes.push(0);
        for followup in &violation.recommended_followups {
            bytes.extend_from_slice(followup.as_bytes());
            bytes.push(0xff);
        }
        bytes.push(b'\n');
    }
    crate::utils::hash_bytes(&bytes)
}

pub(crate) fn suppressed_violations_hash(violations: &[SuppressedQualityViolationEntry]) -> String {
    let mut bytes = Vec::new();
    for violation in violations {
        bytes.extend_from_slice(
            violations_hash(std::slice::from_ref(&violation.violation)).as_bytes(),
        );
        bytes.push(0);
        for suppression in &violation.suppressions {
            bytes.extend_from_slice(suppression.suppression_id.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(suppression.reason.as_bytes());
            bytes.push(0);
            if let Some(scope_id) = &suppression.scope_id {
                bytes.extend_from_slice(scope_id.as_bytes());
            }
            bytes.push(0xff);
        }
        bytes.push(b'\n');
    }
    crate::utils::hash_bytes(&bytes)
}

fn merge_observed_metric(slot: &mut Option<ObservedMetric>, candidate: Option<ObservedMetric>) {
    match (slot.as_ref(), candidate) {
        (_, None) => {}
        (Some(current), Some(candidate)) if current.metric_value >= candidate.metric_value => {}
        (_, Some(candidate)) => *slot = Some(candidate),
    }
}

#[cfg(test)]
#[path = "quality/tests/mod.rs"]
mod tests;
