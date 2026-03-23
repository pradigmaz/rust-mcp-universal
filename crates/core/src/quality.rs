use crate::model::{QualityMode, QualityViolationEntry};

#[path = "quality/evaluate.rs"]
mod evaluate;
#[path = "quality/location.rs"]
mod location;
#[path = "quality/metrics.rs"]
mod metrics;
#[path = "quality/policy.rs"]
mod policy;
#[path = "quality/rules.rs"]
mod rules;

pub(crate) const QUALITY_RULESET_ID: &str = "quality-core-v2";
pub(crate) const CURRENT_QUALITY_RULESET_VERSION: i64 = 2;

#[derive(Debug, Clone)]
pub(crate) struct QualityMetricEntry {
    pub(crate) metric_id: String,
    pub(crate) metric_value: i64,
    pub(crate) location: Option<crate::model::QualityLocation>,
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
}

#[derive(Debug, Clone)]
pub(crate) struct QualityEvaluation {
    pub(crate) snapshot: QualitySnapshot,
    pub(crate) had_rule_errors: bool,
    pub(crate) last_error_rule_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct QualityCandidateFacts {
    pub(crate) size_bytes: i64,
    pub(crate) total_lines: Option<i64>,
    pub(crate) non_empty_lines: Option<i64>,
    pub(crate) import_count: Option<i64>,
    pub(crate) max_line_length: Option<i64>,
    pub(crate) import_region: Option<crate::model::QualityLocation>,
    pub(crate) max_line_length_location: Option<crate::model::QualityLocation>,
    pub(crate) quality_mode: QualityMode,
    pub(crate) file_kind: metrics::FileKind,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct IndexedQualityMetrics {
    pub(crate) symbol_count: Option<i64>,
    pub(crate) ref_count: Option<i64>,
    pub(crate) module_dep_count: Option<i64>,
    pub(crate) graph_edge_out_count: Option<i64>,
}

pub(crate) use evaluate::{
    build_indexed_quality_facts, build_oversize_quality_facts, evaluate_quality,
};
pub(crate) use metrics::quality_metrics_hash;
pub(crate) use policy::{
    QualityPolicy, QualityThresholds, default_quality_policy, load_quality_policy,
};

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
        if let Some(location) = &violation.location {
            bytes.extend_from_slice(location.start_line.to_string().as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(location.start_column.to_string().as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(location.end_line.to_string().as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(location.end_column.to_string().as_bytes());
        }
        bytes.push(b'\n');
    }
    crate::utils::hash_bytes(&bytes)
}

#[cfg(test)]
#[path = "quality/tests.rs"]
mod tests;
