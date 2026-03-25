use std::collections::BTreeMap;
use std::fmt::Display;

use anyhow::Result;
use rmu_core::{
    ConceptSeedKind, Engine, InvestigationAssertion, InvestigationBenchmarkCase,
    InvestigationBenchmarkTool, InvestigationCaseReport, InvestigationThresholdVerdict,
    InvestigationThresholds, InvestigationToolMetrics, PrivacyMode, sanitize_value_for_privacy,
};
use serde_json::Value;

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
            let passed = evaluate_assertion(&payload, assertion);
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
    let privacy_failures = count_privacy_failures(engine, privacy_mode, &payload);
    if privacy_failures > 0 {
        notes.push(format!("privacy_failures={privacy_failures}"));
    }
    let snapshot = evaluate_case_metrics(case, &payload);
    let variant_rank_consistent = concept_cluster_rank_consistent(engine, case, limit, &payload);
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
    let mut grouped = BTreeMap::new();
    for case in cases {
        grouped
            .entry(tool_label(case.tool).to_string())
            .or_insert_with(Vec::new)
            .push(case);
    }
    grouped
        .into_values()
        .filter_map(|cases| {
            let first = *cases.first()?;
            let case_count = cases.len();
            let passed_cases = cases.iter().filter(|case| case.pass).count();
            let unsupported_cases = cases
                .iter()
                .filter(|case| !case.unsupported_sources.is_empty())
                .count();
            let latencies = cases.iter().map(|case| case.latency_ms).collect::<Vec<_>>();
            Some(InvestigationToolMetrics {
                tool: first.tool,
                case_count,
                passed_cases,
                pass_rate: passed_cases as f32 / case_count as f32,
                unsupported_case_rate: unsupported_cases as f32 / case_count as f32,
                latency_p50_ms: percentile(&latencies, 50.0),
                latency_p95_ms: percentile(&latencies, 95.0),
                body_anchor_precision: any_positive(
                    cases.iter().map(|case| case.expected_body_anchor_count),
                )
                .then(|| {
                    ratio(
                        cases.iter().map(|case| case.matched_anchor_count).sum(),
                        cases.iter().map(|case| case.returned_anchor_count).sum(),
                    )
                })
                .flatten(),
                body_request_p95_budget_ms: None,
                body_request_p95_ratio: None,
                route_trace_success_at_1: bool_ratio(
                    cases.iter().filter_map(|case| case.route_success_at_1),
                ),
                route_trace_success_at_3: bool_ratio(
                    cases.iter().filter_map(|case| case.route_success_at_3),
                ),
                segment_type_precision: ratio(
                    cases
                        .iter()
                        .map(|case| case.correctly_typed_route_segment_count)
                        .sum(),
                    cases
                        .iter()
                        .map(|case| case.matched_route_segment_count)
                        .sum(),
                ),
                constraint_evidence_precision: any_positive(
                    cases
                        .iter()
                        .map(|case| case.expected_constraint_source_count),
                )
                .then(|| {
                    ratio(
                        cases.iter().map(|case| case.matched_constraint_count).sum(),
                        cases
                            .iter()
                            .map(|case| case.returned_constraint_count)
                            .sum(),
                    )
                })
                .flatten(),
                constraint_source_recall: ratio(
                    cases
                        .iter()
                        .map(|case| case.recovered_constraint_source_count)
                        .sum(),
                    cases
                        .iter()
                        .map(|case| case.expected_constraint_source_count)
                        .sum(),
                ),
                variant_recall_at_3: ratio(
                    cases
                        .iter()
                        .map(|case| case.recovered_variant_count_at_3)
                        .sum(),
                    cases.iter().map(|case| case.expected_variant_count).sum(),
                ),
                top_variant_precision: bool_ratio(
                    cases.iter().filter_map(|case| case.top_variant_match),
                ),
                variant_rank_consistency: bool_ratio(
                    cases.iter().filter_map(|case| case.variant_rank_consistent),
                ),
                semantic_state_coverage: bool_ratio(
                    cases
                        .iter()
                        .filter_map(|case| semantic_state_case_pass(case)),
                ),
                semantic_fail_open_visibility: bool_ratio(
                    cases
                        .iter()
                        .filter_map(|case| case.semantic_fail_open_visible),
                ),
                low_signal_semantic_false_penalty_rate: ratio(
                    cases
                        .iter()
                        .filter_map(|case| case.low_signal_semantic_false_penalty)
                        .filter(|flag| *flag)
                        .count(),
                    cases
                        .iter()
                        .filter(|case| case.low_signal_semantic_false_penalty.is_some())
                        .count(),
                ),
                divergence_signal_precision: any_positive(
                    cases
                        .iter()
                        .map(|case| case.expected_divergence_signal_count),
                )
                .then(|| {
                    ratio(
                        cases
                            .iter()
                            .map(|case| case.matched_divergence_signal_count)
                            .sum(),
                        cases
                            .iter()
                            .map(|case| case.returned_divergence_signal_count)
                            .sum(),
                    )
                })
                .flatten(),
                false_positive_divergence_rate: any_positive(
                    cases
                        .iter()
                        .map(|case| case.expected_divergence_signal_count),
                )
                .then(|| {
                    ratio(
                        cases
                            .iter()
                            .map(|case| case.unexpected_divergence_signal_count)
                            .sum(),
                        cases
                            .iter()
                            .map(|case| case.returned_divergence_signal_count)
                            .sum(),
                    )
                })
                .flatten(),
                explain_evidence_coverage: ratio(
                    cases.iter().map(|case| case.evidence_fields_present).sum(),
                    cases.iter().map(|case| case.evidence_fields_total).sum(),
                ),
            })
        })
        .collect()
}

