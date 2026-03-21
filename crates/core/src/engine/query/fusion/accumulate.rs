use std::collections::HashMap;

use crate::model::SearchHit;

use super::types::CandidateState;
use crate::engine::query::chunking::ChunkPoolCandidate;
use crate::engine::query::graph_stage::GraphPoolCandidate;
use crate::engine::query::semantic_candidates::SemanticFileCandidate;

pub(super) fn build_candidate_states(
    lexical_pool: &[SearchHit],
    file_pool: &[SemanticFileCandidate],
    chunk_pool: &[ChunkPoolCandidate],
    graph_pool: &[GraphPoolCandidate],
) -> HashMap<String, CandidateState> {
    let mut states = HashMap::new();

    for (idx, hit) in lexical_pool.iter().enumerate() {
        states.insert(
            hit.path.clone(),
            CandidateState {
                path: hit.path.clone(),
                preview: hit.preview.clone(),
                size_bytes: hit.size_bytes,
                language: hit.language.clone(),
                lexical_rank: Some(idx + 1),
                file_rank: None,
                chunk_rank: None,
                graph_rank: None,
                lexical_score: hit.score.max(0.0),
                file_score: 0.0,
                chunk_score: 0.0,
                graph_score: 0.0,
                semantic_indexed: false,
                semantic_fallback: false,
                graph_seed_path: String::new(),
                graph_edge_kinds: Vec::new(),
                graph_hops: 0,
            },
        );
    }

    for (idx, candidate) in file_pool.iter().enumerate() {
        let entry = states
            .entry(candidate.path.clone())
            .or_insert_with(|| CandidateState {
                path: candidate.path.clone(),
                preview: candidate.preview.clone(),
                size_bytes: candidate.size_bytes,
                language: candidate.language.clone(),
                lexical_rank: None,
                file_rank: None,
                chunk_rank: None,
                graph_rank: None,
                lexical_score: 0.0,
                file_score: 0.0,
                chunk_score: 0.0,
                graph_score: 0.0,
                semantic_indexed: false,
                semantic_fallback: false,
                graph_seed_path: String::new(),
                graph_edge_kinds: Vec::new(),
                graph_hops: 0,
            });
        entry.file_rank = Some(idx + 1);
        entry.file_score = entry.file_score.max(candidate.semantic_score.max(0.0));
        if candidate.semantic_fallback {
            entry.semantic_fallback = true;
        } else {
            entry.semantic_indexed = true;
        }
        if entry.lexical_rank.is_none() {
            entry.preview = candidate.preview.clone();
        }
    }

    for (idx, candidate) in chunk_pool.iter().enumerate() {
        let entry = states
            .entry(candidate.path.clone())
            .or_insert_with(|| CandidateState {
                path: candidate.path.clone(),
                preview: candidate.preview.clone(),
                size_bytes: candidate.size_bytes,
                language: candidate.language.clone(),
                lexical_rank: None,
                file_rank: None,
                chunk_rank: None,
                graph_rank: None,
                lexical_score: 0.0,
                file_score: 0.0,
                chunk_score: 0.0,
                graph_score: 0.0,
                semantic_indexed: false,
                semantic_fallback: false,
                graph_seed_path: String::new(),
                graph_edge_kinds: Vec::new(),
                graph_hops: 0,
            });
        entry.chunk_rank = Some(idx + 1);
        entry.chunk_score = entry.chunk_score.max(candidate.semantic_score.max(0.0));
        entry.semantic_indexed |= candidate.semantic_indexed;
        entry.semantic_fallback |= candidate.semantic_fallback;
        entry.preview = candidate.preview.clone();
    }

    for (idx, candidate) in graph_pool.iter().enumerate() {
        let entry = states
            .entry(candidate.path.clone())
            .or_insert_with(|| CandidateState {
                path: candidate.path.clone(),
                preview: candidate.preview.clone(),
                size_bytes: candidate.size_bytes,
                language: candidate.language.clone(),
                lexical_rank: None,
                file_rank: None,
                chunk_rank: None,
                graph_rank: None,
                lexical_score: 0.0,
                file_score: 0.0,
                chunk_score: 0.0,
                graph_score: 0.0,
                semantic_indexed: false,
                semantic_fallback: false,
                graph_seed_path: String::new(),
                graph_edge_kinds: Vec::new(),
                graph_hops: 0,
            });
        entry.graph_rank = Some(idx + 1);
        if candidate.graph_score > entry.graph_score {
            entry.graph_score = candidate.graph_score.max(0.0);
            entry.graph_seed_path = candidate.seed_path.clone();
            entry.graph_edge_kinds = candidate.edge_kinds.clone();
            entry.graph_hops = candidate.hops;
            if entry.lexical_rank.is_none()
                && entry.file_rank.is_none()
                && entry.chunk_rank.is_none()
            {
                entry.preview = candidate.preview.clone();
            }
        }
    }

    states
}
