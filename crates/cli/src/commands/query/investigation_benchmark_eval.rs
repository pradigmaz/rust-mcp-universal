use anyhow::Result;
use rmu_core::{
    ConceptSeedKind, Engine, InvestigationBenchmarkCase, InvestigationBenchmarkTool,
    InvestigationCaseReport, InvestigationThresholdVerdict, InvestigationThresholds,
    InvestigationToolMetrics, PrivacyMode,
};
use serde_json::Value;

#[path = "investigation_benchmark_aggregation.rs"]
mod aggregation;
#[path = "investigation_benchmark_assertions.rs"]
mod assertions;
#[path = "investigation_benchmark_thresholds.rs"]
mod thresholds;

use super::investigation_benchmark_metrics::evaluate_case_metrics;

pub(super) fn run_case(
    engine: &Engine,
    case: &InvestigationBenchmarkCase,
    limit: usize,
    privacy_mode: PrivacyMode,
) -> Result<InvestigationCaseReport> {
    let started = std::time::Instant::now();
    let payload = run_tool(engine, case.tool, &case.seed, case.seed_kind, limit)?;
    let latency_ms = started.elapsed().as_secs_f32() * 1000.0;
    let capability_status = payload["capability_status"]
        .as_str()
        .unwrap_or("unsupported")
        .to_string();
    let unsupported_sources = payload["unsupported_sources"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    let mut notes = Vec::new();
    let assertion_pass_count = case
        .expected_assertions
        .iter()
        .filter(|assertion| {
            let passed = assertions::evaluate_assertion(&payload, assertion);
            if !passed {
                notes.push(format!(
                    "assertion_failed:{}:{}",
                    assertion.kind,
                    assertion.value.clone().unwrap_or_default()
                ));
            }
            passed
        })
        .count();
    let capability_pass = capability_status == case.expected_capability_status;
    if !capability_pass {
        notes.push(format!(
            "capability_status_mismatch:{}!={}",
            capability_status, case.expected_capability_status
        ));
    }
    let privacy_failures = assertions::count_privacy_failures(engine, privacy_mode, &payload);
    if privacy_failures > 0 {
        notes.push(format!("privacy_failures={privacy_failures}"));
    }
    let snapshot = evaluate_case_metrics(case, &payload);
    let variant_rank_consistent =
        assertions::concept_cluster_rank_consistent(engine, case, limit, &payload);
    Ok(InvestigationCaseReport {
        id: case.id.clone(),
        tool: case.tool,
        fixture: case.fixture.clone(),
        pass: capability_pass && assertion_pass_count == case.expected_assertions.len(),
        assertion_pass_count,
        assertion_total_count: case.expected_assertions.len(),
        capability_status,
        expected_capability_status: case.expected_capability_status.clone(),
        unsupported_sources,
        privacy_failures,
        latency_ms,
        notes,
        returned_anchor_count: snapshot.returned_anchor_count,
        expected_body_anchor_count: snapshot.expected_body_anchor_count,
        matched_anchor_count: snapshot.matched_anchor_count,
        route_success_at_1: snapshot.route_success_at_1,
        route_success_at_3: snapshot.route_success_at_3,
        matched_route_segment_count: snapshot.matched_route_segment_count,
        correctly_typed_route_segment_count: snapshot.correctly_typed_route_segment_count,
        returned_constraint_count: snapshot.returned_constraint_count,
        matched_constraint_count: snapshot.matched_constraint_count,
        expected_constraint_source_count: snapshot.expected_constraint_source_count,
        recovered_constraint_source_count: snapshot.recovered_constraint_source_count,
        expected_variant_count: snapshot.expected_variant_count,
        recovered_variant_count_at_3: snapshot.recovered_variant_count_at_3,
        top_variant_match: snapshot.top_variant_match,
        variant_rank_consistent,
        semantic_state_present: snapshot.semantic_state_present,
        semantic_state_matches_expectation: snapshot.semantic_state_matches_expectation,
        semantic_fail_open_visible: snapshot.semantic_fail_open_visible,
        low_signal_semantic_false_penalty: snapshot.low_signal_semantic_false_penalty,
        returned_divergence_signal_count: snapshot.returned_divergence_signal_count,
        expected_divergence_signal_count: snapshot.expected_divergence_signal_count,
        matched_divergence_signal_count: snapshot.matched_divergence_signal_count,
        unexpected_divergence_signal_count: snapshot.unexpected_divergence_signal_count,
        evidence_fields_present: snapshot.evidence_fields_present,
        evidence_fields_total: snapshot.evidence_fields_total,
    })
}

pub(super) fn build_tool_metrics(
    cases: &[InvestigationCaseReport],
) -> Vec<InvestigationToolMetrics> {
    aggregation::build_tool_metrics(cases)
}

pub(super) fn evaluate_thresholds(
    thresholds: &InvestigationThresholds,
    metrics: &[InvestigationToolMetrics],
    privacy_failures: usize,
) -> InvestigationThresholdVerdict {
    thresholds::evaluate_thresholds(thresholds, metrics, privacy_failures)
}

pub(super) fn tool_label(tool: InvestigationBenchmarkTool) -> &'static str {
    match tool {
        InvestigationBenchmarkTool::SymbolBody => "symbol_body",
        InvestigationBenchmarkTool::RouteTrace => "route_trace",
        InvestigationBenchmarkTool::ConstraintEvidence => "constraint_evidence",
        InvestigationBenchmarkTool::ConceptCluster => "concept_cluster",
        InvestigationBenchmarkTool::DivergenceReport => "divergence_report",
    }
}

pub(super) fn run_tool(
    engine: &Engine,
    tool: InvestigationBenchmarkTool,
    seed: &str,
    seed_kind: ConceptSeedKind,
    limit: usize,
) -> Result<Value> {
    match tool {
        InvestigationBenchmarkTool::SymbolBody => {
            serde_json::to_value(engine.symbol_body(seed, seed_kind, limit)?).map_err(Into::into)
        }
        InvestigationBenchmarkTool::RouteTrace => {
            serde_json::to_value(engine.route_trace(seed, seed_kind, limit)?).map_err(Into::into)
        }
        InvestigationBenchmarkTool::ConstraintEvidence => {
            serde_json::to_value(engine.constraint_evidence(seed, seed_kind, limit)?)
                .map_err(Into::into)
        }
        InvestigationBenchmarkTool::ConceptCluster => {
            serde_json::to_value(engine.concept_cluster(seed, seed_kind, limit)?)
                .map_err(Into::into)
        }
        InvestigationBenchmarkTool::DivergenceReport => {
            serde_json::to_value(engine.divergence_report(seed, seed_kind, limit)?)
                .map_err(Into::into)
        }
    }
}
