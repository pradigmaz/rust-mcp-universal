use std::collections::HashMap;

use crate::model::SearchHit;

use super::super::context;
use super::ChunkPoolCandidate;

fn semantic_flags_from_source(source: &str) -> (bool, bool) {
    match source {
        "chunk_embedding_index" => (true, false),
        "chunk_embedding_fallback" => (false, true),
        _ => (false, false),
    }
}

pub(super) fn build_chunk_pool_candidates(
    hits: &[SearchHit],
    chunk_map: &HashMap<String, context::ChunkExcerpt>,
    candidate_limit: usize,
) -> Vec<ChunkPoolCandidate> {
    let mut candidates = Vec::with_capacity(chunk_map.len());
    for hit in hits {
        let Some(chunk) = chunk_map.get(&hit.path) else {
            continue;
        };
        let (semantic_indexed, semantic_fallback) = semantic_flags_from_source(&chunk.source);
        candidates.push(ChunkPoolCandidate {
            path: hit.path.clone(),
            preview: chunk.excerpt.clone(),
            size_bytes: hit.size_bytes,
            language: hit.language.clone(),
            semantic_score: chunk.score.max(0.0),
            semantic_indexed,
            semantic_fallback,
        });
    }

    candidates.sort_by(|a, b| {
        b.semantic_score
            .total_cmp(&a.semantic_score)
            .then_with(|| a.path.cmp(&b.path))
    });
    candidates.truncate(candidate_limit.max(1));
    candidates
}
