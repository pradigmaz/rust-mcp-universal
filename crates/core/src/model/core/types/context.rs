use serde::{Deserialize, Serialize};

use super::super::ContextMode;
use super::investigation_embed::InvestigationHints;
use super::report::QuerySurfaceTimings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    pub path: String,
    pub excerpt: String,
    pub score: f32,
    pub chunk_idx: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub chunk_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSelection {
    pub files: Vec<ContextFile>,
    pub total_chars: usize,
    pub estimated_tokens: usize,
    pub truncated: bool,
    #[serde(default)]
    pub chunk_candidates: usize,
    #[serde(default)]
    pub chunk_selected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackResult {
    pub mode: ContextMode,
    pub context: ContextSelection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub investigation_hints: Option<InvestigationHints>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timings: Option<QuerySurfaceTimings>,
}
