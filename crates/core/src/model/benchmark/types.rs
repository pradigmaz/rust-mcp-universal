use serde::{Deserialize, Serialize};

use super::super::core::{PrivacyMode, SemanticFailMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryQrel {
    pub path: String,
    #[serde(default = "super::serde::default_qrel_relevance")]
    pub relevance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryBenchmarkCase {
    pub query: String,
    #[serde(default)]
    pub qrels: Vec<QueryQrel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryBenchmarkDataset {
    #[serde(default)]
    pub queries: Vec<QueryBenchmarkCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryBenchmarkReport {
    pub dataset_path: String,
    pub k: usize,
    pub query_count: usize,
    pub recall_at_k: f32,
    pub mrr_at_k: f32,
    pub ndcg_at_k: f32,
    pub avg_estimated_tokens: f32,
    pub latency_p50_ms: f32,
    pub latency_p95_ms: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QueryBenchmarkOptions {
    pub k: usize,
    pub limit: usize,
    pub semantic: bool,
    #[serde(default)]
    pub semantic_fail_mode: SemanticFailMode,
    #[serde(default)]
    pub privacy_mode: PrivacyMode,
    pub max_chars: usize,
    pub max_tokens: usize,
    #[serde(default = "super::serde::default_query_benchmark_auto_index")]
    pub auto_index: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QueryBenchmarkComparisonOptions {
    pub baseline: QueryBenchmarkOptions,
    pub candidate: QueryBenchmarkOptions,
    #[serde(default = "super::serde::default_query_benchmark_runs")]
    pub runs: usize,
    #[serde(default)]
    pub gate_thresholds: QueryBenchmarkGateThresholds,
    #[serde(default = "super::serde::default_query_benchmark_fail_fast")]
    pub fail_fast: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QueryBenchmarkGateThresholds {
    #[serde(default = "super::serde::default_quality_max_drop_ratio")]
    pub quality_max_drop_ratio: f32,
    #[serde(default = "super::serde::default_latency_p50_max_increase_ratio")]
    pub latency_p50_max_increase_ratio: f32,
    #[serde(default = "super::serde::default_latency_p95_max_increase_ratio")]
    pub latency_p95_max_increase_ratio: f32,
    #[serde(default = "super::serde::default_latency_p95_max_increase_ms")]
    pub latency_p95_max_increase_ms: f32,
    #[serde(default = "super::serde::default_token_cost_max_increase_ratio")]
    pub token_cost_max_increase_ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryBenchmarkMultiRunReport {
    pub runs: Vec<QueryBenchmarkReport>,
    pub median: QueryBenchmarkReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryBenchmarkMetricDiff {
    pub baseline: f32,
    pub candidate: f32,
    pub delta: f32,
    pub delta_ratio: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryBenchmarkDiffReport {
    pub recall_at_k: QueryBenchmarkMetricDiff,
    pub mrr_at_k: QueryBenchmarkMetricDiff,
    pub ndcg_at_k: QueryBenchmarkMetricDiff,
    pub avg_estimated_tokens: QueryBenchmarkMetricDiff,
    pub latency_p50_ms: QueryBenchmarkMetricDiff,
    pub latency_p95_ms: QueryBenchmarkMetricDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryBenchmarkGateMetricResult {
    pub metric: String,
    pub threshold_kind: String,
    pub threshold: f32,
    pub baseline: f32,
    pub candidate: f32,
    pub pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryBenchmarkGateCategoryResult {
    pub category: String,
    pub pass: bool,
    #[serde(default)]
    pub skipped: bool,
    #[serde(default)]
    pub metrics: Vec<QueryBenchmarkGateMetricResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryBenchmarkGateEvaluation {
    pub fail_fast: bool,
    pub overall_pass: bool,
    pub first_failed_category: Option<String>,
    pub categories: Vec<QueryBenchmarkGateCategoryResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryBenchmarkComparisonReport {
    pub dataset_path: String,
    pub runs_count: usize,
    pub median_rule: String,
    pub baseline: QueryBenchmarkMultiRunReport,
    pub candidate: QueryBenchmarkMultiRunReport,
    pub diff: QueryBenchmarkDiffReport,
    pub gates: QueryBenchmarkGateEvaluation,
}