pub(super) fn evaluate_thresholds(
    thresholds: &InvestigationThresholds,
    metrics: &[InvestigationToolMetrics],
    privacy_failures: usize,
) -> InvestigationThresholdVerdict {
    let mut failures = Vec::new();
    for metric in metrics {
        check_max(
            &mut failures,
            metric.tool,
            "latency_p95_ms",
            Some(metric.latency_p95_ms),
            Some(thresholds.max_latency_p95_ms),
        );
        check_max(
            &mut failures,
            metric.tool,
            "unsupported_case_rate",
            Some(metric.unsupported_case_rate),
            Some(thresholds.max_unsupported_case_rate),
        );
        let required_pass_rate = match metric.tool {
            InvestigationBenchmarkTool::SymbolBody => {
                Some(thresholds.symbol_body_supported_success)
            }
            InvestigationBenchmarkTool::RouteTrace => Some(thresholds.route_trace_case_pass_rate),
            InvestigationBenchmarkTool::ConstraintEvidence => {
                Some(thresholds.constraint_evidence_case_pass_rate)
            }
            InvestigationBenchmarkTool::ConceptCluster => {
                Some(thresholds.concept_cluster_case_pass_rate)
            }
            InvestigationBenchmarkTool::DivergenceReport => {
                Some(thresholds.divergence_case_pass_rate)
            }
        };
        check_min(
            &mut failures,
            metric.tool,
            "pass_rate",
            Some(metric.pass_rate),
            required_pass_rate,
        );
        if matches!(metric.tool, InvestigationBenchmarkTool::SymbolBody) {
            check_min(
                &mut failures,
                metric.tool,
                "body_anchor_precision",
                metric.body_anchor_precision,
                thresholds.body_anchor_precision_min,
            );
            check_max(
                &mut failures,
                metric.tool,
                "body_request_p95_ratio",
                metric.body_request_p95_ratio,
                thresholds.body_request_p95_ratio_max,
            );
        }
        if matches!(metric.tool, InvestigationBenchmarkTool::RouteTrace) {
            check_min(
                &mut failures,
                metric.tool,
                "route_trace_success_at_1",
                metric.route_trace_success_at_1,
                thresholds.route_trace_success_at_1_min,
            );
            check_min(
                &mut failures,
                metric.tool,
                "route_trace_success_at_3",
                metric.route_trace_success_at_3,
                thresholds.route_trace_success_at_3_min,
            );
            check_min(
                &mut failures,
                metric.tool,
                "segment_type_precision",
                metric.segment_type_precision,
                thresholds.segment_type_precision_min,
            );
        }
        if matches!(metric.tool, InvestigationBenchmarkTool::ConstraintEvidence) {
            check_min(
                &mut failures,
                metric.tool,
                "constraint_evidence_precision",
                metric.constraint_evidence_precision,
                thresholds.constraint_evidence_precision_min,
            );
            check_min(
                &mut failures,
                metric.tool,
                "constraint_source_recall",
                metric.constraint_source_recall,
                thresholds.constraint_source_recall_min,
            );
        }
        if matches!(metric.tool, InvestigationBenchmarkTool::ConceptCluster) {
            check_min(
                &mut failures,
                metric.tool,
                "top_variant_precision",
                metric.top_variant_precision,
                thresholds.top_variant_precision_min,
            );
            check_min(
                &mut failures,
                metric.tool,
                "variant_rank_consistency",
                metric.variant_rank_consistency,
                thresholds.variant_rank_consistency_min,
            );
            check_min(
                &mut failures,
                metric.tool,
                "semantic_state_coverage",
                metric.semantic_state_coverage,
                thresholds.semantic_state_coverage_min,
            );
            check_min(
                &mut failures,
                metric.tool,
                "semantic_fail_open_visibility",
                metric.semantic_fail_open_visibility,
                thresholds.semantic_fail_open_visibility_min,
            );
            check_max(
                &mut failures,
                metric.tool,
                "low_signal_semantic_false_penalty_rate",
                metric.low_signal_semantic_false_penalty_rate,
                thresholds.low_signal_semantic_false_penalty_rate_max,
            );
        }
        if matches!(
            metric.tool,
            InvestigationBenchmarkTool::ConceptCluster
                | InvestigationBenchmarkTool::DivergenceReport
        ) {
            check_min(
                &mut failures,
                metric.tool,
                "variant_recall_at_3",
                metric.variant_recall_at_3,
                thresholds.variant_recall_at_3_min,
            );
        }
        if matches!(metric.tool, InvestigationBenchmarkTool::DivergenceReport) {
            check_min(
                &mut failures,
                metric.tool,
                "divergence_signal_precision",
                metric.divergence_signal_precision,
                thresholds.divergence_signal_precision_min,
            );
            check_max(
                &mut failures,
                metric.tool,
                "false_positive_divergence_rate",
                metric.false_positive_divergence_rate,
                thresholds.false_positive_divergence_rate_max,
            );
        }
        check_min(
            &mut failures,
            metric.tool,
            "explain_evidence_coverage",
            metric.explain_evidence_coverage,
            thresholds.explain_evidence_coverage_min,
        );
    }
    if privacy_failures > thresholds.privacy_failures {
        failures.push(format!(
            "privacy_failures {} > {}",
            privacy_failures, thresholds.privacy_failures
        ));
    }
    InvestigationThresholdVerdict {
        passed: failures.is_empty(),
        failures,
    }
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

fn run_tool(
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

fn evaluate_assertion(payload: &Value, assertion: &InvestigationAssertion) -> bool {
    match assertion.kind.as_str() {
        "body_anchor_present" => {
            payload["items"]
                .as_array()
                .is_some_and(|items| items.iter().any(|item| item.get("anchor").is_some()))
                || payload["variants"].as_array().is_some_and(|variants| {
                    variants
                        .iter()
                        .any(|variant| !variant["body_anchor"].is_null())
                })
        }
        "route_kind_present" | "contains_route_kind" => collect_route_strings(payload, "kind")
            .iter()
            .any(|value| *value == assertion.value.as_deref().unwrap_or_default()),
        "relation_kind_present" => collect_route_strings(payload, "relation_kind")
            .iter()
            .any(|value| *value == assertion.value.as_deref().unwrap_or_default()),
        "source_span_present" => route_segments(payload)
            .into_iter()
            .any(|segment| !segment["source_span"].is_null()),
        "constraint_kind_present" | "contains_constraint_kind" => {
            collect_constraint_strings(payload, "kind")
                .iter()
                .any(|value| *value == assertion.value.as_deref().unwrap_or_default())
        }
        "contains_language" => collect_languages(payload)
            .iter()
            .any(|value| *value == assertion.value.as_deref().unwrap_or_default()),
        "contains_gap" => collect_gaps(payload)
            .iter()
            .any(|value| *value == assertion.value.as_deref().unwrap_or_default()),
        "min_variant_count" => payload["variants"]
            .as_array()
            .is_some_and(|variants| variants.len() >= parse_usize_assertion(assertion)),
        "min_body_items" => payload["items"]
            .as_array()
            .is_some_and(|items| items.len() >= parse_usize_assertion(assertion)),
        "min_divergence_axes" => payload["divergence_axes"]
            .as_array()
            .is_some_and(|axes| axes.len() >= parse_usize_assertion(assertion)),
        "strong_constraint_present" => {
            collect_constraint_strings(payload, "strength").contains(&"strong")
        }
        "divergence_axis_present" => payload["divergence_axes"].as_array().is_some_and(|axes| {
            axes.iter()
                .any(|axis| axis["axis"].as_str() == assertion.value.as_deref())
        }),
        "divergence_severity_present" | "expected_severity" => payload["divergence_signals"]
            .as_array()
            .is_some_and(|signals| {
                signals
                    .iter()
                    .any(|signal| signal["severity"].as_str() == assertion.value.as_deref())
            }),
        "manual_review_required" => payload["manual_review_required"]
            .as_bool()
            .is_some_and(|value| value == parse_bool_assertion(assertion)),
        _ => false,
    }
}

fn collect_route_strings<'a>(payload: &'a Value, key: &str) -> Vec<&'a str> {
    route_segments(payload)
        .into_iter()
        .filter_map(|entry| entry[key].as_str())
        .collect()
}

