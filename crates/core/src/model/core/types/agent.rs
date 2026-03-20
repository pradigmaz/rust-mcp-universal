use serde::{Deserialize, Serialize};

use super::context::ContextSelection;
use super::query::SearchHit;
use super::report::QueryReport;
use super::workspace::WorkspaceBrief;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQueryBundle {
    pub query: String,
    pub limit: usize,
    pub semantic: bool,
    pub max_chars: usize,
    pub max_tokens: usize,
    pub hits: Vec<SearchHit>,
    pub context: ContextSelection,
    pub report: QueryReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBootstrap {
    pub brief: WorkspaceBrief,
    pub query_bundle: Option<AgentQueryBundle>,
}
