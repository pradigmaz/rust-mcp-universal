use serde::{Deserialize, Serialize};

use super::investigation::InvestigationBenchmarkTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationMetricChange {
    pub tool: InvestigationBenchmarkTool,
    pub metric: String,
    pub expectation: String,
    pub baseline: f32,
    pub current: f32,
    pub delta: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_ratio: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationToolMetricDelta {
    pub tool: InvestigationBenchmarkTool,
    #[serde(default)]
    pub metrics: Vec<InvestigationMetricChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationBenchmarkDiffReport {
    pub baseline_case_count: usize,
    pub current_case_count: usize,
    #[serde(default)]
    pub per_tool_deltas: Vec<InvestigationToolMetricDelta>,
    #[serde(default)]
    pub regressed_metrics: Vec<InvestigationMetricChange>,
    #[serde(default)]
    pub improved_metrics: Vec<InvestigationMetricChange>,
    #[serde(default)]
    pub regression_failures: Vec<String>,
}
