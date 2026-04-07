use std::collections::{BTreeMap, BTreeSet};

use rmu_core::{InvestigationBenchmarkCase, InvestigationBenchmarkTool};
use serde_json::Value;

use super::investigation_benchmark_cluster_metrics::evaluate_cluster_metrics;
use super::investigation_benchmark_constraint_metrics::{
    constraint_matches_any, constraint_path, count_constraint,
};
use super::investigation_benchmark_route_metrics::{
    count_expected_entry_paths, count_expected_route_entry_paths, found_expected_entry_path,
    found_expected_route_entry, route_trace_paths,
};

#[derive(Default)]
pub(super) struct CaseMetricSnapshot {
    pub returned_anchor_count: usize,
    pub expected_body_anchor_count: usize,
    pub matched_anchor_count: usize,
    pub route_success_at_1: Option<bool>,
    pub route_success_at_3: Option<bool>,
    pub matched_route_segment_count: usize,
    pub correctly_typed_route_segment_count: usize,
    pub returned_constraint_count: usize,
    pub matched_constraint_count: usize,
    pub expected_constraint_source_count: usize,
    pub recovered_constraint_source_count: usize,
    pub expected_variant_count: usize,
    pub recovered_variant_count_at_3: usize,
    pub top_variant_match: Option<bool>,
    pub semantic_state_present: Option<bool>,
    pub semantic_state_matches_expectation: Option<bool>,
    pub semantic_fail_open_visible: Option<bool>,
    pub low_signal_semantic_false_penalty: Option<bool>,
    pub returned_divergence_signal_count: usize,
    pub expected_divergence_signal_count: usize,
    pub matched_divergence_signal_count: usize,
    pub unexpected_divergence_signal_count: usize,
    pub evidence_fields_present: usize,
    pub evidence_fields_total: usize,
}

