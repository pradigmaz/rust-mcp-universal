use serde::{Deserialize, Serialize};

use super::investigation::{ConceptSeed, InvestigationAnchor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractTraceRole {
    SchemaOrModel,
    Endpoint,
    Service,
    GeneratedClient,
    Consumer,
    Test,
    Migration,
    Validator,
    Adapter,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedLineageStatus {
    NotGenerated,
    Generated,
    SuspectedGenerated,
    GeneratedUnknownSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedLineageBasis {
    PathConvention,
    FileBanner,
    ContentMarker,
    AdjacentContract,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedSourceOfTruthKind {
    Schema,
    ApiSpec,
    GeneratorInput,
    UpstreamContract,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedLineage {
    pub status: GeneratedLineageStatus,
    pub detection_basis: GeneratedLineageBasis,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_of_truth_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_of_truth_kind: Option<GeneratedSourceOfTruthKind>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionabilityStep {
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actionability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_target_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_target_role: Option<ContractTraceRole>,
    pub reason: String,
    #[serde(default)]
    pub next_steps: Vec<ActionabilityStep>,
    #[serde(default)]
    pub related_tests: Vec<String>,
    #[serde(default)]
    pub adjacent_paths: Vec<String>,
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub rollback_sensitive_paths: Vec<String>,
    #[serde(default)]
    pub manual_review_required: bool,
}

impl Default for Actionability {
    fn default() -> Self {
        Self {
            recommended_target_path: None,
            recommended_target_role: None,
            reason: "insufficient_actionable_evidence".to_string(),
            next_steps: Vec::new(),
            related_tests: Vec::new(),
            adjacent_paths: Vec::new(),
            checks: Vec::new(),
            rollback_sensitive_paths: Vec::new(),
            manual_review_required: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractTraceLink {
    pub role: ContractTraceRole,
    pub anchor: InvestigationAnchor,
    pub source_kind: String,
    pub evidence: String,
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_lineage: Option<GeneratedLineage>,
    pub rank_score: f32,
    pub rank_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractBreak {
    pub expected_role: ContractTraceRole,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_resolved_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractTraceResult {
    pub seed: ConceptSeed,
    pub chain: Vec<ContractTraceLink>,
    pub contract_breaks: Vec<ContractBreak>,
    pub actionability: Actionability,
    pub manual_review_required: bool,
    pub capability_status: String,
    pub unsupported_sources: Vec<String>,
    pub confidence: f32,
}
