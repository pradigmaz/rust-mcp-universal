use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::model::SemanticFailMode;
use crate::vector_rank::semantic_model_name;

use super::vector_utils::{cosine_similarity, is_zero_vector, parse_vector_with_dim, trim_excerpt};

#[path = "semantic_candidates/ann_probing.rs"]
mod ann_probing;
#[path = "semantic_candidates/fallback.rs"]
mod fallback;
#[path = "semantic_candidates/limits.rs"]
mod limits;
#[path = "semantic_candidates/telemetry.rs"]
mod telemetry;

use telemetry::RetrievalTelemetry;

#[derive(Debug, Clone)]
pub(super) struct SemanticFileCandidate {
    pub(super) path: String,
    pub(super) preview: String,
    pub(super) size_bytes: i64,
    pub(super) language: String,
    pub(super) semantic_score: f32,
    pub(super) semantic_fallback: bool,
}

#[derive(Debug, Clone, Default)]
pub(super) struct SemanticFileCandidateBatch {
    pub(super) candidates: Vec<SemanticFileCandidate>,
    pub(super) corrupted_rows: usize,
}

type RawSemanticCandidateRow = (String, String, String, i64, String);

pub(super) fn semantic_file_candidates(
    conn: &Connection,
    query_vec: &[f32],
    candidate_limit: usize,
    probe_factor: f32,
    fail_mode: SemanticFailMode,
) -> Result<SemanticFileCandidateBatch> {
    if candidate_limit == 0 {
        return Ok(SemanticFileCandidateBatch::default());
    }
    if is_zero_vector(query_vec) {
        return Ok(SemanticFileCandidateBatch::default());
    }

    let model_name = semantic_model_name();
    let strict_corruption = fail_mode == SemanticFailMode::FailClosed;
    if let Some(candidates) = ann_probing::semantic_candidates_from_ann(
        conn,
        query_vec,
        &model_name,
        candidate_limit,
        probe_factor,
        strict_corruption,
    )? {
        return Ok(candidates);
    }

    fallback::semantic_candidates_from_full_scan(
        conn,
        query_vec,
        &model_name,
        candidate_limit,
        strict_corruption,
    )
}

fn score_semantic_candidates(
    rows: Vec<RawSemanticCandidateRow>,
    query_vec: &[f32],
    candidate_limit: usize,
    telemetry: RetrievalTelemetry,
    strict_corruption: bool,
) -> Result<SemanticFileCandidateBatch> {
    let mut scored = Vec::with_capacity(rows.len());
    let mut corrupted_rows = 0_usize;
    for (path, raw_vector, sample, size_bytes, language) in rows {
        let vector = match parse_vector_with_dim(&raw_vector, query_vec.len()).with_context(|| {
            format!("invalid semantic vector for semantic candidate path `{path}`")
        }) {
            Ok(vector) => vector,
            Err(err) => {
                if strict_corruption {
                    return Err(err);
                }
                corrupted_rows = corrupted_rows.saturating_add(1);
                continue;
            }
        };
        let semantic_score = cosine_similarity(query_vec, &vector).max(0.0);
        scored.push(SemanticFileCandidate {
            path,
            preview: trim_excerpt(&sample, 260),
            size_bytes,
            language,
            semantic_score,
            semantic_fallback: telemetry.semantic_fallback,
        });
    }

    scored.sort_by(|a, b| {
        b.semantic_score
            .total_cmp(&a.semantic_score)
            .then_with(|| a.path.cmp(&b.path))
    });
    scored.truncate(candidate_limit);
    Ok(SemanticFileCandidateBatch {
        candidates: scored,
        corrupted_rows,
    })
}

#[cfg(test)]
pub(super) fn ann_probe_limit(candidate_limit: usize) -> usize {
    limits::ann_probe_limit(candidate_limit)
}

pub(super) fn ann_probe_limit_with_factor(candidate_limit: usize, probe_factor: f32) -> usize {
    limits::ann_probe_limit_with_factor(candidate_limit, probe_factor)
}

pub(super) fn ann_accept_floor(candidate_limit: usize) -> usize {
    limits::ann_accept_floor(candidate_limit)
}
