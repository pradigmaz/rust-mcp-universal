use serde::{Deserialize, Serialize};

use super::super::{ContextMode, PrivacyMode, SemanticFailMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOptions {
    pub query: String,
    pub limit: usize,
    pub detailed: bool,
    pub semantic: bool,
    #[serde(default)]
    pub semantic_fail_mode: SemanticFailMode,
    #[serde(default)]
    pub privacy_mode: PrivacyMode,
    #[serde(default)]
    pub context_mode: Option<ContextMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub path: String,
    pub preview: String,
    pub score: f32,
    pub size_bytes: i64,
    pub language: String,
}
