use serde::{Deserialize, Serialize};

use super::{ConceptSeed, ConstraintEvidence, InvestigationAnchor, RouteSegment};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SemanticState {
    Used,
    DisabledLowSignal,
    UnavailableFailOpen,
    #[default]
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VariantScoreBreakdown {
    #[serde(default)]
    pub lexical: f32,
    #[serde(default)]
    pub semantic: f32,
    #[serde(default)]
    pub route: f32,
    #[serde(default)]
    pub symbol: f32,
    #[serde(default)]
    pub constraint: f32,
    #[serde(default)]
    pub test: f32,
    #[serde(default)]
    pub penalties: f32,
    #[serde(default, rename = "final")]
    pub final_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationVariant {
    pub id: String,
    pub entry_anchor: InvestigationAnchor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_anchor: Option<InvestigationAnchor>,
    pub route: Vec<RouteSegment>,
    pub constraints: Vec<ConstraintEvidence>,
    pub related_tests: Vec<String>,
    #[serde(default)]
    pub lexical_proximity: f32,
    #[serde(default)]
    pub semantic_proximity: f32,
    #[serde(default)]
    pub route_centrality: f32,
    #[serde(default)]
    pub symbol_overlap: f32,
    #[serde(default)]
    pub constraint_overlap: f32,
    #[serde(default)]
    pub test_adjacency: f32,
    #[serde(default)]
    pub semantic_state: SemanticState,
    #[serde(default = "default_score_model")]
    pub score_model: String,
    #[serde(default)]
    pub score_breakdown: VariantScoreBreakdown,
    pub confidence: f32,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptClusterSummary {
    pub variant_count: usize,
    pub languages: Vec<String>,
    pub route_kinds: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expansion_sources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expansion_policy: Option<ConceptClusterExpansionPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cutoff_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedup_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConceptClusterExpansionPolicy {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initial_sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enrichment_sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feedback_sources: Vec<String>,
    #[serde(default)]
    pub route_trace_reused: bool,
    #[serde(default)]
    pub candidate_pool_limit_multiplier: usize,
    pub dedup_unit: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tie_break_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptClusterResult {
    pub seed: ConceptSeed,
    pub variants: Vec<ImplementationVariant>,
    pub cluster_summary: ConceptClusterSummary,
    pub gaps: Vec<String>,
    pub capability_status: String,
    pub unsupported_sources: Vec<String>,
    pub confidence: f32,
}

fn default_score_model() -> String {
    "heuristic_v2".to_string()
}
