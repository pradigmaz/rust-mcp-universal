use crate::model::{
    Actionability, ActionabilityStep, ContractBreak, ContractTraceRole, GeneratedLineageStatus,
    ImplementationVariant, RouteSegment, RouteSegmentKind,
};

use super::common::normalized_values;

pub(super) fn build_actionability(
    seed_path: Option<&str>,
    variants: &[ImplementationVariant],
    contract_breaks: &[ContractBreak],
    recommended_followups: &[String],
    manual_review_required: bool,
) -> Actionability {
    let related_tests = normalized_values(variants.iter().flat_map(|variant| {
        variant
            .related_tests
            .iter()
            .filter(|path| same_context(seed_path, path))
            .cloned()
    }));
    let adjacent_paths = normalized_values(variants.iter().flat_map(|variant| {
        variant
            .route
            .iter()
            .filter(|segment| same_context(seed_path, &segment.path))
            .map(|segment| segment.path.clone())
            .chain(
                std::iter::once(variant.entry_anchor.path.clone())
                    .filter(|path| same_context(seed_path, path)),
            )
    }))
    .into_iter()
    .take(8)
    .collect::<Vec<_>>();
    let rollback_sensitive_paths = normalized_values(variants.iter().flat_map(|variant| {
        variant.route.iter().filter_map(|segment| {
            if matches!(
                segment.kind,
                RouteSegmentKind::Migration | RouteSegmentKind::Query | RouteSegmentKind::Crud
            ) && same_context(seed_path, &segment.path)
            {
                Some(segment.path.clone())
            } else {
                None
            }
        })
    }));

    let recommended_target = select_recommended_target(seed_path, variants);
    let mut next_steps = Vec::new();
    let mut checks = Vec::new();

    if let Some((path, role, redirected_from_generated)) = &recommended_target {
        next_steps.push(ActionabilityStep {
            kind: "inspect_primary_target".to_string(),
            detail: format!("Inspect and update `{path}` as primary {} target", role_label(*role)),
        });
        if *redirected_from_generated {
            next_steps.push(ActionabilityStep {
                kind: "follow_source_of_truth".to_string(),
                detail: "Generated artifact detected. Change source of truth, not generated output"
                    .to_string(),
            });
        }
    } else {
        next_steps.push(ActionabilityStep {
            kind: "manual_trace".to_string(),
            detail: "No stable source-of-truth target found. Inspect contract chain manually"
                .to_string(),
        });
    }

    if related_tests.is_empty() {
        next_steps.push(ActionabilityStep {
            kind: "add_targeted_tests".to_string(),
            detail: "Add tests covering each affected execution path before widening changes"
                .to_string(),
        });
    } else {
        checks.push("run_related_tests".to_string());
        next_steps.push(ActionabilityStep {
            kind: "verify_related_tests".to_string(),
            detail: "Run related tests after contract-target change".to_string(),
        });
    }

    if !adjacent_paths.is_empty() {
        checks.push("review_adjacent_impact".to_string());
    }
    if !rollback_sensitive_paths.is_empty() {
        checks.push("review_rollback_sensitivity".to_string());
    }
    if manual_review_required || !contract_breaks.is_empty() {
        next_steps.push(ActionabilityStep {
            kind: "manual_review".to_string(),
            detail: "Manual review required because chain is partial or proxy-only".to_string(),
        });
    }
    next_steps.extend(recommended_followups.iter().take(2).map(|detail| ActionabilityStep {
        kind: "followup".to_string(),
        detail: detail.clone(),
    }));

    let (recommended_target_path, recommended_target_role, reason) = match recommended_target {
        Some((path, role, true)) => (
            Some(path),
            Some(role),
            "generated_artifact_redirected_to_source_of_truth".to_string(),
        ),
        Some((path, role, false)) => (
            Some(path),
            Some(role),
            "highest_confidence_contract_target".to_string(),
        ),
        None => (
            None,
            None,
            "manual_review_required_no_stable_source_of_truth".to_string(),
        ),
    };

    Actionability {
        recommended_target_path,
        recommended_target_role,
        reason,
        next_steps,
        related_tests,
        adjacent_paths,
        checks: normalized_values(checks),
        rollback_sensitive_paths,
        manual_review_required: manual_review_required || !contract_breaks.is_empty(),
    }
}

