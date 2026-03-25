use anyhow::Result;

use crate::engine::Engine;
use crate::model::{
    AxisObservation, ConceptSeedKind, DivergenceAxis, DivergenceReport, DivergenceSignal,
    ImplementationVariant, RouteSegmentKind,
};

use super::cluster::concept_cluster;
use super::common::normalized_values;

pub(super) fn divergence_report(
    engine: &Engine,
    seed: &str,
    seed_kind: ConceptSeedKind,
    limit: usize,
) -> Result<DivergenceReport> {
    let cluster = concept_cluster(engine, seed, seed_kind, limit)?;
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
    Ok(DivergenceReport {
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
        recommended_followups,
        overall_confidence: cluster.confidence,
        capability_status: cluster.capability_status,
        unsupported_sources: cluster.unsupported_sources,
    })
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

fn build_divergence_signals(
    variants: &[ImplementationVariant],
    divergence_axes: &[DivergenceAxis],
) -> Vec<DivergenceSignal> {
    let shared_strong_constraints = shared_strong_constraints(variants);
    let has_test_gap = variants
        .iter()
        .any(|variant| variant.related_tests.is_empty());
    let material_proxy_axis_count = divergence_axes
        .iter()
        .filter(|axis| is_material_proxy_axis(axis.axis.as_str()))
        .count();
    divergence_axes
        .iter()
        .map(|axis| {
            let classification = classify_signal(
                axis.axis.as_str(),
                &shared_strong_constraints,
                has_test_gap,
                variants,
                material_proxy_axis_count,
            );
            DivergenceSignal {
                severity: classification.severity.to_string(),
                axis: axis.axis.clone(),
                evidence_strength: classification.evidence_strength.to_string(),
                classification_reason: classification.classification_reason.to_string(),
                summary: format!(
                    "{} diverges across {} variants",
                    axis.axis,
                    axis.values.len()
                ),
                variant_ids: axis
                    .values
                    .iter()
                    .map(|value| value.variant_id.clone())
                    .collect(),
            }
        })
        .collect()
}

struct SignalClassification {
    severity: &'static str,
    evidence_strength: &'static str,
    classification_reason: &'static str,
}

fn shared_strong_constraints(variants: &[ImplementationVariant]) -> Vec<String> {
    let mut shared: Option<Vec<String>> = None;
    for variant in variants {
        let current = normalized_values(
            variant
                .constraints
                .iter()
                .filter(|item| item.strength == "strong")
                .map(|item| item.normalized_text.clone()),
        );
        shared = Some(match shared.take() {
            Some(existing) => existing
                .into_iter()
                .filter(|item| current.contains(item))
                .collect(),
            None => current,
        });
    }
    shared.unwrap_or_default()
}

fn classify_signal(
    axis: &str,
    shared_strong_constraints: &[String],
    has_test_gap: bool,
    variants: &[ImplementationVariant],
    material_proxy_axis_count: usize,
) -> SignalClassification {
    let has_any_strong_constraints = variants.iter().any(|variant| {
        variant
            .constraints
            .iter()
            .any(|constraint| constraint.strength == "strong")
    });
    if axis == "constraint_evidence"
        && has_any_strong_constraints
        && shared_strong_constraints.is_empty()
    {
        return SignalClassification {
            severity: "high_risk",
            evidence_strength: "hard",
            classification_reason: "conflicting_hard_constraints",
        };
    }
    if axis == "db_entities_and_queries" && has_test_gap {
        return SignalClassification {
            severity: "high_risk",
            evidence_strength: "corroborated_proxy",
            classification_reason: "db_query_without_test_backing",
        };
    }
    if axis == "entrypoints" {
        return SignalClassification {
            severity: "informational",
            evidence_strength: "proxy_only",
            classification_reason: "entrypoint_only",
        };
    }
    if axis == "test_coverage" {
        return SignalClassification {
            severity: "likely_expected",
            evidence_strength: "proxy_only",
            classification_reason: "test_only",
        };
    }
    if axis == "downstream_symbols" && !shared_strong_constraints.is_empty() && !has_test_gap {
        return SignalClassification {
            severity: "likely_expected",
            evidence_strength: "corroborated_proxy",
            classification_reason: "shared_backing_downstream_variation",
        };
    }
    if is_material_proxy_axis(axis) {
        if material_proxy_axis_count >= 2 {
            return SignalClassification {
                severity: "suspicious",
                evidence_strength: "corroborated_proxy",
                classification_reason: "multi_axis_proxy_corroboration",
            };
        }
        if shared_strong_constraints.is_empty() && has_test_gap {
            return SignalClassification {
                severity: "suspicious",
                evidence_strength: "corroborated_proxy",
                classification_reason: "single_axis_proxy_plus_test_gap",
            };
        }
        return SignalClassification {
            severity: "likely_expected",
            evidence_strength: "proxy_only",
            classification_reason: "single_axis_proxy_only",
        };
    }
    SignalClassification {
        severity: "likely_expected",
        evidence_strength: "proxy_only",
        classification_reason: "single_axis_proxy_only",
    }
}

fn is_material_proxy_axis(axis: &str) -> bool {
    matches!(
        axis,
        "guards_and_validators"
            | "predicate_signatures"
            | "db_entities_and_queries"
            | "constraint_evidence"
    )
}

fn build_summary(
    variants: &[ImplementationVariant],
    divergence_signals: &[DivergenceSignal],
    overall_severity: &str,
    manual_review_required: bool,
    all_non_informational_proxy_only: bool,
) -> String {
    let key_axes = divergence_signals
        .iter()
        .filter(|signal| severity_rank(&signal.severity) >= severity_rank("suspicious"))
        .map(|signal| signal.axis.clone())
        .take(2)
        .collect::<Vec<_>>();
    let axis_summary = if key_axes.is_empty() {
        "no material risk axes identified".to_string()
    } else {
        format!("key axes: {}", key_axes.join(", "))
    };
    let review_summary = if all_non_informational_proxy_only {
        "proxy-only divergence; do not treat as a bug without hard evidence".to_string()
    } else if manual_review_required {
        "manual review required for proxy-only signals".to_string()
    } else {
        axis_summary
    };
    format!(
        "{} variants, {} divergence axes; highest severity {} ({})",
        variants.len(),
        divergence_signals.len(),
        overall_severity,
        review_summary
    )
}

fn recommended_followups(
    divergence_axes: &[DivergenceAxis],
    shared_evidence: &[String],
    has_test_gap: bool,
    unknowns: &[String],
    missing_evidence: &[String],
    divergence_signals: &[DivergenceSignal],
) -> Vec<String> {
    let mut followups = Vec::new();
    let axis_names = divergence_axes
        .iter()
        .map(|axis| axis.axis.as_str())
        .collect::<Vec<_>>();

    if axis_names.contains(&"constraint_evidence") && shared_evidence.is_empty() {
        followups.push(
            "Inspect schema, migration, and model backing for conflicting constraint evidence."
                .to_string(),
        );
    }
    if axis_names.contains(&"db_entities_and_queries") && has_test_gap {
        followups.push(
            "Verify diverging query paths and add tests covering each database-facing variant."
                .to_string(),
        );
    }
    if axis_names.contains(&"entrypoints")
        && axis_names
            .iter()
            .all(|axis| matches!(*axis, "entrypoints" | "test_coverage"))
    {
        followups.push(
            "Manual review only: divergence is limited to entrypoints or test backing and looks likely expected."
                .to_string(),
        );
    }
    if !missing_evidence.is_empty() || !unknowns.is_empty() {
        followups.push(
            "Collect additional evidence for unresolved gaps before treating this divergence as a bug."
                .to_string(),
        );
    }
    let all_non_informational_proxy_only = divergence_signals
        .iter()
        .filter(|signal| signal.severity != "informational")
        .all(|signal| signal.evidence_strength == "proxy_only");
    let has_non_informational_signal = divergence_signals
        .iter()
        .any(|signal| signal.severity != "informational");
    if has_non_informational_signal && all_non_informational_proxy_only {
        followups.push(
            "Do not treat this divergence as a bug until hard evidence contradicts the current variants."
                .to_string(),
        );
    }

    normalized_values(followups)
}

fn overall_severity(divergence_signals: &[DivergenceSignal]) -> String {
    divergence_signals
        .iter()
        .map(|signal| signal.severity.as_str())
        .max_by_key(|severity| severity_rank(severity))
        .unwrap_or("informational")
        .to_string()
}

fn manual_review_required(divergence_signals: &[DivergenceSignal]) -> bool {
    let non_informational = divergence_signals
        .iter()
        .filter(|signal| signal.severity != "informational")
        .collect::<Vec<_>>();
    !non_informational.is_empty()
        && non_informational
            .iter()
            .all(|signal| signal.evidence_strength == "proxy_only")
}

fn severity_rank(severity: &str) -> usize {
    match severity {
        "informational" => 0,
        "likely_expected" => 1,
        "suspicious" => 2,
        "high_risk" => 3,
        _ => 0,
    }
}

#[cfg(test)]
#[path = "divergence_tests.rs"]
mod tests;
