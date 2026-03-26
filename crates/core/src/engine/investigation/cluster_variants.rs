use anyhow::Result;

use crate::engine::Engine;
use crate::model::ConceptSeedKind;
use crate::model::ImplementationVariant;
use crate::model::RouteSegmentKind;
use crate::model::SemanticState;
use crate::vector_rank::SemanticRerankOutcome;

use super::body::extract_body_for_candidate;
use super::cluster_constraints::constraint_relevant_paths;
use super::cluster_policy::canonical_entry_candidate;
use super::cluster_scoring::ClusterScoringInputs;
use super::cluster_scoring::ClusterScoringSignals;
use super::cluster_scoring::compute_scoring_signals;
use super::common::CandidateFile;
use super::common::build_anchor;
use super::common::normalized_values;
use super::constraint_relevance::retain_relevant_constraints;
use super::constraints::collect_constraint_evidence;
use super::route::build_route;

pub(super) fn build_variants(
    engine: &Engine,
    candidates: &[CandidateFile],
    seed: &str,
    seed_kind: ConceptSeedKind,
    semantic_state: SemanticState,
    variant_failures: &mut Vec<String>,
) -> Vec<ImplementationVariant> {
    candidates
        .iter()
        .filter_map(|candidate| {
            match build_variant(engine, candidate, seed, seed_kind, semantic_state) {
                Ok(variant) => Some(variant),
                Err(err) => {
                    variant_failures
                        .push(format!("variant_build_failed:{}:{}", candidate.path, err));
                    None
                }
            }
        })
        .collect::<Vec<_>>()
}

pub(super) fn average_confidence(variants: &[ImplementationVariant]) -> f32 {
    if variants.is_empty() {
        0.0
    } else {
        variants
            .iter()
            .map(|variant| variant.confidence)
            .sum::<f32>()
            / variants.len() as f32
    }
}

pub(super) fn retain_relevant_variants(
    variants: Vec<ImplementationVariant>,
) -> Vec<ImplementationVariant> {
    let filtered = variants
        .iter()
        .filter(|variant| {
            variant.lexical_proximity > 0.0
                || variant.semantic_proximity > 0.0
                || variant.symbol_overlap > 0.0
                || variant.confidence >= 0.4
        })
        .cloned()
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        variants
    } else {
        filtered
    }
}

pub(super) fn semantic_state_for_seed(
    seed_kind: ConceptSeedKind,
    semantic_outcome: Option<SemanticRerankOutcome>,
) -> SemanticState {
    if !matches!(seed_kind, ConceptSeedKind::Query) {
        return SemanticState::NotApplicable;
    }
    match semantic_outcome {
        Some(
            SemanticRerankOutcome::AppliedRrfFallback
            | SemanticRerankOutcome::AppliedRrfIndexed
            | SemanticRerankOutcome::AppliedRrfMixed,
        ) => SemanticState::Used,
        Some(SemanticRerankOutcome::Failed) => SemanticState::UnavailableFailOpen,
        Some(SemanticRerankOutcome::NotApplied | SemanticRerankOutcome::ShortCircuitedLexical)
        | None => SemanticState::DisabledLowSignal,
    }
}

fn build_variant(
    engine: &Engine,
    candidate: &CandidateFile,
    seed: &str,
    seed_kind: ConceptSeedKind,
    semantic_state: SemanticState,
) -> Result<ImplementationVariant> {
    let route = build_route(engine, candidate)?;
    let canonical_candidate = canonical_entry_candidate(candidate, &route);
    let entry_anchor = build_anchor(&canonical_candidate);
    let body_anchor = extract_body_for_candidate(engine, &canonical_candidate)?
        .map(|item| item.anchor)
        .or_else(|| {
            extract_body_for_candidate(engine, candidate)
                .ok()
                .flatten()
                .map(|item| item.anchor)
        });
    let paths = constraint_relevant_paths(candidate, &canonical_candidate, &route);
    let constraints = retain_relevant_constraints(
        seed,
        seed_kind,
        collect_constraint_evidence(engine, &entry_anchor, &paths)?,
    );
    let related_tests = normalized_values(
        route
            .iter()
            .filter(|segment| segment.kind == RouteSegmentKind::Test)
            .map(|segment| segment.path.clone()),
    );
    let mut gaps = Vec::new();
    if body_anchor.is_none() {
        gaps.push("body_unresolved".to_string());
    }
    if constraints.is_empty() {
        gaps.push("no_constraint_evidence".to_string());
    }
    if related_tests.is_empty() {
        gaps.push("no_test_evidence".to_string());
    }
    if semantic_state == SemanticState::UnavailableFailOpen {
        gaps.push("semantic_unavailable_fail_open".to_string());
    }
    let strong_constraint_count = constraints
        .iter()
        .filter(|constraint| constraint.strength == "strong")
        .count();
    let weak_constraint_count = constraints.len().saturating_sub(strong_constraint_count);
    let body_unresolved = body_anchor.is_none();
    let no_constraint_evidence = constraints.is_empty();
    let no_test_evidence = related_tests.is_empty();
    let ClusterScoringSignals {
        lexical_proximity,
        semantic_proximity,
        route_centrality,
        symbol_overlap,
        constraint_overlap,
        test_adjacency,
        confidence,
        score_breakdown,
    } = compute_scoring_signals(ClusterScoringInputs {
        seed,
        candidate,
        route: &route,
        strong_constraint_count,
        weak_constraint_count,
        related_tests: &related_tests,
        semantic_state,
        body_unresolved,
        no_constraint_evidence,
        no_test_evidence,
    });
    Ok(ImplementationVariant {
        id: format!("variant:{}", entry_anchor.path),
        entry_anchor,
        body_anchor,
        route,
        constraints,
        related_tests,
        lexical_proximity,
        semantic_proximity,
        route_centrality,
        symbol_overlap,
        constraint_overlap,
        test_adjacency,
        semantic_state,
        score_model: "heuristic_v2".to_string(),
        score_breakdown,
        confidence,
        gaps,
    })
}
