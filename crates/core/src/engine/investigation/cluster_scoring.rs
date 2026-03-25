use std::collections::HashSet;

use crate::model::{RouteSegment, RouteSegmentKind, SemanticState, VariantScoreBreakdown};

use super::common::{CandidateFile, normalized_values};

pub(super) struct ClusterScoringSignals {
    pub(super) lexical_proximity: f32,
    pub(super) semantic_proximity: f32,
    pub(super) route_centrality: f32,
    pub(super) symbol_overlap: f32,
    pub(super) constraint_overlap: f32,
    pub(super) test_adjacency: f32,
    pub(super) confidence: f32,
    pub(super) score_breakdown: VariantScoreBreakdown,
}

pub(super) fn compute_scoring_signals(
    seed: &str,
    candidate: &CandidateFile,
    route: &[RouteSegment],
    strong_constraint_count: usize,
    weak_constraint_count: usize,
    related_tests: &[String],
    semantic_state: SemanticState,
    body_unresolved: bool,
    no_constraint_evidence: bool,
    no_test_evidence: bool,
) -> ClusterScoringSignals {
    let lexical_proximity = lexical_signal(seed, candidate);
    let semantic_proximity = if candidate.source_kind == "semantic_search_candidate" {
        candidate.score.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let route_centrality = route_signal(route);
    let symbol_overlap = symbol_signal(seed, candidate, route);
    let constraint_overlap = ((strong_constraint_count as f32 * 0.5)
        + (weak_constraint_count as f32 * 0.25))
        .clamp(0.0, 1.0);
    let test_adjacency = if related_tests.is_empty() {
        0.0
    } else {
        (0.5 + (related_tests.len() as f32 * 0.2)).clamp(0.0, 1.0)
    };
    let base = (lexical_proximity * 0.20)
        + (route_centrality * 0.20)
        + (symbol_overlap * 0.15)
        + (constraint_overlap * 0.25)
        + (test_adjacency * 0.20);
    let semantic_bonus = if semantic_state == SemanticState::Used {
        semantic_proximity * 0.10
    } else {
        0.0
    };
    let penalties = (if body_unresolved { 0.10 } else { 0.0 })
        + (if no_constraint_evidence { 0.10 } else { 0.0 })
        + (if no_test_evidence { 0.05 } else { 0.0 });
    let confidence = (base + semantic_bonus - penalties).clamp(0.0, 1.0);
    ClusterScoringSignals {
        lexical_proximity,
        semantic_proximity,
        route_centrality,
        symbol_overlap,
        constraint_overlap,
        test_adjacency,
        confidence,
        score_breakdown: VariantScoreBreakdown {
            lexical: lexical_proximity,
            semantic: semantic_proximity,
            route: route_centrality,
            symbol: symbol_overlap,
            constraint: constraint_overlap,
            test: test_adjacency,
            penalties,
            final_score: confidence,
        },
    }
}

fn lexical_signal(seed: &str, candidate: &CandidateFile) -> f32 {
    let haystack = format!(
        "{} {}",
        candidate.path,
        candidate.symbol.clone().unwrap_or_default()
    );
    token_overlap(seed, &haystack).max(if candidate.source_kind == "search_candidate" {
        candidate.score.clamp(0.0, 1.0)
    } else {
        (candidate.score * 0.85).clamp(0.0, 1.0)
    })
}

fn route_signal(route: &[RouteSegment]) -> f32 {
    if route.is_empty() {
        return 0.0;
    }
    let weights = route
        .iter()
        .map(|segment| match segment.kind {
            RouteSegmentKind::Endpoint | RouteSegmentKind::Service | RouteSegmentKind::Crud => 1.0,
            RouteSegmentKind::Query | RouteSegmentKind::ApiClient | RouteSegmentKind::Ui => 0.8,
            RouteSegmentKind::Test | RouteSegmentKind::Migration => 0.55,
            RouteSegmentKind::Unknown => 0.2,
        })
        .collect::<Vec<_>>();
    (weights.iter().sum::<f32>() / weights.len() as f32).clamp(0.0, 1.0)
}

fn symbol_signal(seed: &str, candidate: &CandidateFile, route: &[RouteSegment]) -> f32 {
    if let Some(symbol) = &candidate.symbol
        && route
            .iter()
            .any(|segment| segment.anchor_symbol.as_ref() == Some(symbol))
    {
        return 1.0;
    }
    let symbols = normalized_values(
        route
            .iter()
            .filter_map(|segment| segment.anchor_symbol.clone()),
    );
    if symbols.is_empty() {
        return 0.0;
    }
    token_overlap(seed, &symbols.join(" "))
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

fn tokenize(value: &str) -> HashSet<String> {
    value
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .collect()
}
