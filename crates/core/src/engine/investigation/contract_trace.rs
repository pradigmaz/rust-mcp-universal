use anyhow::Result;

use crate::engine::Engine;
use crate::model::{
    ConceptClusterResult, ConceptSeedKind, ContractBreak, ContractTraceLink, ContractTraceResult,
    ContractTraceRole, InvestigationAnchor,
};

use super::actionability::{
    build_actionability, path_affinity, role_for_entry_variant, role_for_route_segment, role_rank,
    same_context,
};
use super::cluster::concept_cluster;
use super::common::{capability_status, normalized_values};

pub(super) fn contract_trace(
    engine: &Engine,
    seed: &str,
    seed_kind: ConceptSeedKind,
    limit: usize,
) -> Result<ContractTraceResult> {
    let cluster = concept_cluster(engine, seed, seed_kind, limit)?;
    Ok(contract_trace_from_cluster(cluster))
}

pub(super) fn contract_trace_from_cluster(cluster: ConceptClusterResult) -> ContractTraceResult {
    let seed_path =
        (cluster.seed.seed_kind == ConceptSeedKind::Path).then_some(cluster.seed.seed.as_str());
    let chain = canonical_chain(seed_path, &cluster);
    let contract_breaks = detect_contract_breaks(&chain);
    let manual_review_required = !contract_breaks.is_empty()
        || cluster
            .variants
            .iter()
            .any(|variant| !variant.gaps.is_empty());
    let recommended_followups = normalized_values(cluster.gaps.clone());
    let actionability = build_actionability(
        seed_path,
        &cluster.variants,
        &contract_breaks,
        &recommended_followups,
        manual_review_required,
    );
    let capability_status = capability_status(
        chain.len(),
        cluster.variants.len(),
        &cluster.unsupported_sources,
    );
    let confidence = if chain.is_empty() {
        cluster.confidence
    } else {
        (cluster.confidence
            + chain.iter().map(|link| link.confidence).sum::<f32>() / chain.len() as f32)
            / 2.0
    };

    ContractTraceResult {
        seed: cluster.seed,
        chain,
        contract_breaks,
        actionability,
        manual_review_required,
        capability_status,
        unsupported_sources: cluster.unsupported_sources,
        confidence,
    }
}

fn canonical_chain(
    seed_path: Option<&str>,
    cluster: &ConceptClusterResult,
) -> Vec<ContractTraceLink> {
    let mut links = Vec::new();
    for variant in &cluster.variants {
        let entry_role = role_for_entry_variant(variant);
        let rank_score = role_rank(entry_role)
            + variant.confidence * 0.2
            + variant.route_centrality.clamp(0.0, 1.0) * 0.1
            + path_affinity(seed_path, &variant.entry_anchor.path) * 0.35;
        links.push(ContractTraceLink {
            role: entry_role,
            anchor: variant.entry_anchor.clone(),
            source_kind: variant
                .route
                .first()
                .map(|segment| segment.source_kind.clone())
                .unwrap_or_else(|| "entry_anchor".to_string()),
            evidence: "entry_anchor".to_string(),
            confidence: variant.confidence,
            generated_lineage: variant.generated_lineage.clone(),
            rank_score,
            rank_reason: "variant_entry_confidence_and_role_priority".to_string(),
        });

        for segment in &variant.route {
            let role = role_for_route_segment(segment);
            let anchor = InvestigationAnchor {
                path: segment.path.clone(),
                language: segment.language.clone(),
                symbol: segment.anchor_symbol.clone(),
                kind: Some(format!("{:?}", segment.kind)),
                line: segment.source_span.as_ref().map(|span| span.start_line),
                column: segment
                    .source_span
                    .as_ref()
                    .and_then(|span| span.start_column),
            };
            links.push(ContractTraceLink {
                role,
                anchor,
                source_kind: segment.source_kind.clone(),
                evidence: segment.evidence.clone(),
                confidence: segment.score.clamp(0.0, 1.0),
                generated_lineage: None,
                rank_score: role_rank(role)
                    + segment.score.clamp(0.0, 1.0) * 0.15
                    + path_affinity(seed_path, &segment.path) * 0.35,
                rank_reason: "route_segment_role_priority".to_string(),
            });
        }
    }

    let mut deduped = Vec::new();
    for role in ordered_roles() {
        let candidates = links
            .iter()
            .filter(|link| link.role == *role)
            .filter(|link| !is_low_value_link(seed_path, link))
            .collect::<Vec<_>>();
        let contextual = candidates
            .iter()
            .copied()
            .filter(|link| same_context(seed_path, &link.anchor.path))
            .collect::<Vec<_>>();
        let pool = if contextual.is_empty() {
            candidates
        } else {
            contextual
        };
        if let Some(best) = pool
            .into_iter()
            .max_by(|left, right| left.rank_score.total_cmp(&right.rank_score))
            .cloned()
        {
            deduped.push(best);
        }
    }
    deduped
}

