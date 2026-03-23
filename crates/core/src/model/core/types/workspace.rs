use serde::{Deserialize, Serialize};

use super::index::IndexStatus;
use super::quality::WorkspaceQualitySummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLanguageStat {
    pub language: String,
    pub files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTopSymbol {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceBrief {
    pub auto_indexed: bool,
    pub index_status: IndexStatus,
    pub languages: Vec<WorkspaceLanguageStat>,
    pub top_symbols: Vec<WorkspaceTopSymbol>,
    pub quality_summary: WorkspaceQualitySummary,
    pub recommendations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_hint: Option<WorkspaceRepairHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRepairHint {
    pub action: String,
    pub reason: String,
    pub message: String,
}
