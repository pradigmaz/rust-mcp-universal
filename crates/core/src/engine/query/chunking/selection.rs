use anyhow::{Context, Result};

use crate::model::SearchHit;
use crate::vector_rank::embed_for_index;

use super::super::context;
use super::super::vector_utils::{cosine_similarity, parse_vector_with_dim};
use super::scoring::{chunk_lexical_signal, compact_excerpt_for_budget};

#[derive(Debug, Clone)]
pub(super) struct ChunkCandidate {
    pub(super) chunk_idx: usize,
    pub(super) start_line: usize,
    pub(super) end_line: usize,
    pub(super) excerpt: String,
    pub(super) semantic_vector_json: Option<String>,
}

pub(super) struct ChunkSelectionParams<'a> {
    pub(super) query_lc: &'a str,
    pub(super) query_tokens: &'a [String],
    pub(super) query_vec: &'a [f32],
    pub(super) has_query_vec: bool,
    pub(super) chunk_semantic_weight: f32,
    pub(super) chunk_lexical_weight: f32,
    pub(super) excerpt_max_chars: usize,
}

pub(super) fn select_best_chunk_for_hit(
    hit: &SearchHit,
    rows: Vec<ChunkCandidate>,
    params: &ChunkSelectionParams<'_>,
) -> Result<Option<context::ChunkExcerpt>> {
    let mut best: Option<(f32, f32, f32, context::ChunkExcerpt)> = None;
    for row in rows {
        let raw_excerpt = if row.excerpt.trim().is_empty() {
            hit.preview.clone()
        } else {
            row.excerpt
        };
        let lexical = chunk_lexical_signal(params.query_lc, params.query_tokens, &raw_excerpt);
        let (semantic, source) = if params.has_query_vec {
            if let Some(raw_vector) = row.semantic_vector_json {
                match parse_vector_with_dim(&raw_vector, params.query_vec.len()).with_context(
                    || {
                        format!(
                            "invalid chunk embedding for path `{}` chunk `{}`",
                            hit.path, row.chunk_idx
                        )
                    },
                ) {
                    Ok(chunk_vec) => (
                        cosine_similarity(params.query_vec, &chunk_vec).max(0.0),
                        "chunk_embedding_index".to_string(),
                    ),
                    Err(_) => {
                        let fallback_vec = embed_for_index(&raw_excerpt);
                        (
                            cosine_similarity(params.query_vec, &fallback_vec).max(0.0),
                            "chunk_embedding_fallback".to_string(),
                        )
                    }
                }
            } else {
                let fallback_vec = embed_for_index(&raw_excerpt);
                (
                    cosine_similarity(params.query_vec, &fallback_vec).max(0.0),
                    "chunk_embedding_fallback".to_string(),
                )
            }
        } else {
            (0.0, "chunk_lexical_only".to_string())
        };
        let combined =
            (params.chunk_semantic_weight * semantic) + (params.chunk_lexical_weight * lexical);
        let candidate = context::ChunkExcerpt {
            excerpt: compact_excerpt_for_budget(
                &raw_excerpt,
                params.query_lc,
                params.query_tokens,
                params.excerpt_max_chars,
            ),
            chunk_idx: row.chunk_idx,
            start_line: row.start_line,
            end_line: row.end_line,
            score: combined,
            source,
        };

        let should_replace = match &best {
            None => true,
            Some((best_combined, best_semantic, best_lexical, _)) => {
                combined > (*best_combined + 1e-6)
                    || ((combined - *best_combined).abs() <= 1e-6
                        && (semantic > (*best_semantic + 1e-6)
                            || ((semantic - *best_semantic).abs() <= 1e-6
                                && lexical > (*best_lexical + 1e-6))))
            }
        };
        if should_replace {
            best = Some((combined, semantic, lexical, candidate));
        }
    }
    Ok(best.map(|(_, _, _, selected)| selected))
}