pub(super) fn role_for_route_segment(segment: &RouteSegment) -> ContractTraceRole {
    match segment.kind {
        RouteSegmentKind::Migration | RouteSegmentKind::Query | RouteSegmentKind::Crud => {
            ContractTraceRole::SchemaOrModel
        }
        RouteSegmentKind::Endpoint => ContractTraceRole::Endpoint,
        RouteSegmentKind::Service => {
            if segment.source_kind == "validator"
                || segment.path.to_ascii_lowercase().contains("validator")
            {
                ContractTraceRole::Validator
            } else if segment.path.to_ascii_lowercase().contains("adapter") {
                ContractTraceRole::Adapter
            } else {
                ContractTraceRole::Service
            }
        }
        RouteSegmentKind::ApiClient => ContractTraceRole::GeneratedClient,
        RouteSegmentKind::Ui => ContractTraceRole::Consumer,
        RouteSegmentKind::Test => ContractTraceRole::Test,
        RouteSegmentKind::Unknown => {
            if segment.path.to_ascii_lowercase().contains("adapter") {
                ContractTraceRole::Adapter
            } else {
                ContractTraceRole::Unknown
            }
        }
    }
}

pub(super) fn role_rank(role: ContractTraceRole) -> f32 {
    match role {
        ContractTraceRole::SchemaOrModel => 1.0,
        ContractTraceRole::Migration => 0.96,
        ContractTraceRole::Endpoint => 0.94,
        ContractTraceRole::Service => 0.9,
        ContractTraceRole::Validator => 0.84,
        ContractTraceRole::Adapter => 0.7,
        ContractTraceRole::GeneratedClient => 0.35,
        ContractTraceRole::Consumer => 0.52,
        ContractTraceRole::Test => 0.2,
        ContractTraceRole::Unknown => 0.1,
    }
}

pub(super) fn path_affinity(seed_path: Option<&str>, candidate_path: &str) -> f32 {
    let Some(seed_path) = seed_path else {
        return 0.0;
    };
    let seed_parts = normalize_path(seed_path);
    let candidate_parts = normalize_path(candidate_path);
    if seed_parts.is_empty() || candidate_parts.is_empty() {
        return 0.0;
    }
    let max_len = seed_parts.len().min(candidate_parts.len()).saturating_sub(1);
    let shared = seed_parts
        .iter()
        .take(max_len)
        .zip(candidate_parts.iter().take(max_len))
        .take_while(|(left, right)| left == right)
        .count();
    if shared == 0 {
        return 0.0;
    }
    shared as f32 / max_len.max(1) as f32
}

