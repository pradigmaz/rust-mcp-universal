use std::collections::HashSet;
use std::path::Path;

use crate::model::SearchHit;
use crate::search_db::extract_tokens;

use super::types::FusedExplainMeta;

pub(super) fn lexical_anchor_paths(query: &str, lexical_pool: &[SearchHit]) -> HashSet<String> {
    let tokens = extract_tokens(query);
    let compact_query = compact_alnum(query);
    lexical_pool
        .iter()
        .filter(|hit| is_exact_path_anchor(&hit.path, &tokens, &compact_query))
        .map(|hit| hit.path.clone())
        .collect()
}

pub(super) fn retain_lexical_anchors(
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
