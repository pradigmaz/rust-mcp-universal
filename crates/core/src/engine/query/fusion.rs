use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;

use crate::model::SearchHit;
use crate::search_db::extract_tokens;

use super::chunking::ChunkPoolCandidate;
use super::graph_stage::GraphPoolCandidate;
use super::semantic_candidates::SemanticFileCandidate;
use super::support::{FusionProfile, path_role_prior};
use crate::model::ContextMode;

const RRF_K: f32 = 60.0;

#[derive(Debug, Clone)]
pub(super) struct FusedExplainMeta {
    pub(super) semantic_score: f32,
    pub(super) graph_score: f32,
    pub(super) rrf_score: f32,
    pub(super) graph_rrf: f32,
    pub(super) semantic_source: String,
    pub(super) graph_seed_path: String,
    pub(super) graph_edge_kinds: Vec<String>,
    pub(super) graph_hops: usize,
    pub(super) rank_before: usize,
    pub(super) rank_after: usize,
}

#[derive(Debug, Clone)]
pub(super) struct FusionResult {
    pub(super) hits: Vec<SearchHit>,
    pub(super) explain_by_path: HashMap<String, FusedExplainMeta>,
}

pub(super) struct FusionInputs<'a> {
    pub(super) query: &'a str,
    pub(super) lexical_pool: &'a [SearchHit],
    pub(super) file_pool: &'a [SemanticFileCandidate],
    pub(super) chunk_pool: &'a [ChunkPoolCandidate],
    pub(super) graph_pool: &'a [GraphPoolCandidate],
    pub(super) profile: FusionProfile,
    pub(super) context_mode: Option<ContextMode>,
    pub(super) candidate_limit: usize,
}

#[derive(Debug, Clone)]
struct CandidateState {
    path: String,
    preview: String,
    size_bytes: i64,
    language: String,
    lexical_rank: Option<usize>,
    file_rank: Option<usize>,
    chunk_rank: Option<usize>,
    graph_rank: Option<usize>,
    lexical_score: f32,
    file_score: f32,
    chunk_score: f32,
    graph_score: f32,
    semantic_indexed: bool,
    semantic_fallback: bool,
    graph_seed_path: String,
    graph_edge_kinds: Vec<String>,
    graph_hops: usize,
}

pub(super) fn fuse_candidate_pools(input: FusionInputs<'_>) -> FusionResult {
    let FusionInputs {
        query,
        lexical_pool,
        file_pool,
        chunk_pool,
        graph_pool,
        profile,
        context_mode,
        candidate_limit,
    } = input;
    let mut states = HashMap::new();
    let lexical_anchor_paths = lexical_anchor_paths(query, lexical_pool);

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

    scored.sort_by(|a, b| {
        b.0.score
            .total_cmp(&a.0.score)
            .then_with(|| a.0.path.cmp(&b.0.path))
    });
    retain_lexical_anchors(&mut scored, &lexical_anchor_paths, candidate_limit.max(1));
    scored.truncate(candidate_limit.max(1));

    let mut hits = Vec::with_capacity(scored.len());
    let mut explain_by_path = HashMap::with_capacity(scored.len());
    for (idx, (hit, mut meta)) in scored.into_iter().enumerate() {
        meta.rank_after = idx + 1;
        explain_by_path.insert(hit.path.clone(), meta);
        hits.push(hit);
    }

    FusionResult {
        hits,
        explain_by_path,
    }
}

fn reciprocal_rank(rank_1based: usize) -> f32 {
    1.0 / (RRF_K + rank_1based as f32)
}

fn lexical_anchor_paths(query: &str, lexical_pool: &[SearchHit]) -> HashSet<String> {
    let tokens = extract_tokens(query);
    let compact_query = compact_alnum(query);
    lexical_pool
        .iter()
        .filter(|hit| is_exact_path_anchor(&hit.path, &tokens, &compact_query))
        .map(|hit| hit.path.clone())
        .collect()
}

fn is_exact_path_anchor(path: &str, tokens: &[String], compact_query: &str) -> bool {
    let file_name = Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(path);
    let file_stem = Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(file_name);
    let compact_name = compact_alnum(file_name);
    let compact_stem = compact_alnum(file_stem);
    if !compact_query.is_empty() && (compact_name == compact_query || compact_stem == compact_query)
    {
        return true;
    }

    let compact_segments = path
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .map(compact_alnum)
        .collect::<Vec<_>>();
    tokens.iter().any(|token| {
        compact_segments.iter().any(|segment| segment == token)
            || compact_name == *token
            || compact_stem == *token
    })
}

fn compact_alnum(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn retain_lexical_anchors(
    scored: &mut Vec<(SearchHit, FusedExplainMeta)>,
    lexical_anchor_paths: &HashSet<String>,
    candidate_limit: usize,
) {
    if lexical_anchor_paths.is_empty() || scored.len() <= candidate_limit {
        return;
    }

    let mut overflow = scored.split_off(candidate_limit);
    for anchor in overflow.drain(..) {
        if !lexical_anchor_paths.contains(&anchor.0.path) {
            continue;
        }
        if scored.iter().any(|(hit, _)| hit.path == anchor.0.path) {
            continue;
        }
        if let Some(replace_idx) = scored
            .iter()
            .rposition(|(hit, _)| !lexical_anchor_paths.contains(&hit.path))
        {
            scored[replace_idx] = anchor;
        }
    }

    scored.sort_by(|a, b| {
        b.0.score
            .total_cmp(&a.0.score)
            .then_with(|| a.0.path.cmp(&b.0.path))
    });
}
