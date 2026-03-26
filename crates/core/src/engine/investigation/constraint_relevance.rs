use std::collections::HashSet;

use crate::model::ConceptSeedKind;
use crate::model::ConstraintEvidence;

const MAX_QUERY_CONSTRAINTS_PER_VARIANT: usize = 20;

#[derive(Clone, Copy)]
struct ConstraintQueryRelevance {
    combined_overlap: f32,
    path_overlap: f32,
    content_overlap: f32,
    content_matches: usize,
}

pub(super) fn retain_relevant_constraints(
    seed: &str,
    seed_kind: ConceptSeedKind,
    constraints: Vec<ConstraintEvidence>,
) -> Vec<ConstraintEvidence> {
    if !matches!(seed_kind, ConceptSeedKind::Query) {
        return constraints;
    }

    let mut scored = constraints
        .into_iter()
        .map(|constraint| {
            let relevance = constraint_query_relevance(seed, &constraint);
            (constraint, relevance)
        })
        .collect::<Vec<_>>();
    let seed_token_count = tokenize(seed).len();
    let has_positive_content_overlap = scored
        .iter()
        .any(|(_, relevance)| relevance.content_matches > 0);
    let has_positive_overlap = scored
        .iter()
        .any(|(_, relevance)| relevance.combined_overlap > 0.0);
    if has_positive_content_overlap {
        scored.retain(|(constraint, relevance)| {
            constraint_matches_query(seed_token_count, constraint, *relevance)
        });
    } else if has_positive_overlap {
        scored.retain(|(constraint, relevance)| {
            relevance.combined_overlap >= minimum_constraint_overlap(constraint)
        });
    } else {
        scored.clear();
    }
    scored.sort_by(|left, right| {
        if has_positive_content_overlap {
            right
                .1
                .content_matches
                .cmp(&left.1.content_matches)
                .then_with(|| right.1.content_overlap.total_cmp(&left.1.content_overlap))
                .then_with(|| right.1.path_overlap.total_cmp(&left.1.path_overlap))
                .then_with(|| right.1.combined_overlap.total_cmp(&left.1.combined_overlap))
                .then_with(|| right.0.confidence.total_cmp(&left.0.confidence))
                .then_with(|| left.0.path.cmp(&right.0.path))
                .then_with(|| left.0.line_start.cmp(&right.0.line_start))
        } else {
            right
                .1
                .combined_overlap
                .total_cmp(&left.1.combined_overlap)
                .then_with(|| right.0.confidence.total_cmp(&left.0.confidence))
                .then_with(|| left.0.path.cmp(&right.0.path))
                .then_with(|| left.0.line_start.cmp(&right.0.line_start))
        }
    });

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (constraint, _) in scored {
        if seen.insert((
            constraint.path.clone(),
            constraint.line_start,
            constraint.normalized_key.clone(),
        )) {
            out.push(constraint);
        }
        if out.len() >= MAX_QUERY_CONSTRAINTS_PER_VARIANT {
            break;
        }
    }
    out
}

fn constraint_query_relevance(
    seed: &str,
    constraint: &ConstraintEvidence,
) -> ConstraintQueryRelevance {
    let content_haystack = format!(
        "{} {} {}",
        constraint.excerpt, constraint.normalized_text, constraint.normalized_key
    );
    let combined_haystack = format!("{} {}", constraint.path, content_haystack);
    ConstraintQueryRelevance {
        combined_overlap: token_overlap(seed, &combined_haystack),
        path_overlap: token_overlap(seed, &constraint.path),
        content_overlap: token_overlap(seed, &content_haystack),
        content_matches: token_match_count(seed, &content_haystack),
    }
}

fn constraint_matches_query(
    seed_token_count: usize,
    constraint: &ConstraintEvidence,
    relevance: ConstraintQueryRelevance,
) -> bool {
    relevance.content_matches >= minimum_constraint_content_matches(seed_token_count)
        && relevance.content_overlap >= minimum_constraint_content_overlap(constraint)
}

fn minimum_constraint_overlap(constraint: &ConstraintEvidence) -> f32 {
    if constraint.strength == "strong" {
        0.12
    } else {
        0.16
    }
}

fn minimum_constraint_content_overlap(constraint: &ConstraintEvidence) -> f32 {
    if constraint.strength == "strong" {
        0.10
    } else {
        0.14
    }
}

fn minimum_constraint_content_matches(seed_token_count: usize) -> usize {
    if seed_token_count >= 4 { 2 } else { 1 }
}

fn token_overlap(left: &str, right: &str) -> f32 {
    let left_tokens = tokenize(left);
    let right_tokens = tokenize(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }
    let overlap = left_tokens
        .iter()
        .filter(|token| right_tokens.contains(*token))
        .count();
    (overlap as f32 / left_tokens.len().max(right_tokens.len()) as f32).clamp(0.0, 1.0)
}

fn token_match_count(left: &str, right: &str) -> usize {
    let left_tokens = tokenize(left);
    let right_tokens = tokenize(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0;
    }
    left_tokens
        .iter()
        .filter(|token| right_tokens.contains(*token))
        .count()
}

fn tokenize(value: &str) -> HashSet<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .collect()
}
