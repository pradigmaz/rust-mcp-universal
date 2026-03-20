use std::collections::HashMap;

use anyhow::Result;
use rusqlite::{Connection, params};

#[path = "chunking/pool.rs"]
mod pool;
#[path = "chunking/scoring.rs"]
mod scoring;
#[path = "chunking/selection.rs"]
mod selection;

use pool::build_chunk_pool_candidates;
use selection::{ChunkCandidate, ChunkSelectionParams, select_best_chunk_for_hit};

use crate::model::SearchHit;
use crate::search_db::extract_tokens;
use crate::vector_rank::{embed_for_index, semantic_model_name};

use super::super::context;
use super::vector_utils::{i64_to_usize, is_zero_vector};

const CHUNK_SEMANTIC_WEIGHT: f32 = 0.58;
const CHUNK_LEXICAL_WEIGHT: f32 = 0.42;
const CHUNK_EXCERPT_MAX_CHARS: usize = 180;

#[derive(Debug, Clone)]
pub(super) struct ChunkPoolCandidate {
    pub(super) path: String,
    pub(super) preview: String,
    pub(super) size_bytes: i64,
    pub(super) language: String,
    pub(super) semantic_score: f32,
    pub(super) semantic_indexed: bool,
    pub(super) semantic_fallback: bool,
}

pub(super) fn best_chunks_for_hits(
    conn: &Connection,
    query: &str,
    hits: &[SearchHit],
) -> Result<HashMap<String, context::ChunkExcerpt>> {
    best_chunks_for_hits_with_query_vec(conn, query, None, hits)
}

pub(super) fn best_chunks_for_hits_with_query_vec(
    conn: &Connection,
    query: &str,
    query_vec_prefetched: Option<&[f32]>,
    hits: &[SearchHit],
) -> Result<HashMap<String, context::ChunkExcerpt>> {
    let mut out = HashMap::with_capacity(hits.len());
    if hits.is_empty() {
        return Ok(out);
    }

    let model_name = semantic_model_name();
    let owned_query_vec;
    let query_vec = if let Some(prefetched) = query_vec_prefetched {
        prefetched
    } else {
        owned_query_vec = embed_for_index(query);
        &owned_query_vec
    };
    let has_query_vec = !is_zero_vector(query_vec);
    let query_tokens = extract_tokens(query);
    let query_lc = query.to_lowercase();

    let mut stmt = conn.prepare(
        r#"
        SELECT fc.chunk_idx, fc.start_line, fc.end_line, fc.excerpt, ce.vector_json
        FROM file_chunks fc
        LEFT JOIN chunk_embeddings ce
            ON ce.chunk_hash = fc.chunk_hash
           AND ce.model = ?2
        WHERE fc.path = ?1
        ORDER BY fc.chunk_idx ASC
        "#,
    )?;

    for hit in hits {
        let rows = stmt
            .query_map(params![&hit.path, &model_name], |row| {
                Ok(ChunkCandidate {
                    chunk_idx: i64_to_usize(row.get::<_, i64>(0)?),
                    start_line: row.get::<_, Option<i64>>(1)?.map_or(0, i64_to_usize),
                    end_line: row.get::<_, Option<i64>>(2)?.map_or(0, i64_to_usize),
                    excerpt: row.get::<_, String>(3)?,
                    semantic_vector_json: row.get::<_, Option<String>>(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        if rows.is_empty() {
            continue;
        }
        let selection_params = ChunkSelectionParams {
            query_lc: &query_lc,
            query_tokens: &query_tokens,
            query_vec,
            has_query_vec,
            chunk_semantic_weight: CHUNK_SEMANTIC_WEIGHT,
            chunk_lexical_weight: CHUNK_LEXICAL_WEIGHT,
            excerpt_max_chars: CHUNK_EXCERPT_MAX_CHARS,
        };
        let selected = select_best_chunk_for_hit(hit, rows, &selection_params)?;
        if let Some(selected) = selected {
            out.insert(hit.path.clone(), selected);
        }
    }

    Ok(out)
}

pub(super) fn chunk_pool_for_hits(
    conn: &Connection,
    query: &str,
    query_vec_prefetched: Option<&[f32]>,
    hits: &[SearchHit],
    candidate_limit: usize,
) -> Result<(
    Vec<ChunkPoolCandidate>,
    HashMap<String, context::ChunkExcerpt>,
)> {
    let chunk_map = best_chunks_for_hits_with_query_vec(conn, query, query_vec_prefetched, hits)?;
    if chunk_map.is_empty() {
        return Ok((Vec::new(), chunk_map));
    }

    let candidates = build_chunk_pool_candidates(hits, &chunk_map, candidate_limit);
    Ok((candidates, chunk_map))
}
