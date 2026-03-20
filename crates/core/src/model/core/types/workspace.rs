use serde::{Deserialize, Serialize};

use super::index::IndexStatus;

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
    pub recommendations: Vec<String>,
}
