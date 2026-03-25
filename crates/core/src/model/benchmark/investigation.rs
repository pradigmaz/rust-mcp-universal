use serde::{Deserialize, Serialize};

use crate::model::ConceptSeedKind;

use super::investigation_diff::InvestigationBenchmarkDiffReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvestigationBenchmarkTool {
    SymbolBody,
    RouteTrace,
    ConstraintEvidence,
    ConceptCluster,
    DivergenceReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationAssertion {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InvestigationAnchorLabel {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InvestigationRouteSegmentLabel {
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InvestigationConstraintLabel {
    #[serde(skip_serializing_if = "Option::is_none", alias = "kind")]
    pub constraint_kind: Option<String>,
    #[serde(alias = "source_path")]
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strength: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InvestigationDivergenceSignalLabel {
    pub axis: String,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_strength: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InvestigationCaseLabels {
    #[serde(default)]
    pub expected_body_anchors: Vec<InvestigationAnchorLabel>,
    #[serde(default)]
    pub expected_variant_entry_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_top_variant_entry_path: Option<String>,
    #[serde(default)]
    pub expected_route_segments: Vec<InvestigationRouteSegmentLabel>,
    #[serde(default)]
    pub expected_constraints: Vec<InvestigationConstraintLabel>,
    #[serde(default)]
    pub expected_divergence_signals: Vec<InvestigationDivergenceSignalLabel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_semantic_state: Option<String>,
    #[serde(default)]
    pub low_signal_semantic_case: bool,
    #[serde(default)]
    pub semantic_fail_open_case: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationBenchmarkCase {
    pub id: String,
    pub tool: InvestigationBenchmarkTool,
    pub fixture: String,
    pub seed: String,
    pub seed_kind: ConceptSeedKind,
    #[serde(default, alias = "expected_min_assertions")]
    pub expected_assertions: Vec<InvestigationAssertion>,
    pub expected_capability_status: String,
    #[serde(default)]
    pub labels: InvestigationCaseLabels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationBenchmarkDataset {
    #[serde(default)]
    pub cases: Vec<InvestigationBenchmarkCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationCaseReport {
    pub id: String,
    pub tool: InvestigationBenchmarkTool,
    pub fixture: String,
    pub pass: bool,
    pub assertion_pass_count: usize,
    pub assertion_total_count: usize,
    pub capability_status: String,
    pub expected_capability_status: String,
    #[serde(default)]
    pub unsupported_sources: Vec<String>,
    #[serde(default)]
    pub privacy_failures: usize,
    pub latency_ms: f32,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub returned_anchor_count: usize,
    #[serde(default)]
    pub expected_body_anchor_count: usize,
    #[serde(default)]
    pub matched_anchor_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_success_at_1: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_success_at_3: Option<bool>,
    #[serde(default)]
    pub matched_route_segment_count: usize,
    #[serde(default)]
    pub correctly_typed_route_segment_count: usize,
    #[serde(default)]
    pub returned_constraint_count: usize,
    #[serde(default)]
    pub matched_constraint_count: usize,
    #[serde(default)]
    pub expected_constraint_source_count: usize,
    #[serde(default)]
    pub recovered_constraint_source_count: usize,
    #[serde(default)]
    pub expected_variant_count: usize,
    #[serde(default)]
    pub recovered_variant_count_at_3: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_variant_match: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_rank_consistent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_state_present: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_state_matches_expectation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_fail_open_visible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low_signal_semantic_false_penalty: Option<bool>,
    #[serde(default)]
    pub returned_divergence_signal_count: usize,
    #[serde(default)]
    pub expected_divergence_signal_count: usize,
    #[serde(default)]
    pub matched_divergence_signal_count: usize,
    #[serde(default)]
    pub unexpected_divergence_signal_count: usize,
    #[serde(default)]
    pub evidence_fields_present: usize,
    #[serde(default)]
    pub evidence_fields_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationToolMetrics {
    pub tool: InvestigationBenchmarkTool,
    pub case_count: usize,
    pub passed_cases: usize,
    pub pass_rate: f32,
    pub unsupported_case_rate: f32,
    pub latency_p50_ms: f32,
    pub latency_p95_ms: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_anchor_precision: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_request_p95_budget_ms: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_request_p95_ratio: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_trace_success_at_1: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_trace_success_at_3: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_type_precision: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraint_evidence_precision: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraint_source_recall: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_recall_at_3: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_variant_precision: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_rank_consistency: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_state_coverage: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_fail_open_visibility: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low_signal_semantic_false_penalty_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divergence_signal_precision: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub false_positive_divergence_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain_evidence_coverage: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationThresholds {
    #[serde(default = "default_symbol_body_supported_success")]
    pub symbol_body_supported_success: f32,
    #[serde(default = "default_route_trace_case_pass_rate")]
    pub route_trace_case_pass_rate: f32,
    #[serde(default = "default_constraint_evidence_case_pass_rate")]
    pub constraint_evidence_case_pass_rate: f32,
    #[serde(default = "default_concept_cluster_case_pass_rate")]
    pub concept_cluster_case_pass_rate: f32,
    #[serde(default = "default_divergence_case_pass_rate")]
    pub divergence_case_pass_rate: f32,
    #[serde(default = "default_max_latency_p95_ms")]
    pub max_latency_p95_ms: f32,
    #[serde(default = "default_max_unsupported_case_rate")]
    pub max_unsupported_case_rate: f32,
    #[serde(default)]
    pub privacy_failures: usize,
    #[serde(default)]
    pub body_anchor_precision_min: Option<f32>,
    #[serde(default)]
    pub body_request_p95_ratio_max: Option<f32>,
    #[serde(default)]
    pub route_trace_success_at_1_min: Option<f32>,
    #[serde(default)]
    pub route_trace_success_at_3_min: Option<f32>,
    #[serde(default)]
    pub segment_type_precision_min: Option<f32>,
    #[serde(default)]
    pub constraint_evidence_precision_min: Option<f32>,
    #[serde(default)]
    pub constraint_source_recall_min: Option<f32>,
    #[serde(default)]
    pub variant_recall_at_3_min: Option<f32>,
    #[serde(default)]
    pub top_variant_precision_min: Option<f32>,
    #[serde(default)]
    pub variant_rank_consistency_min: Option<f32>,
    #[serde(default)]
    pub semantic_state_coverage_min: Option<f32>,
    #[serde(default)]
    pub semantic_fail_open_visibility_min: Option<f32>,
    #[serde(default)]
    pub low_signal_semantic_false_penalty_rate_max: Option<f32>,
    #[serde(default)]
    pub divergence_signal_precision_min: Option<f32>,
    #[serde(default)]
    pub false_positive_divergence_rate_max: Option<f32>,
    #[serde(default)]
    pub explain_evidence_coverage_min: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationThresholdVerdict {
    pub passed: bool,
    #[serde(default)]
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationBenchmarkReport {
    pub dataset_path: String,
    pub limit: usize,
    pub case_count: usize,
    pub per_tool_metrics: Vec<InvestigationToolMetrics>,
    pub cases: Vec<InvestigationCaseReport>,
    #[serde(default)]
    pub unsupported_behavior_summary: Vec<String>,
    #[serde(default)]
    pub privacy_failures: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold_verdict: Option<InvestigationThresholdVerdict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub navigation_latency_baseline_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<InvestigationBenchmarkDiffReport>,
}

const fn default_symbol_body_supported_success() -> f32 {
    0.90
}

const fn default_route_trace_case_pass_rate() -> f32 {
    0.80
}

const fn default_constraint_evidence_case_pass_rate() -> f32 {
    0.85
}

const fn default_concept_cluster_case_pass_rate() -> f32 {
    0.85
}

const fn default_divergence_case_pass_rate() -> f32 {
    0.85
}

const fn default_max_latency_p95_ms() -> f32 {
    250.0
}

const fn default_max_unsupported_case_rate() -> f32 {
    0.25
}
