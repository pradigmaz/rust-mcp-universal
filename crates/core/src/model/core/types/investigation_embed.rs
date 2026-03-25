use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationTopVariant {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationConceptClusterSummary {
    pub variant_count: usize,
    pub top_variants: Vec<InvestigationTopVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationRouteSummary {
    pub best_route_segment_count: usize,
    pub alternate_route_count: usize,
    pub unresolved_gap_count: usize,
    pub segment_kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationConstraintSummary {
    pub total: usize,
    pub strong: usize,
    pub weak: usize,
    pub constraint_kinds: Vec<String>,
    pub normalized_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationDivergenceSummary {
    pub surface_kind: String,
    pub authoritative_tool: String,
    pub preview_only: bool,
    pub highest_severity: String,
    pub signal_count: usize,
    pub recommended_followups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationSummary {
    pub surface_kind: String,
    pub concept_cluster: InvestigationConceptClusterSummary,
    pub route_trace: InvestigationRouteSummary,
    pub constraint_evidence: InvestigationConstraintSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divergence: Option<InvestigationDivergenceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationHints {
    pub top_variants: Vec<InvestigationTopVariant>,
    pub route_summary: InvestigationRouteSummary,
    pub constraint_keys: Vec<String>,
    pub followups: Vec<String>,
}