fn select_recommended_target(
    seed_path: Option<&str>,
    variants: &[ImplementationVariant],
) -> Option<(String, ContractTraceRole, bool)> {
    let mut best: Option<(f32, String, ContractTraceRole, bool)> = None;
    for variant in variants {
        if let Some(lineage) = &variant.generated_lineage {
            if matches!(
                lineage.status,
                GeneratedLineageStatus::Generated | GeneratedLineageStatus::SuspectedGenerated
            ) {
                if let Some(source_path) = lineage.source_of_truth_path.as_ref() {
                    let role = ContractTraceRole::SchemaOrModel;
                    let score = role_rank(role)
                        + lineage.confidence * 0.2
                        + path_affinity(seed_path, source_path) * 0.35;
                    if best.as_ref().is_none_or(|current| score > current.0) {
                        best = Some((score, source_path.clone(), role, true));
                    }
                }
            }
        }

        let entry_role = role_for_entry_variant(variant);
        let entry_path = variant.entry_anchor.path.clone();
        if !is_disfavored_target(entry_role, seed_path, &entry_path) {
            let entry_score = role_rank(entry_role)
                + variant.confidence * 0.2
                + variant.route_centrality.clamp(0.0, 1.0) * 0.1
                + path_affinity(seed_path, &entry_path) * 0.35;
            if best.as_ref().is_none_or(|current| entry_score > current.0) {
                best = Some((entry_score, entry_path, entry_role, false));
            }
        }

        for segment in &variant.route {
            let role = role_for_route_segment(segment);
            if role == ContractTraceRole::GeneratedClient
                || is_disfavored_target(role, seed_path, &segment.path)
            {
                continue;
            }
            let score = role_rank(role)
                + segment.score.clamp(0.0, 1.0) * 0.15
                + path_affinity(seed_path, &segment.path) * 0.35;
            if best.as_ref().is_none_or(|current| score > current.0) {
                best = Some((score, segment.path.clone(), role, false));
            }
        }
    }
    best.map(|(_, path, role, redirected)| (path, role, redirected))
}

pub(super) fn role_for_entry_variant(variant: &ImplementationVariant) -> ContractTraceRole {
    if variant
        .generated_lineage
        .as_ref()
        .is_some_and(|lineage| lineage.status != GeneratedLineageStatus::NotGenerated)
    {
        return ContractTraceRole::GeneratedClient;
    }
    match variant.entry_anchor.kind.as_deref() {
        Some("Query" | "Crud") => ContractTraceRole::SchemaOrModel,
        Some("Endpoint") => ContractTraceRole::Endpoint,
        Some("Ui") => ContractTraceRole::Consumer,
        Some("Test") => ContractTraceRole::Test,
        Some("Service") => {
            let lowered = variant.entry_anchor.path.to_ascii_lowercase();
            if lowered.contains("validator") {
                ContractTraceRole::Validator
            } else if lowered.contains("adapter") {
                ContractTraceRole::Adapter
            } else {
                ContractTraceRole::Service
            }
        }
        _ => variant
            .route
            .first()
            .map(role_for_route_segment)
            .unwrap_or_else(|| {
                if variant.entry_anchor.path.to_ascii_lowercase().contains("test") {
                    ContractTraceRole::Test
                } else {
                    ContractTraceRole::Unknown
                }
            }),
    }
}

fn is_disfavored_target(
    role: ContractTraceRole,
    seed_path: Option<&str>,
    candidate_path: &str,
) -> bool {
    matches!(role, ContractTraceRole::Test | ContractTraceRole::Unknown)
        && path_affinity(seed_path, candidate_path) < 0.30
}

pub(super) fn same_context(seed_path: Option<&str>, candidate_path: &str) -> bool {
    let Some(seed_path) = seed_path else {
        return true;
    };
    context_root(seed_path) == context_root(candidate_path)
}

fn context_root(path: &str) -> Vec<String> {
    normalize_path(path)
        .into_iter()
        .take(4)
        .collect()
}

fn normalize_path(path: &str) -> Vec<String> {
    path.replace('\\', "/")
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_ascii_lowercase())
        .collect()
}

fn role_label(role: ContractTraceRole) -> &'static str {
    match role {
        ContractTraceRole::SchemaOrModel => "schema_or_model",
        ContractTraceRole::Endpoint => "endpoint",
        ContractTraceRole::Service => "service",
        ContractTraceRole::GeneratedClient => "generated_client",
        ContractTraceRole::Consumer => "consumer",
        ContractTraceRole::Test => "test",
        ContractTraceRole::Migration => "migration",
        ContractTraceRole::Validator => "validator",
        ContractTraceRole::Adapter => "adapter",
        ContractTraceRole::Unknown => "unknown",
    }
}