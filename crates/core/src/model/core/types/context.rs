use serde::{Deserialize, Serialize};

use super::super::ContextMode;

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
}
