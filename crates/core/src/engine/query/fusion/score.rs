use std::collections::HashMap;

use crate::model::{ContextMode, SearchHit};

use super::types::{CandidateState, FusedExplainMeta};
use crate::engine::query::intent::SearchIntent;
use crate::engine::query::support::{FusionProfile, path_role_prior};

const RRF_K: f32 = 60.0;

pub(super) fn score_candidates(
    states: HashMap<String, CandidateState>,
    profile: FusionProfile,
    context_mode: Option<ContextMode>,
    search_intent: &SearchIntent,
    lexical_anchor_paths: &std::collections::HashSet<String>,
) -> Vec<(SearchHit, FusedExplainMeta)> {
    let mut scored = Vec::with_capacity(states.len());

    for state in states.into_values() {
        let lexical_rrf = state.lexical_rank.map(reciprocal_rank).unwrap_or(0.0);
        let file_rrf = state.file_rank.map(reciprocal_rank).unwrap_or(0.0);
        let chunk_rrf = state.chunk_rank.map(reciprocal_rank).unwrap_or(0.0);
        let graph_rrf = state.graph_rank.map(reciprocal_rank).unwrap_or(0.0);
        let rrf_score = (profile.lexical_weight * lexical_rrf)
            + (profile.semantic_file_weight * file_rrf)
            + (profile.semantic_chunk_weight * chunk_rrf)
            + (profile.graph_weight * graph_rrf);
        let lexical_anchor_bonus = if lexical_anchor_paths.contains(&state.path) {
            0.035
        } else {
            0.0
        };
        let fused_score = rrf_score
            + (0.020 * state.file_score)
            + (0.028 * state.chunk_score)
            + (0.012 * state.lexical_score)
            + lexical_anchor_bonus
            + search_intent.score_hit(&state.path, &state.preview, &state.language, context_mode)
            + path_role_prior(&state.path, &state.language, context_mode);
        let semantic_score = state.file_score.max(state.chunk_score);
        let semantic_source = match (state.semantic_indexed, state.semantic_fallback) {
            (true, true) => "mixed".to_string(),
            (true, false) => "indexed".to_string(),
            (false, true) => "fallback".to_string(),
            (false, false) => "none".to_string(),
        };
        let rank_before = state
            .lexical_rank
            .or(state.file_rank)
            .or(state.chunk_rank)
            .or(state.graph_rank)
            .unwrap_or(1);

        scored.push((
            SearchHit {
                path: state.path.clone(),
                preview: state.preview,
                score: fused_score.max(0.0),
                size_bytes: state.size_bytes,
                language: state.language,
            },
            FusedExplainMeta {
                semantic_score,
                graph_score: state.graph_score,
                rrf_score: rrf_score.max(0.0),
                graph_rrf: graph_rrf.max(0.0),
                semantic_source,
                graph_seed_path: state.graph_seed_path,
                graph_edge_kinds: state.graph_edge_kinds,
                graph_hops: state.graph_hops,
                rank_before,
                rank_after: 0,
            },
        ));
    }

    scored
}

fn reciprocal_rank(rank_1based: usize) -> f32 {
    1.0 / (RRF_K + rank_1based as f32)
}
