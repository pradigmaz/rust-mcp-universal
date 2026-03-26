use crate::model::RouteSegment;
use crate::model::RouteSegmentKind;

use super::common::CandidateFile;
use super::common::normalized_values;

const MIN_CONSTRAINT_PATH_OVERLAP: f32 = 0.15;

pub(super) fn constraint_relevant_paths(
    candidate: &CandidateFile,
    canonical_candidate: &CandidateFile,
    route: &[RouteSegment],
) -> Vec<String> {
    let anchor_paths = [candidate.path.as_str(), canonical_candidate.path.as_str()];
    normalized_values(
        route
            .iter()
            .filter(|segment| constraint_path_is_relevant(segment.kind, &segment.source_kind))
            .filter(|segment| {
                anchor_paths.iter().any(|anchor_path| {
                    segment.path == *anchor_path
                        || path_overlap_score(&segment.path, anchor_path)
                            >= MIN_CONSTRAINT_PATH_OVERLAP
                })
            })
            .map(|segment| segment.path.clone())
            .chain(
                [candidate, canonical_candidate]
                    .into_iter()
                    .filter(|entry| {
                        constraint_path_is_relevant(
                            super::common::classify_route_segment(&entry.path),
                            &entry.source_kind,
                        )
                    })
                    .map(|entry| entry.path.clone()),
            ),
    )
}

fn constraint_path_is_relevant(kind: RouteSegmentKind, source_kind: &str) -> bool {
    if matches!(kind, RouteSegmentKind::Ui | RouteSegmentKind::ApiClient) {
        return false;
    }
    matches!(
        kind,
        RouteSegmentKind::Endpoint
            | RouteSegmentKind::Service
            | RouteSegmentKind::Crud
            | RouteSegmentKind::Query
            | RouteSegmentKind::Migration
    ) || matches!(source_kind, "model" | "validator" | "constraint_source")
}

fn path_overlap_score(left: &str, right: &str) -> f32 {
    let left_tokens = path_tokens(left);
    let right_tokens = path_tokens(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }
    let overlap = left_tokens
        .iter()
        .filter(|token| right_tokens.contains(*token))
        .count();
    overlap as f32 / left_tokens.len().max(right_tokens.len()) as f32
}

fn path_tokens(value: &str) -> std::collections::HashSet<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .collect()
}