fn route_segments(payload: &Value) -> Vec<&Value> {
    if payload.get("best_route").is_some() {
        payload["best_route"]["segments"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(
                payload["alternate_routes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(|route| route["segments"].as_array().into_iter().flatten()),
            )
            .collect()
    } else {
        payload["variants"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|item| item["route"].as_array())
            .flatten()
            .collect()
    }
}
fn collect_constraint_strings<'a>(payload: &'a Value, key: &str) -> Vec<&'a str> {
    payload["items"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(
            payload["variants"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|variant| variant["constraints"].as_array())
                .flatten(),
        )
        .filter_map(|entry| entry[key].as_str())
        .collect()
}
fn collect_languages(payload: &Value) -> Vec<&str> {
    payload["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["anchor"]["language"].as_str())
        .chain(
            payload["variants"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|variant| variant["entry_anchor"]["language"].as_str()),
        )
        .collect()
}
fn collect_gaps(payload: &Value) -> Vec<&str> {
    payload["gaps"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .chain(
            payload["unresolved_gaps"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|gap| gap["reason"].as_str()),
        )
        .chain(
            payload["variants"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|variant| variant["gaps"].as_array())
                .flatten()
                .filter_map(Value::as_str),
        )
        .collect()
}
fn parse_usize_assertion(assertion: &InvestigationAssertion) -> usize {
    assertion
        .value
        .as_deref()
        .unwrap_or("0")
        .parse::<usize>()
        .unwrap_or(0)
}

fn parse_bool_assertion(assertion: &InvestigationAssertion) -> bool {
    matches!(
        assertion.value.as_deref().unwrap_or_default().trim(),
        "true" | "True" | "TRUE" | "1"
    )
}

fn concept_cluster_rank_consistent(
    engine: &Engine,
    case: &InvestigationBenchmarkCase,
    limit: usize,
    payload: &Value,
) -> Option<bool> {
    if !matches!(case.tool, InvestigationBenchmarkTool::ConceptCluster) {
        return None;
    }
    let rerun_payload = run_tool(engine, case.tool, &case.seed, case.seed_kind, limit).ok()?;
    Some(top_variant_paths(payload) == top_variant_paths(&rerun_payload))
}

fn top_variant_paths(payload: &Value) -> Vec<String> {
    payload["variants"]
        .as_array()
        .into_iter()
        .flatten()
        .take(3)
        .filter_map(|variant| variant["entry_anchor"]["path"].as_str())
        .map(ToString::to_string)
        .collect()
}

fn semantic_state_case_pass(case: &InvestigationCaseReport) -> Option<bool> {
    case.semantic_state_matches_expectation
        .or(case.semantic_state_present)
}

fn count_privacy_failures(engine: &Engine, privacy_mode: PrivacyMode, payload: &Value) -> usize {
    if matches!(privacy_mode, PrivacyMode::Off) {
        return 0;
    }
    let mut sanitized = payload.clone();
    sanitize_value_for_privacy(privacy_mode, &mut sanitized);
    let serialized = serde_json::to_string(&sanitized).unwrap_or_default();
    usize::from(serialized.contains(&engine.project_root.display().to_string()))
}

fn ratio(numerator: usize, denominator: usize) -> Option<f32> {
    (denominator > 0).then(|| numerator as f32 / denominator as f32)
}
fn bool_ratio(values: impl Iterator<Item = bool>) -> Option<f32> {
    let (trues, total) = values.fold((0_usize, 0_usize), |(trues, total), value| {
        (trues + usize::from(value), total + 1)
    });
    ratio(trues, total)
}
fn any_positive(mut values: impl Iterator<Item = usize>) -> bool {
    values.any(|value| value > 0)
}
fn percentile(values: &[f32], percentile: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let max_index = sorted.len().saturating_sub(1);
    let rank = ((percentile / 100.0) * max_index as f32).round() as usize;
    sorted[rank.min(max_index)]
}
fn check_min(
    failures: &mut Vec<String>,
    tool: InvestigationBenchmarkTool,
    name: &str,
    actual: Option<f32>,
    expected: Option<f32>,
) {
    match (actual, expected) {
        (Some(actual), Some(expected)) if actual < expected => failures.push(format!(
            "{} {} {:.2} < {:.2}",
            tool_label(tool),
            name,
            actual,
            expected
        )),
        (None, Some(_)) => failures.push(format!("{} {} missing", tool_label(tool), name)),
        _ => {}
    }
}
fn check_max<T>(
    failures: &mut Vec<String>,
    tool: InvestigationBenchmarkTool,
    name: &str,
    actual: Option<T>,
    expected: Option<T>,
) where
    T: Copy + PartialOrd + Display,
{
    match (actual, expected) {
        (Some(actual), Some(expected)) if actual > expected => failures.push(format!(
            "{} {} {} > {}",
            tool_label(tool),
            name,
            actual,
            expected
        )),
        (None, Some(_)) => failures.push(format!("{} {} missing", tool_label(tool), name)),
        _ => {}
    }
}
