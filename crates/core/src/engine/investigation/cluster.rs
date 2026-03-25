use std::collections::HashSet;

use anyhow::Result;

use crate::engine::Engine;
use crate::model::{
    ConceptClusterExpansionPolicy, ConceptClusterResult, ConceptClusterSummary, ConceptSeedKind,
    ConstraintEvidence, ConstraintEvidenceResult, ImplementationVariant, RouteSegmentKind,
    SemanticState,
};
use crate::vector_rank::SemanticRerankOutcome;

use super::body::extract_body_for_candidate;
use super::cluster_policy::{
    canonical_entry_candidate, cap_candidate_pool, collect_expanded_candidates, dedupe_candidates,
    dedupe_variants, expand_evidence_candidates,
};
use super::cluster_scoring::{ClusterScoringSignals, compute_scoring_signals};
use super::common::{
    CandidateFile, build_anchor, capability_status, collect_candidates, normalized_values,
    route_kind_label,
};
use super::constraints::collect_constraint_evidence;
use super::route::build_route;

pub(super) fn concept_cluster(
    engine: &Engine,
    seed: &str,
    seed_kind: ConceptSeedKind,
    limit: usize,
) -> Result<ConceptClusterResult> {
    let (seed, initial_candidates, unsupported_sources) =
        collect_candidates(engine, seed, seed_kind, limit)?;
    let (mut candidates, mut expansion_sources, semantic_outcome) = collect_expanded_candidates(
        engine,
        &seed.seed,
        seed.seed_kind,
        limit,
        initial_candidates,
    )?;
    let semantic_state = semantic_state_for_seed(seed.seed_kind, semantic_outcome);
    let mut preliminary_failures = Vec::new();
    let preliminary_variants = build_variants(
        engine,
        &candidates,
        &seed.seed,
        semantic_state,
        &mut preliminary_failures,
    );
    expand_evidence_candidates(
        &preliminary_variants,
        &mut candidates,
        &mut expansion_sources,
    );
    let candidates = cap_candidate_pool(dedupe_candidates(candidates), limit.max(1));
    let mut variant_failures = Vec::new();
    let variants = dedupe_variants(
        build_variants(
            engine,
            &candidates,
            &seed.seed,
            semantic_state,
            &mut variant_failures,
        ),
        limit.max(1),
    );
    let capability_status =
        capability_status(variants.len(), candidates.len(), &unsupported_sources);
    let gaps = normalized_values(
        variants
            .iter()
            .flat_map(|variant| variant.gaps.iter().cloned())
            .chain(variant_failures),
    );
    let summary = ConceptClusterSummary {
        variant_count: variants.len(),
        languages: normalized_values(
            variants
                .iter()
                .map(|variant| variant.entry_anchor.language.clone()),
        ),
        route_kinds: normalized_values(variants.iter().flat_map(|variant| {
            variant
                .route
                .iter()
                .map(|segment| route_kind_label(segment.kind).to_string())
        })),
        expansion_sources,
        expansion_policy: Some(ConceptClusterExpansionPolicy {
            initial_sources: vec![
                "retrieval_shortlist".to_string(),
                "symbol_neighbors".to_string(),
            ],
            enrichment_sources: vec![
                "semantic_retrieval".to_string(),
                "route_trace_anchors".to_string(),
                "related_files".to_string(),
            ],
            feedback_sources: vec!["tests".to_string(), "constraint_evidence".to_string()],
            route_trace_reused: true,
            candidate_pool_limit_multiplier: 3,
            dedup_unit: "entry_anchor.path".to_string(),
            tie_break_order: vec![
                "final_confidence".to_string(),
                "constraint_overlap".to_string(),
                "route_centrality".to_string(),
                "lexical_proximity".to_string(),
                "path".to_string(),
            ],
        }),
        cutoff_policy: Some(format!(
            "expand<=limit*3; score+dedup full pool; return top_{} by final confidence",
            limit.max(1),
        )),
        dedup_policy: Some(
            "candidate(path,symbol,line); variant(entry_anchor.path)->confidence,constraint,route,lexical,path"
                .to_string(),
        ),
    };
    Ok(ConceptClusterResult {
        seed,
        variants: variants.clone(),
        cluster_summary: summary,
        gaps,
        capability_status,
        unsupported_sources,
        confidence: average_confidence(&variants),
    })
}

pub(super) fn constraint_evidence(
    engine: &Engine,
    seed: &str,
    seed_kind: ConceptSeedKind,
    limit: usize,
) -> Result<ConstraintEvidenceResult> {
    let cluster = concept_cluster(engine, seed, seed_kind, limit)?;
    let items = normalized_constraint_items(&cluster.variants);
    Ok(ConstraintEvidenceResult {
        seed: cluster.seed,
        items,
        capability_status: cluster.capability_status,
        unsupported_sources: cluster.unsupported_sources,
        confidence: cluster.confidence,
    })
}

pub(super) fn build_variant(
    engine: &Engine,
    candidate: &CandidateFile,
    seed: &str,
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
    let paths = route
        .iter()
        .map(|segment| segment.path.clone())
        .chain(std::iter::once(candidate.path.clone()))
        .chain(std::iter::once(canonical_candidate.path.clone()))
        .collect::<Vec<_>>();
    let constraints = collect_constraint_evidence(engine, &entry_anchor, &paths)?;
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
    } = compute_scoring_signals(
        seed,
        candidate,
        &route,
        strong_constraint_count,
        weak_constraint_count,
        &related_tests,
        semantic_state,
        body_unresolved,
        no_constraint_evidence,
        no_test_evidence,
    );
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

fn build_variants(
    engine: &Engine,
    candidates: &[CandidateFile],
    seed: &str,
    semantic_state: SemanticState,
    variant_failures: &mut Vec<String>,
) -> Vec<ImplementationVariant> {
    candidates
        .iter()
        .filter_map(
            |candidate| match build_variant(engine, candidate, seed, semantic_state) {
                Ok(variant) => Some(variant),
                Err(err) => {
                    variant_failures
                        .push(format!("variant_build_failed:{}:{}", candidate.path, err));
                    None
                }
            },
        )
        .collect::<Vec<_>>()
}

fn normalized_constraint_items(variants: &[ImplementationVariant]) -> Vec<ConstraintEvidence> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for item in variants
        .iter()
        .flat_map(|variant| variant.constraints.iter())
    {
        if seen.insert((
            item.path.clone(),
            item.line_start,
            item.constraint_kind.clone(),
            item.normalized_key.clone(),
        )) {
            items.push(item.clone());
        }
    }
    items
}

fn average_confidence(variants: &[ImplementationVariant]) -> f32 {
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

fn semantic_state_for_seed(
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