fn detect_contract_breaks(chain: &[ContractTraceLink]) -> Vec<ContractBreak> {
    let mut breaks = Vec::new();
    let has_schema = has_role(chain, ContractTraceRole::SchemaOrModel)
        || has_role(chain, ContractTraceRole::Migration);
    let has_endpoint = has_role(chain, ContractTraceRole::Endpoint);
    let has_service = has_role(chain, ContractTraceRole::Service)
        || has_role(chain, ContractTraceRole::Validator)
        || has_role(chain, ContractTraceRole::Adapter);
    let has_generated_client = has_role(chain, ContractTraceRole::GeneratedClient);
    let has_consumer = has_role(chain, ContractTraceRole::Consumer);
    let has_tests = has_role(chain, ContractTraceRole::Test);

    if (has_generated_client || has_consumer) && !has_service && !has_endpoint {
        breaks.push(ContractBreak {
            expected_role: ContractTraceRole::Service,
            reason: "chain_ended_before_backend_contract_root".to_string(),
            last_resolved_path: last_path(chain),
        });
    }
    if (has_endpoint || has_service) && !has_schema {
        breaks.push(ContractBreak {
            expected_role: ContractTraceRole::SchemaOrModel,
            reason: "schema_or_model_backing_not_found".to_string(),
            last_resolved_path: last_path(chain),
        });
    }
    if !has_tests {
        breaks.push(ContractBreak {
            expected_role: ContractTraceRole::Test,
            reason: "related_tests_not_found".to_string(),
            last_resolved_path: last_path(chain),
        });
    }
    for link in chain {
        if link
            .generated_lineage
            .as_ref()
            .is_some_and(|lineage| lineage.source_of_truth_path.is_none())
        {
            breaks.push(ContractBreak {
                expected_role: ContractTraceRole::SchemaOrModel,
                reason: "generated_target_without_source_of_truth".to_string(),
                last_resolved_path: Some(link.anchor.path.clone()),
            });
        }
    }

    let mut out = Vec::new();
    for item in breaks {
        if !out.iter().any(|existing: &ContractBreak| {
            existing.expected_role == item.expected_role && existing.reason == item.reason
        }) {
            out.push(item);
        }
    }
    out
}

fn has_role(chain: &[ContractTraceLink], role: ContractTraceRole) -> bool {
    chain.iter().any(|link| link.role == role)
}

fn last_path(chain: &[ContractTraceLink]) -> Option<String> {
    chain.last().map(|link| link.anchor.path.clone())
}

fn ordered_roles() -> &'static [ContractTraceRole] {
    &[
        ContractTraceRole::SchemaOrModel,
        ContractTraceRole::Migration,
        ContractTraceRole::Endpoint,
        ContractTraceRole::Service,
        ContractTraceRole::Validator,
        ContractTraceRole::Adapter,
        ContractTraceRole::GeneratedClient,
        ContractTraceRole::Consumer,
        ContractTraceRole::Test,
        ContractTraceRole::Unknown,
    ]
}

fn is_low_value_link(seed_path: Option<&str>, link: &ContractTraceLink) -> bool {
    matches!(
        link.role,
        ContractTraceRole::Test | ContractTraceRole::Unknown
    ) && path_affinity(seed_path, &link.anchor.path) < 0.30
}
