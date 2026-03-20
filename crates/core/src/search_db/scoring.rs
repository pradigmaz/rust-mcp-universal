use std::cmp::Ordering;
use std::path::Path;

use crate::model::SearchHit;

use super::helpers::first_match_char_offset;
use super::unicode;

pub(super) fn rank_to_score(rank: f64) -> f32 {
    // In FTS5 bm25: lower is better. Convert to stable positive score.
    let m = rank.abs() as f32;
    1.0 / (1.0 + m)
}

pub(super) fn compare_hits_desc(a: &SearchHit, b: &SearchHit) -> Ordering {
    b.score
        .total_cmp(&a.score)
        .then_with(|| a.path.cmp(&b.path))
}

pub(super) fn keep_top_hits(
    best_hits: &mut Vec<SearchHit>,
    candidate: SearchHit,
    keep_limit: usize,
) {
    if keep_limit == 0 {
        return;
    }
    if best_hits.len() < keep_limit {
        best_hits.push(candidate);
        return;
    }

    let mut worst_idx = 0;
    for idx in 1..best_hits.len() {
        if compare_hits_desc(&best_hits[worst_idx], &best_hits[idx]).is_lt() {
            worst_idx = idx;
        }
    }

    if compare_hits_desc(&candidate, &best_hits[worst_idx]).is_lt() {
        best_hits[worst_idx] = candidate;
    }
}

pub(super) fn like_score(query_key: &str, path_key: &str, sample_key: &str) -> f32 {
    if query_key.is_empty() {
        return 0.0;
    }

    let path_hits = path_key.matches(query_key).count() as f32;
    let sample_hits = sample_key.matches(query_key).count() as f32;
    let path_pos = first_match_char_offset(path_key, query_key).unwrap_or(usize::MAX);
    let sample_pos = first_match_char_offset(sample_key, query_key).unwrap_or(usize::MAX);

    let mut score = 0.25;
    score += (path_hits.min(5.0)) * 0.14;
    score += (sample_hits.min(8.0)) * 0.07;
    if path_pos != usize::MAX {
        score += 0.2 / (1.0 + path_pos as f32);
    }
    if sample_pos != usize::MAX {
        score += 0.12 / (1.0 + sample_pos as f32);
    }

    score.min(2.0)
}

pub(super) fn path_match_boost(path: &str, tokens: &[String]) -> f32 {
    if tokens.is_empty() {
        return 0.0;
    }

    let normalized_path = unicode::normalize_match_key(path);
    let normalized_file_name = Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(unicode::normalize_match_key)
        .unwrap_or_else(|| normalized_path.clone());
    let normalized_file_stem = Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(unicode::normalize_match_key)
        .unwrap_or_else(|| normalized_file_name.clone());
    let path_segments = normalized_path
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    let mut score = 0.0_f32;
    for token in tokens {
        let token_key = unicode::normalize_match_key(token);
        if token_key.is_empty() {
            continue;
        }

        if normalized_file_stem == token_key {
            score += 0.85;
            continue;
        }
        if normalized_file_name == token_key {
            score += 0.75;
            continue;
        }
        if path_segments.iter().any(|segment| *segment == token_key) {
            score += 0.32;
            continue;
        }
        if normalized_file_name.contains(&token_key) {
            score += 0.18;
        }
    }

    score.min(1.2)
}