pub(super) fn evaluate_case_metrics(
    case: &InvestigationBenchmarkCase,
    payload: &Value,
) -> CaseMetricSnapshot {
    let mut snapshot = CaseMetricSnapshot::default();
    let labels = &case.labels;

    let item_anchors = payload["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("anchor"))
        .collect::<Vec<_>>();
    snapshot.returned_anchor_count = item_anchors.len();
    snapshot.expected_body_anchor_count = labels.expected_body_anchors.len();
    snapshot.matched_anchor_count = item_anchors
        .iter()
        .filter(|anchor| anchor_matches_any(anchor, labels.expected_body_anchors.iter()))
        .count();

    let variants = payload["variants"].as_array().cloned().unwrap_or_default();
    let chain = payload["chain"].as_array().cloned().unwrap_or_default();
    let cluster_metrics = evaluate_cluster_metrics(labels, &variants);
    snapshot.top_variant_match = cluster_metrics.top_variant_match;
    snapshot.semantic_state_present = cluster_metrics.semantic_state_present;
    snapshot.semantic_state_matches_expectation =
        cluster_metrics.semantic_state_matches_expectation;
    snapshot.semantic_fail_open_visible = cluster_metrics.semantic_fail_open_visible;
    snapshot.low_signal_semantic_false_penalty = cluster_metrics.low_signal_semantic_false_penalty;
    let route_paths = route_trace_paths(payload);
    let chain_paths = contract_trace_paths(&chain);
    if !labels.expected_variant_entry_paths.is_empty() {
        snapshot.expected_variant_count = labels.expected_variant_entry_paths.len();
        snapshot.route_success_at_1 = if case.tool == InvestigationBenchmarkTool::RouteTrace {
            Some(found_expected_route_entry(
                route_paths.iter().copied().take(1),
                &labels.expected_variant_entry_paths,
            ))
        } else if case.tool == InvestigationBenchmarkTool::ContractTrace {
            Some(found_expected_contract_entry(
                chain_paths.iter().map(String::as_str).take(1),
                &labels.expected_variant_entry_paths,
            ))
        } else {
            Some(found_expected_entry_path(
                variants.iter().take(1),
                &labels.expected_variant_entry_paths,
            ))
        };
        snapshot.route_success_at_3 = if case.tool == InvestigationBenchmarkTool::RouteTrace {
            Some(found_expected_route_entry(
                route_paths.iter().copied().take(3),
                &labels.expected_variant_entry_paths,
            ))
        } else if case.tool == InvestigationBenchmarkTool::ContractTrace {
            Some(found_expected_contract_entry(
                chain_paths.iter().map(String::as_str).take(3),
                &labels.expected_variant_entry_paths,
            ))
        } else {
            Some(found_expected_entry_path(
                variants.iter().take(3),
                &labels.expected_variant_entry_paths,
            ))
        };
        snapshot.recovered_variant_count_at_3 =
            if case.tool == InvestigationBenchmarkTool::RouteTrace {
                count_expected_route_entry_paths(
                    route_paths.iter().copied().take(3),
                    &labels.expected_variant_entry_paths,
                )
            } else if case.tool == InvestigationBenchmarkTool::ContractTrace {
                count_expected_contract_entry_paths(
                    chain_paths.iter().map(String::as_str).take(3),
                    &labels.expected_variant_entry_paths,
                )
            } else {
                count_expected_entry_paths(
                    variants.iter().take(3),
                    &labels.expected_variant_entry_paths,
                )
            };
    }

    if !labels.expected_route_segments.is_empty() {
        let mut actual = BTreeMap::<String, BTreeSet<String>>::new();
        let segment_iter: Vec<&Value> = if case.tool == InvestigationBenchmarkTool::RouteTrace {
            route_paths
                .iter()
                .take(3)
                .flat_map(|route| route["segments"].as_array().into_iter().flatten())
                .collect()
        } else if case.tool == InvestigationBenchmarkTool::ContractTrace {
            chain.iter().take(3).collect()
        } else {
            variants
                .iter()
                .take(3)
                .flat_map(|variant| variant["route"].as_array().into_iter().flatten())
                .collect()
        };
        for segment in segment_iter {
            let kind = if case.tool == InvestigationBenchmarkTool::ContractTrace {
                segment["role"].as_str()
            } else {
                segment["kind"].as_str()
            };
            let path = if case.tool == InvestigationBenchmarkTool::ContractTrace {
                segment["anchor"]["path"].as_str()
            } else {
                segment["path"].as_str()
            };
            if let (Some(path), Some(kind)) = (path, kind) {
                actual
                    .entry(path.to_string())
                    .or_default()
                    .insert(kind.to_string());
            }
        }
        for label in &labels.expected_route_segments {
            if let Some(kinds) = actual.get(&label.path) {
                snapshot.matched_route_segment_count += 1;
                if kinds.contains(&label.kind) {
                    snapshot.correctly_typed_route_segment_count += 1;
                }
            }
        }
    }

    if matches!(
        case.tool,
        InvestigationBenchmarkTool::ConstraintEvidence
            | InvestigationBenchmarkTool::ConceptCluster
            | InvestigationBenchmarkTool::ContractTrace
            | InvestigationBenchmarkTool::DivergenceReport
    ) {
        let constraints = payload["items"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(
                variants
                    .iter()
                    .flat_map(|variant| variant["constraints"].as_array().into_iter().flatten()),
            )
            .collect::<Vec<_>>();
        snapshot.returned_constraint_count = constraints.len();
        snapshot.matched_constraint_count = constraints
            .iter()
            .filter(|item| constraint_matches_any(item, labels.expected_constraints.iter()))
            .count();
        let expected_sources = labels
            .expected_constraints
            .iter()
            .map(|label| label.path.clone())
            .collect::<BTreeSet<_>>();
        let recovered_sources = constraints
            .iter()
            .filter_map(|item| constraint_path(item))
            .filter(|path| expected_sources.contains(*path))
            .map(ToString::to_string)
            .collect::<BTreeSet<_>>();
        snapshot.expected_constraint_source_count = expected_sources.len();
        snapshot.recovered_constraint_source_count = recovered_sources.len();
    }

    if !labels.expected_divergence_signals.is_empty() {
        let signals = payload["divergence_signals"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        snapshot.returned_divergence_signal_count = signals.len();
        snapshot.expected_divergence_signal_count = labels.expected_divergence_signals.len();
        for signal in &signals {
            if divergence_signal_matches_any(signal, labels.expected_divergence_signals.iter()) {
                snapshot.matched_divergence_signal_count += 1;
            } else if divergence_signal_counts_as_false_positive(signal) {
                snapshot.unexpected_divergence_signal_count += 1;
            }
        }
    }

    let (present, total) = evidence_counts(case.tool, payload, &variants);
    snapshot.evidence_fields_present = present;
    snapshot.evidence_fields_total = total;
    snapshot
}

fn anchor_matches_any<'a, I>(anchor: &Value, labels: I) -> bool
where
    I: IntoIterator<Item = &'a rmu_core::InvestigationAnchorLabel>,
{
    labels.into_iter().any(|label| {
        anchor["path"].as_str() == Some(label.path.as_str())
            && label
                .symbol
                .as_deref()
                .is_none_or(|symbol| anchor["symbol"].as_str() == Some(symbol))
            && label
                .line
                .is_none_or(|line| anchor["line"].as_u64() == Some(line as u64))
    })
}

fn divergence_signal_matches_any<'a, I>(signal: &Value, labels: I) -> bool
where
    I: IntoIterator<Item = &'a rmu_core::InvestigationDivergenceSignalLabel>,
{
    labels.into_iter().any(|label| {
        signal["axis"].as_str() == Some(label.axis.as_str())
            && signal["severity"].as_str() == Some(label.severity.as_str())
            && label
                .evidence_strength
                .as_deref()
                .is_none_or(|value| signal["evidence_strength"].as_str() == Some(value))
            && label
                .classification_reason
                .as_deref()
                .is_none_or(|value| signal["classification_reason"].as_str() == Some(value))
    })
}

fn divergence_signal_counts_as_false_positive(signal: &Value) -> bool {
    !matches!(
        signal["severity"].as_str(),
        Some("informational" | "likely_expected")
    )
}

fn evidence_counts(
    tool: InvestigationBenchmarkTool,
    payload: &Value,
    variants: &[Value],
) -> (usize, usize) {
    let mut counts = (0, 0);
    if matches!(tool, InvestigationBenchmarkTool::SymbolBody) {
        for item in payload["items"].as_array().into_iter().flatten() {
            count_anchor(&item["anchor"], &mut counts);
            count_field(item["signature"].as_str(), &mut counts);
            count_field(item["body"].as_str(), &mut counts);
            count_span(&item["span"], &mut counts);
            count_field(item["source_kind"].as_str(), &mut counts);
        }
    }
    if matches!(tool, InvestigationBenchmarkTool::RouteTrace) {
        for route in route_trace_paths(payload) {
            count_number(route["total_hops"].as_u64(), &mut counts);
            count_number(route["collapsed_hops"].as_u64(), &mut counts);
            count_number(route["confidence"].as_f64(), &mut counts);
            for segment in route["segments"].as_array().into_iter().flatten() {
                count_field(segment["kind"].as_str(), &mut counts);
                count_field(segment["path"].as_str(), &mut counts);
                count_field(segment["language"].as_str(), &mut counts);
                count_field(segment["evidence"].as_str(), &mut counts);
                count_field(segment["relation_kind"].as_str(), &mut counts);
                count_field(segment["source_kind"].as_str(), &mut counts);
            }
        }
        for gap in payload["unresolved_gaps"].as_array().into_iter().flatten() {
            count_field(gap["reason"].as_str(), &mut counts);
        }
    }
    for variant in variants {
        count_anchor(&variant["entry_anchor"], &mut counts);
        count_number(variant["confidence"].as_f64(), &mut counts);
        count_field(variant["semantic_state"].as_str(), &mut counts);
        count_field(variant["score_model"].as_str(), &mut counts);
        count_number(
            variant["score_breakdown"]["penalties"].as_f64(),
            &mut counts,
        );
        count_number(variant["score_breakdown"]["final"].as_f64(), &mut counts);
        for segment in variant["route"].as_array().into_iter().flatten() {
            count_field(segment["kind"].as_str(), &mut counts);
            count_field(segment["path"].as_str(), &mut counts);
            count_field(segment["language"].as_str(), &mut counts);
            count_field(segment["evidence"].as_str(), &mut counts);
            count_field(segment["relation_kind"].as_str(), &mut counts);
            count_field(segment["source_kind"].as_str(), &mut counts);
        }
        for constraint in variant["constraints"].as_array().into_iter().flatten() {
            count_constraint(constraint, &mut counts);
        }
    }
    if matches!(tool, InvestigationBenchmarkTool::ConstraintEvidence) {
        for item in payload["items"].as_array().into_iter().flatten() {
            count_constraint(item, &mut counts);
        }
    }
    if matches!(tool, InvestigationBenchmarkTool::ContractTrace) {
        for link in payload["chain"].as_array().into_iter().flatten() {
            count_field(link["role"].as_str(), &mut counts);
            count_field(link["source_kind"].as_str(), &mut counts);
            count_field(link["evidence"].as_str(), &mut counts);
            count_number(link["confidence"].as_f64(), &mut counts);
            count_number(link["rank_score"].as_f64(), &mut counts);
            count_field(link["rank_reason"].as_str(), &mut counts);
            count_anchor(&link["anchor"], &mut counts);
            if let Some(lineage) = link.get("generated_lineage") {
                count_field(lineage["status"].as_str(), &mut counts);
                if lineage["status"].as_str().is_some_and(|status| {
                    status != "not_generated" && status != "generated_unknown_source"
                }) {
                    count_field(lineage["source_of_truth_path"].as_str(), &mut counts);
                }
            }
        }
        count_bool(payload["manual_review_required"].as_bool(), &mut counts);
        count_number(payload["confidence"].as_f64(), &mut counts);
        for break_item in payload["contract_breaks"].as_array().into_iter().flatten() {
            count_field(break_item["reason"].as_str(), &mut counts);
            count_field(break_item["expected_role"].as_str(), &mut counts);
            count_field(break_item["last_resolved_path"].as_str(), &mut counts);
        }
        count_field(payload["actionability"]["recommended_target_path"].as_str(), &mut counts);
        count_field(payload["actionability"]["recommended_target_role"].as_str(), &mut counts);
        count_field(payload["actionability"]["reason"].as_str(), &mut counts);
        count_non_empty_array(payload["actionability"]["related_tests"].as_array(), &mut counts);
        count_non_empty_array(payload["actionability"]["adjacent_paths"].as_array(), &mut counts);
        count_non_empty_array(payload["actionability"]["checks"].as_array(), &mut counts);
        count_non_empty_array(payload["actionability"]["rollback_sensitive_paths"].as_array(), &mut counts);
        count_bool(payload["actionability"]["manual_review_required"].as_bool(), &mut counts);
        for step in payload["actionability"]["next_steps"].as_array().into_iter().flatten() {
            count_field(step["kind"].as_str(), &mut counts);
            count_field(step["detail"].as_str(), &mut counts);
        }
    }
    if matches!(tool, InvestigationBenchmarkTool::DivergenceReport) {
        count_field(payload["surface_kind"].as_str(), &mut counts);
        count_field(payload["overall_severity"].as_str(), &mut counts);
        count_bool(payload["manual_review_required"].as_bool(), &mut counts);
        count_field(payload["summary"].as_str(), &mut counts);
        for signal in payload["divergence_signals"]
            .as_array()
            .into_iter()
            .flatten()
        {
            count_field(signal["severity"].as_str(), &mut counts);
            count_field(signal["axis"].as_str(), &mut counts);
            count_field(signal["evidence_strength"].as_str(), &mut counts);
            count_field(signal["classification_reason"].as_str(), &mut counts);
            count_field(signal["summary"].as_str(), &mut counts);
            count_non_empty_array(signal["variant_ids"].as_array(), &mut counts);
        }
        count_non_empty_array(payload["shared_evidence"].as_array(), &mut counts);
        count_non_empty_array(payload["recommended_followups"].as_array(), &mut counts);
    }
    if matches!(tool, InvestigationBenchmarkTool::ConceptCluster) {
        let policy = &payload["cluster_summary"]["expansion_policy"];
        count_non_empty_array(policy["initial_sources"].as_array(), &mut counts);
        count_non_empty_array(policy["enrichment_sources"].as_array(), &mut counts);
        count_non_empty_array(policy["feedback_sources"].as_array(), &mut counts);
        count_bool(policy["route_trace_reused"].as_bool(), &mut counts);
        count_number(
            policy["candidate_pool_limit_multiplier"].as_u64(),
            &mut counts,
        );
        count_field(policy["dedup_unit"].as_str(), &mut counts);
        count_non_empty_array(policy["tie_break_order"].as_array(), &mut counts);
    }
    counts
}

fn count_anchor(anchor: &Value, counts: &mut (usize, usize)) {
    count_field(anchor["path"].as_str(), counts);
    count_field(anchor["language"].as_str(), counts);
}

fn count_span(span: &Value, counts: &mut (usize, usize)) {
    count_number(span["start_line"].as_u64(), counts);
    count_number(span["end_line"].as_u64(), counts);
}

fn count_field(value: Option<&str>, counts: &mut (usize, usize)) {
    counts.1 += 1;
    if value.is_some_and(|value| !value.trim().is_empty()) {
        counts.0 += 1;
    }
}

fn count_number<T>(value: Option<T>, counts: &mut (usize, usize)) {
    counts.1 += 1;
    if value.is_some() {
        counts.0 += 1;
    }
}

fn count_bool(value: Option<bool>, counts: &mut (usize, usize)) {
    counts.1 += 1;
    if value.is_some() {
        counts.0 += 1;
    }
}

fn count_non_empty_array(value: Option<&Vec<Value>>, counts: &mut (usize, usize)) {
    counts.1 += 1;
    if value.is_some_and(|items| !items.is_empty()) {
        counts.0 += 1;
    }
}

fn contract_trace_paths(chain: &[Value]) -> Vec<String> {
    chain
        .iter()
        .filter_map(|link| link["anchor"]["path"].as_str())
        .map(ToString::to_string)
        .collect()
}

fn found_expected_contract_entry<'a>(
    actual_paths: impl Iterator<Item = &'a str>,
    expected_paths: &[String],
) -> bool {
    let actual = actual_paths.collect::<Vec<_>>();
    expected_paths
        .iter()
        .any(|expected| actual.contains(&expected.as_str()))
}

fn count_expected_contract_entry_paths<'a>(
    actual_paths: impl Iterator<Item = &'a str>,
    expected_paths: &[String],
) -> usize {
    let actual = actual_paths.collect::<Vec<_>>();
    expected_paths
        .iter()
        .filter(|expected| actual.contains(&expected.as_str()))
        .count()
}