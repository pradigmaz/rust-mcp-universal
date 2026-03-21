use std::collections::HashMap;

use crate::model::{ContextMode, SearchHit};

use super::super::chunking::ChunkPoolCandidate;
use super::super::graph_stage::GraphPoolCandidate;
use super::super::semantic_candidates::SemanticFileCandidate;
use super::super::support::FusionProfile;

#[derive(Debug, Clone)]
pub(crate) struct FusedExplainMeta {
    pub(crate) semantic_score: f32,
    pub(crate) graph_score: f32,
    pub(crate) rrf_score: f32,
    pub(crate) graph_rrf: f32,
    pub(crate) semantic_source: String,
    pub(crate) graph_seed_path: String,
    pub(crate) graph_edge_kinds: Vec<String>,
    pub(crate) graph_hops: usize,
    pub(crate) rank_before: usize,
    pub(crate) rank_after: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct FusionResult {
    pub(crate) hits: Vec<SearchHit>,
    pub(crate) explain_by_path: HashMap<String, FusedExplainMeta>,
}

pub(crate) struct FusionInputs<'a> {
    pub(crate) query: &'a str,
    pub(crate) lexical_pool: &'a [SearchHit],
    pub(crate) file_pool: &'a [SemanticFileCandidate],
    pub(crate) chunk_pool: &'a [ChunkPoolCandidate],
    pub(crate) graph_pool: &'a [GraphPoolCandidate],
    pub(crate) profile: FusionProfile,
    pub(crate) context_mode: Option<ContextMode>,
    pub(crate) candidate_limit: usize,
}

#[derive(Debug, Clone)]
pub(super) struct CandidateState {
    pub(super) path: String,
    pub(super) preview: String,
    pub(super) size_bytes: i64,
    pub(super) language: String,
    pub(super) lexical_rank: Option<usize>,
    pub(super) file_rank: Option<usize>,
    pub(super) chunk_rank: Option<usize>,
    pub(super) graph_rank: Option<usize>,
    pub(super) lexical_score: f32,
    pub(super) file_score: f32,
    pub(super) chunk_score: f32,
    pub(super) graph_score: f32,
    pub(super) semantic_indexed: bool,
    pub(super) semantic_fallback: bool,
    pub(super) graph_seed_path: String,
    pub(super) graph_edge_kinds: Vec<String>,
    pub(super) graph_hops: usize,
}
