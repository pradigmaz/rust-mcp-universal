use serde::{Deserialize, Serialize};

use crate::model::{NodeSummary, ProjectBrief};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedStatus {
    Open,
    Closed,
    Unknown,
}

impl NormalizedStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Unknown => "unknown",
        }
    }
}

pub fn normalize_status(status: &str) -> NormalizedStatus {
    match status.trim().to_ascii_lowercase().as_str() {
        "active" | "open" | "blocked" | "at_risk" => NormalizedStatus::Open,
        "resolved" | "closed" | "done" | "accepted" | "superseded" | "obsolete" => {
            NormalizedStatus::Closed
        }
        _ => NormalizedStatus::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentChangeItem {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub node_type: String,
    pub status: String,
    pub file_path: String,
    pub change_hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionLogEntry {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub status: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskHotspotItem {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub status: String,
    pub normalized_status: NormalizedStatus,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub blocks: Vec<String>,
    #[serde(default)]
    pub affects: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskHotspots {
    #[serde(default)]
    pub risks: Vec<RiskHotspotItem>,
    #[serde(default)]
    pub constraints: Vec<RiskHotspotItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPackNode {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub node_type: String,
    pub summary: String,
    pub why_included: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPackBudget {
    pub max_chars: usize,
    pub max_tokens: usize,
    pub used_chars: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPack {
    pub seed: String,
    pub brief: ProjectBrief,
    #[serde(default)]
    pub included_nodes: Vec<ContextPackNode>,
    #[serde(default)]
    pub recent_changes: Vec<RecentChangeItem>,
    #[serde(default)]
    pub risks: Vec<RiskHotspotItem>,
    pub budget: ContextPackBudget,
}

pub fn mark_node_summary_status(mut node: NodeSummary) -> NodeSummary {
    node.normalized_status = Some(normalize_status(&node.status));
    node
}
