use std::collections::BTreeSet;

use anyhow::Result;

use crate::engine::Engine;
use crate::model::{
    ConceptSeedKind, ImplementationVariant, PrivacyMode, QueryOptions, RouteSegment,
    RouteSegmentKind, RouteTraceResult, SemanticFailMode,
};
use crate::vector_rank::SemanticRerankOutcome;

use super::candidate_relevance::retain_query_relevant_candidates;
pub(super) use super::cluster_pool::{
    canonical_entry_candidate, cap_candidate_pool, dedupe_candidates, dedupe_variants,
};
use super::common::{CandidateFile, CandidateMatchKind, detect_language};
use super::path_helpers::display_path;

pub(super) fn collect_expanded_candidates(
    engine: &Engine,
    seed: &str,
    seed_kind: ConceptSeedKind,
    limit: usize,
    initial_candidates: Vec<CandidateFile>,
    prefetched_route_trace: Option<&RouteTraceResult>,
) -> Result<(
    Vec<CandidateFile>,
    Vec<String>,
    Option<SemanticRerankOutcome>,
)> {
    let mut candidates = initial_candidates.clone();
    let mut expansion_sources = BTreeSet::new();
    note_sources_from_candidates(&initial_candidates, &mut expansion_sources);
    let mut semantic_outcome = None;

    if matches!(seed_kind, ConceptSeedKind::Query) {
        let (semantic_hits, outcome) = engine.search_with_semantic_outcome(&QueryOptions {
            query: seed.trim().to_string(),
            limit: limit.max(1).saturating_mul(2),
            detailed: false,
            semantic: true,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
            agent_intent_mode: None,
        })?;
        semantic_outcome = Some(outcome);
        let mut added_semantic = false;
        for hit in semantic_hits {
            candidates.push(CandidateFile {
                path: hit.path.clone(),
                language: detect_language(&hit.path, &hit.language),
                line: None,
                column: None,
                symbol: None,
                symbol_kind: None,
                source_kind: "semantic_search_candidate".to_string(),
                match_kind: CandidateMatchKind::QuerySearch,
                score: hit.score,
            });
            added_semantic = true;
        }
        if added_semantic {
            expansion_sources.insert("semantic_retrieval".to_string());
        }
    }

    let route_trace = match prefetched_route_trace.cloned() {
        Some(route_trace) => Some(route_trace),
        None => engine.route_trace(seed, seed_kind, limit.max(1)).ok(),
    };
    if let Some(route_trace) = route_trace {
        let mut added_route_anchor = false;
        for segment in route_trace.best_route.segments.iter().chain(
            route_trace
                .alternate_routes
                .iter()
                .flat_map(|route| route.segments.iter()),
        ) {
            if matches!(
                segment.kind,
                RouteSegmentKind::Unknown | RouteSegmentKind::Test | RouteSegmentKind::Migration
            ) {
                continue;
            }
            candidates.push(candidate_from_segment(
                segment,
                "route_trace_anchor",
                segment.score.clamp(0.35, 1.0),
            ));
            added_route_anchor = true;
        }
        if added_route_anchor {
            expansion_sources.insert("route_trace_anchors".to_string());
        }
    }

    let mut added_related = false;
    for candidate in initial_candidates.iter().take(limit.max(1)) {
        if let Ok(hits) = engine.related_files(&candidate.path, 3) {
            for hit in hits {
                let path = display_path(&hit.path);
                candidates.push(CandidateFile {
                    path: path.clone(),
                    language: detect_language(&path, &hit.language),
                    line: None,
                    column: None,
                    symbol: None,
                    symbol_kind: None,
                    source_kind: "related_file_expansion".to_string(),
                    match_kind: CandidateMatchKind::PathAnchor,
                    score: hit.score.clamp(0.25, 0.9),
                });
                added_related = true;
            }
        }
    }
    if added_related {
        expansion_sources.insert("related_files".to_string());
    }

    let candidates = if matches!(seed_kind, ConceptSeedKind::Query) {
        retain_query_relevant_candidates(seed, dedupe_candidates(candidates), limit.max(1) * 4)
    } else {
        dedupe_candidates(candidates)
    };
    let candidates = cap_candidate_pool(candidates, limit.max(1));

    Ok((
        candidates,
        expansion_sources.into_iter().collect(),
        semantic_outcome,
    ))
}

pub(super) fn expand_evidence_candidates(
    variants: &[ImplementationVariant],
    candidates: &mut Vec<CandidateFile>,
    expansion_sources: &mut Vec<String>,
) {
    let mut added_tests = false;
    let mut added_constraints = false;
    for variant in variants {
        for test_path in &variant.related_tests {
            candidates.push(CandidateFile {
                path: test_path.clone(),
                language: detect_language(test_path, ""),
                line: None,
                column: None,
                symbol: None,
                symbol_kind: None,
                source_kind: "test_expansion".to_string(),
                match_kind: CandidateMatchKind::PathAnchor,
                score: 0.55,
            });
            added_tests = true;
        }
        for constraint in &variant.constraints {
            candidates.push(CandidateFile {
                path: constraint.path.clone(),
                language: detect_language(&constraint.path, ""),
                line: Some(constraint.line_start.max(1)),
                column: Some(1),
                symbol: None,
                symbol_kind: Some(constraint.constraint_kind.clone()),
                source_kind: "constraint_evidence_candidate".to_string(),
                match_kind: CandidateMatchKind::PathAnchor,
                score: if constraint.strength == "strong" {
                    0.65
                } else {
                    0.45
                },
            });
            added_constraints = true;
        }
    }
    if added_tests && !expansion_sources.iter().any(|item| item == "tests") {
        expansion_sources.push("tests".to_string());
    }
    if added_constraints
        && !expansion_sources
            .iter()
            .any(|item| item == "constraint_evidence")
    {
        expansion_sources.push("constraint_evidence".to_string());
    }
}

fn note_sources_from_candidates(
    candidates: &[CandidateFile],
    expansion_sources: &mut BTreeSet<String>,
) {
    for candidate in candidates {
        if candidate.source_kind == "search_candidate" {
            expansion_sources.insert("retrieval_shortlist".to_string());
        }
        if candidate.source_kind == "symbol_lookup" {
            expansion_sources.insert("symbol_neighbors".to_string());
        }
    }
}

fn candidate_from_segment(segment: &RouteSegment, source_kind: &str, score: f32) -> CandidateFile {
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
        source_kind: source_kind.to_string(),
        match_kind: CandidateMatchKind::PathAnchor,
        score,
    }
}
