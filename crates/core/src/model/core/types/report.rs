use serde::{Deserialize, Serialize};

use super::super::serde_glue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalStage {
    pub stage: String,
    pub candidates: usize,
    pub kept: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetInfo {
    pub max_tokens: usize,
    pub used_estimate: usize,
    pub hard_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedContextItem {
    pub path: String,
    pub score: f32,
    pub chars: usize,
    pub chunk_idx: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub chunk_source: String,
    pub why: Vec<String>,
    pub explain: RankExplainBreakdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankExplainBreakdown {
    pub lexical: f32,
    pub graph: f32,
    pub semantic: f32,
    pub rrf: f32,
    #[serde(default)]
    pub graph_rrf: f32,
    pub rank_before: usize,
    pub rank_after: usize,
    pub semantic_source: String,
    pub semantic_outcome: String,
    #[serde(default)]
    pub graph_seed_path: String,
    #[serde(default)]
    pub graph_edge_kinds: Vec<String>,
    #[serde(default)]
    pub graph_hops: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceInfo {
    pub overall: f32,
    pub reasons: Vec<String>,
    pub signals: ConfidenceSignals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceSignals {
    pub margin_top1_top2: f32,
    pub explain_coverage: f32,
    pub semantic_coverage: f32,
    pub semantic_outcome: String,
    pub stage_drop_ratio: f32,
    pub hard_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryReport {
    pub query_id: String,
    pub timestamp_utc: String,
    pub project_root: String,
    pub budget: BudgetInfo,
    pub retrieval_pipeline: Vec<RetrievalStage>,
    pub selected_context: Vec<SelectedContextItem>,
    pub confidence: ConfidenceInfo,
    pub gaps: Vec<String>,
    pub index_telemetry: IndexTelemetry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexTelemetry {
    pub last_index_lock_wait_ms: u64,
    pub last_embedding_cache_hits: usize,
    pub last_embedding_cache_misses: usize,
    #[serde(default)]
    pub chunk_coverage: f32,
    #[serde(default = "serde_glue::default_chunk_source")]
    pub chunk_source: String,
}
