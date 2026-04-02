use serde::{Deserialize, Serialize};

use super::super::{AgentIntentMode, BootstrapProfile, DegradationReason, ModeResolutionSource};
use super::context::ContextSelection;
use super::investigation_embed::InvestigationSummary;
use super::query::SearchHit;
use super::report::{CanonicalProvenance, QueryReport};
use super::workspace::WorkspaceBrief;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct AgentBootstrapIncludeOptions {
    #[serde(default)]
    pub include_report: bool,
    #[serde(default)]
    pub include_investigation_summary: bool,
    #[serde(default)]
    pub profile: Option<BootstrapProfile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AgentBootstrapTimings {
    #[serde(default)]
    pub index_ready_ms: u64,
    #[serde(default)]
    pub brief_ms: u64,
    #[serde(default)]
    pub search_ms: u64,
    #[serde(default)]
    pub context_ms: u64,
    #[serde(default)]
    pub investigation_ms: u64,
    #[serde(default)]
    pub report_ms: u64,
    #[serde(default)]
    pub total_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQueryBundle {
    pub query: String,
    pub limit: usize,
    pub semantic: bool,
    pub resolved_mode: AgentIntentMode,
    pub mode_source: ModeResolutionSource,
    pub max_chars: usize,
    pub max_tokens: usize,
    pub hits: Vec<SearchHit>,
    pub context: ContextSelection,
    pub provenance: CanonicalProvenance,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub followups: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub investigation_summary: Option<InvestigationSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<QueryReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBootstrap {
    pub brief: WorkspaceBrief,
    pub profile: BootstrapProfile,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub degradation_reasons: Vec<DegradationReason>,
    #[serde(default)]
    pub deepen_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deepen_hint: Option<String>,
    pub query_bundle: Option<AgentQueryBundle>,
    #[serde(default)]
    pub timings: AgentBootstrapTimings,
}
