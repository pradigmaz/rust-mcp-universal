use anyhow::Result;

use crate::engine::Engine;
use crate::model::{
    ConceptClusterExpansionPolicy, ConceptClusterResult, ConceptClusterSummary, ConceptSeedKind,
    ConstraintEvidenceResult,
};

use super::candidate_relevance::retain_query_relevant_candidates;
use super::cluster_policy::{
    cap_candidate_pool, collect_expanded_candidates, dedupe_candidates, dedupe_variants,
    expand_evidence_candidates,
};
use super::cluster_selection::diversify_variants;
use super::cluster_variants::{
    average_confidence, build_variants, retain_relevant_variants, semantic_state_for_seed,
};
use super::common::{
    capability_status, collect_candidates, normalized_values, route_kind_label,
};
use super::constraint_items::normalized_constraint_items;

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
        seed.seed_kind,
        semantic_state,
        &mut preliminary_failures,
    );
    expand_evidence_candidates(
        &preliminary_variants,
        &mut candidates,
        &mut expansion_sources,
    );
    let candidates = if matches!(seed.seed_kind, ConceptSeedKind::Query) {
        retain_query_relevant_candidates(
            &seed.seed,
            dedupe_candidates(candidates),
            limit.max(1).saturating_mul(4),
        )
    } else {
        dedupe_candidates(candidates)
    };
    let candidates = cap_candidate_pool(candidates, limit.max(1));
    let mut variant_failures = Vec::new();
    let variants = diversify_variants(
        retain_relevant_variants(dedupe_variants(
            build_variants(
                engine,
                &candidates,
                &seed.seed,
                seed.seed_kind,
                semantic_state,
                &mut variant_failures,
            ),
            limit.max(1),
        )),
        seed.seed_kind,
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
            "expand<=limit*3; score+dedup full pool; query seeds promote execution paths within top_4 when score gap<=0.05; return top_{}",
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
