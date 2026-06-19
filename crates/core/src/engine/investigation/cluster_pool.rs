use std::collections::{HashMap, HashSet};

use crate::model::{ImplementationVariant, RouteSegment, RouteSegmentKind};

use super::common::{CandidateFile, detect_language};

pub(super) fn dedupe_candidates(candidates: Vec<CandidateFile>) -> Vec<CandidateFile> {
    let mut seen = HashSet::new();
    let mut out: Vec<CandidateFile> = Vec::new();
    for candidate in candidates {
        if seen.insert((
            candidate.path.clone(),
            candidate.symbol.clone(),
            candidate.line.unwrap_or(0),
        )) {
            out.push(candidate);
        }
    }
    out
}

pub(super) fn canonical_entry_candidate(
    candidate: &CandidateFile,
    route: &[RouteSegment],
) -> CandidateFile {
    let Some(segment) = route
        .iter()
        .filter(|segment| {
            !matches!(
                segment.kind,
                RouteSegmentKind::Unknown | RouteSegmentKind::Test | RouteSegmentKind::Migration
            )
        })
        .min_by_key(|segment| canonical_rank(segment.kind))
    else {
        return candidate.clone();
    };

    CandidateFile {
        path: segment.path.clone(),
        language: detect_language(&segment.path, &segment.language),
        line: segment.source_span.as_ref().map(|span| span.start_line),
        column: segment
            .source_span
            .as_ref()
            .and_then(|span| span.start_column),
        symbol: segment.anchor_symbol.clone(),
        symbol_kind: Some(format!("{:?}", segment.kind)),
        source_kind: "canonical_entry".to_string(),
        match_kind: candidate.match_kind,
        score: candidate.score.max(segment.score),
    }
}

pub(super) fn dedupe_variants(
    mut variants: Vec<ImplementationVariant>,
    limit: usize,
) -> Vec<ImplementationVariant> {
    variants.sort_by(compare_variants);
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<ImplementationVariant> = Vec::new();
    for variant in variants {
        let path = variant.entry_anchor.path.clone();
        if let Some(existing_idx) = seen.get(&path).copied() {
            let merge_marker = format!("merged_duplicate_variant:{path}");
            if !out[existing_idx]
                .gaps
                .iter()
                .any(|gap| gap == &merge_marker)
            {
                out[existing_idx].gaps.push(merge_marker);
            }
        } else {
            seen.insert(path, out.len());
            out.push(variant);
        }
        if out.len() >= limit.max(1) {
            break;
        }
    }
    out
}

pub(super) fn cap_candidate_pool(
    mut candidates: Vec<CandidateFile>,
    limit: usize,
) -> Vec<CandidateFile> {
    let max_candidates = limit.max(1).saturating_mul(3);
    candidates.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
    });
    candidates.truncate(max_candidates);
    candidates
}

fn compare_variants(
    left: &ImplementationVariant,
    right: &ImplementationVariant,
) -> std::cmp::Ordering {
    right
        .confidence
        .total_cmp(&left.confidence)
        .then_with(|| right.constraint_overlap.total_cmp(&left.constraint_overlap))
        .then_with(|| right.route_centrality.total_cmp(&left.route_centrality))
        .then_with(|| right.lexical_proximity.total_cmp(&left.lexical_proximity))
        .then_with(|| left.entry_anchor.path.cmp(&right.entry_anchor.path))
}

fn canonical_rank(kind: RouteSegmentKind) -> usize {
    match kind {
        RouteSegmentKind::Endpoint => 0,
        RouteSegmentKind::Service => 1,
        RouteSegmentKind::Crud => 2,
        RouteSegmentKind::Query => 3,
        RouteSegmentKind::ApiClient => 4,
        RouteSegmentKind::Ui => 5,
        RouteSegmentKind::Test => 6,
        RouteSegmentKind::Migration => 7,
        RouteSegmentKind::Unknown => 8,
    }
}
