use anyhow::Result;

use crate::engine::Engine;
use crate::model::{
    AxisObservation, ConceptClusterResult, ConceptSeedKind, DivergenceAxis, DivergenceReport,
    ImplementationVariant, RouteSegmentKind,
};

use super::actionability::build_actionability;
use super::cluster::concept_cluster;
use super::common::normalized_values;
#[cfg(test)]
use super::divergence_signals::classify_signal;
use super::divergence_signals::{
    build_divergence_signals, build_summary, manual_review_required, overall_severity,
    recommended_followups, shared_strong_constraints,
};

pub(super) fn divergence_report(
    engine: &Engine,
    seed: &str,
    seed_kind: ConceptSeedKind,
    limit: usize,
) -> Result<DivergenceReport> {
    let cluster = concept_cluster(engine, seed, seed_kind, limit)?;
    Ok(divergence_report_from_cluster(cluster))
}

pub(super) fn divergence_report_from_cluster(cluster: ConceptClusterResult) -> DivergenceReport {
    let variants = cluster.variants.clone();
    let mut consensus_axes = Vec::new();
    let mut divergence_axes = Vec::new();
    let mut missing_evidence = cluster.gaps.clone();

    for axis in AXES {
        let values = variants
            .iter()
            .map(|variant| AxisObservation {
                variant_id: variant.id.clone(),
                values: axis_values(variant, axis),
            })
            .collect::<Vec<_>>();
        let unique = normalized_values(values.iter().map(|entry| entry.values.join(" | ")));
        if unique.is_empty() {
            missing_evidence.push(axis.to_string());
        } else if unique.len() == 1 {
            consensus_axes.push(DivergenceAxis {
                axis: axis.to_string(),
                values,
            });
        } else {
            divergence_axes.push(DivergenceAxis {
                axis: axis.to_string(),
                values,
            });
        }
    }

    let divergence_signals = build_divergence_signals(&variants, &divergence_axes);
    let shared_evidence = shared_strong_constraints(&variants);
    let has_test_gap = variants
        .iter()
        .any(|variant| variant.related_tests.is_empty());
    let unknowns = normalized_values(
        variants
            .iter()
            .flat_map(|variant| variant.gaps.iter().cloned())
            .chain(cluster.gaps.iter().cloned()),
    );
    let overall_severity = overall_severity(&divergence_signals);
    let manual_review_required = manual_review_required(&divergence_signals);
    let has_non_informational_signal = divergence_signals
        .iter()
        .any(|signal| signal.severity != "informational");
    let all_non_informational_proxy_only = has_non_informational_signal
        && divergence_signals
            .iter()
            .filter(|signal| signal.severity != "informational")
            .all(|signal| signal.evidence_strength == "proxy_only");
    let summary = build_summary(
        &variants,
        &divergence_signals,
        &overall_severity,
        manual_review_required,
        all_non_informational_proxy_only,
    );
    let recommended_followups = recommended_followups(
        &divergence_axes,
        &shared_evidence,
        has_test_gap,
        &unknowns,
        &missing_evidence,
        &divergence_signals,
    );
    let seed_path =
        (cluster.seed.seed_kind == ConceptSeedKind::Path).then(|| cluster.seed.seed.clone());
    DivergenceReport {
        surface_kind: SURFACE_KIND.to_string(),
        seed: cluster.seed,
        variants,
        consensus_axes,
        divergence_axes,
        divergence_signals,
        overall_severity,
        manual_review_required,
        summary,
        shared_evidence,
        unknowns,
        missing_evidence: normalized_values(missing_evidence),
        recommended_followups: recommended_followups.clone(),
        actionability: build_actionability(
            seed_path.as_deref(),
            &cluster.variants,
            &[],
            &recommended_followups,
            manual_review_required,
        ),
        overall_confidence: cluster.confidence,
        capability_status: cluster.capability_status,
        unsupported_sources: cluster.unsupported_sources,
    }
}

const AXES: [&str; 7] = [
    "entrypoints",
    "guards_and_validators",
    "predicate_signatures",
    "downstream_symbols",
    "db_entities_and_queries",
    "constraint_evidence",
    "test_coverage",
];

const SURFACE_KIND: &str = "divergence_explainability";

fn axis_values(variant: &ImplementationVariant, axis: &str) -> Vec<String> {
    match axis {
        "entrypoints" => vec![variant.entry_anchor.path.clone()],
        "guards_and_validators" => normalized_values(
            variant
                .route
                .iter()
                .filter(|segment| segment.source_kind == "validator")
                .map(|segment| format!("validator:{}:{}", segment.relation_kind, segment.path))
                .chain(
                    variant
                        .constraints
                        .iter()
                        .filter(|item| {
                            item.constraint_kind == "runtime_guard"
                                || item.source_kind == "runtime_guard_code"
                        })
                        .map(|item| format!("guard:{}:{}", item.source_kind, item.source_path)),
                ),
        ),
        "predicate_signatures" => normalized_values(
            variant
                .constraints
                .iter()
                .filter(|item| {
                    item.constraint_kind == "runtime_guard"
                        || item.source_kind == "runtime_guard_code"
                        || item.source_path.to_ascii_lowercase().contains("validator")
                })
                .map(|item| item.normalized_text.clone()),
        ),
        "downstream_symbols" => normalized_values(variant.route.iter().skip(1).map(|segment| {
            format!(
                "{}|{}|{}",
                segment.anchor_symbol.as_deref().unwrap_or("-"),
                segment.relation_kind,
                segment.path
            )
        })),
        "db_entities_and_queries" => normalized_values(
            variant
                .route
                .iter()
                .filter(|segment| {
                    matches!(
                        segment.kind,
                        RouteSegmentKind::Crud | RouteSegmentKind::Query
                    )
                })
                .map(|segment| {
                    format!(
                        "{}|{}|{}",
                        segment.source_kind, segment.relation_kind, segment.path
                    )
                }),
        ),
        "constraint_evidence" => normalized_values(
            variant
                .constraints
                .iter()
                .map(|item| item.normalized_text.clone()),
        ),
        "test_coverage" => normalized_values(variant.related_tests.clone()),
        _ => Vec::new(),
    }
}

#[cfg(test)]
#[path = "divergence_tests.rs"]
mod tests;
