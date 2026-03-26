use std::cmp::Ordering;
use std::collections::HashSet;

use crate::engine::investigation::common::CandidateFile;
use crate::engine::investigation::common::classify_route_segment;
use crate::engine::investigation::common::classify_route_source_kind;
use crate::model::{ConceptSeedKind, RoutePath, RouteSegmentKind};

#[derive(Debug, Clone)]
pub(super) struct RankedRoute {
    pub(super) route: RoutePath,
    pub(super) sequence: Vec<String>,
    pub(super) relevance: f32,
}

pub(super) fn compare_ranked_routes(left: &RankedRoute, right: &RankedRoute) -> Ordering {
    right
        .relevance
        .total_cmp(&left.relevance)
        .then_with(|| right.route.confidence.total_cmp(&left.route.confidence))
        .then_with(|| right.route.segments.len().cmp(&left.route.segments.len()))
        .then_with(|| right.route.total_weight.total_cmp(&left.route.total_weight))
        .then_with(|| left.sequence.cmp(&right.sequence))
}

pub(super) fn route_relevance(
    seed: &str,
    seed_kind: ConceptSeedKind,
    start: &CandidateFile,
    route: Option<&RoutePath>,
) -> f32 {
    if !matches!(seed_kind, ConceptSeedKind::Query) {
        return start.score.clamp(0.0, 1.0);
    }

    let start_overlap = query_text_overlap(
        seed,
        &format!(
            "{} {}",
            start.path,
            start.symbol.as_deref().unwrap_or_default()
        ),
    );
    let route_overlap = route
        .map(|route| {
            route
                .segments
                .iter()
                .map(|segment| {
                    query_text_overlap(
                        seed,
                        &format!(
                            "{} {} {}",
                            segment.path,
                            segment.anchor_symbol.as_deref().unwrap_or_default(),
                            segment.source_kind
                        ),
                    )
                })
                .fold(start_overlap, f32::max)
        })
        .unwrap_or(start_overlap);

    (route_overlap * 0.7 + start.score.clamp(0.0, 1.0) * 0.3).clamp(0.0, 1.0)
}

pub(super) fn prioritize_start_candidates(
    mut starts: Vec<CandidateFile>,
    seed_kind: ConceptSeedKind,
) -> Vec<CandidateFile> {
    starts.sort_by(|left, right| {
        if matches!(seed_kind, ConceptSeedKind::Query) {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| start_priority(right).cmp(&start_priority(left)))
                .then_with(|| left.path.cmp(&right.path))
        } else {
            start_priority(right)
                .cmp(&start_priority(left))
                .then_with(|| right.score.total_cmp(&left.score))
                .then_with(|| left.path.cmp(&right.path))
        }
    });
    starts
}

fn query_text_overlap(seed: &str, text: &str) -> f32 {
    let left_tokens = tokenize(seed);
    let right_tokens = tokenize(text);
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
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .collect()
}

fn start_priority(candidate: &CandidateFile) -> usize {
    let kind = classify_route_segment(&candidate.path);
    let source_kind = classify_route_source_kind(&candidate.path);
    match kind {
        RouteSegmentKind::Endpoint => 6,
        RouteSegmentKind::Ui => 5,
        RouteSegmentKind::Service => 4,
        RouteSegmentKind::Crud => 3,
        RouteSegmentKind::ApiClient => 3,
        RouteSegmentKind::Query => 2,
        RouteSegmentKind::Test | RouteSegmentKind::Migration => 1,
        RouteSegmentKind::Unknown => {
            if matches!(source_kind, "validator") {
                4
            } else if matches!(source_kind, "model") {
                2
            } else {
                0
            }
        }
    }
}
