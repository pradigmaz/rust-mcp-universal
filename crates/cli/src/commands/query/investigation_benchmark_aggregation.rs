use std::collections::BTreeMap;

use rmu_core::{InvestigationCaseReport, InvestigationToolMetrics};

pub(super) fn build_tool_metrics(
    cases: &[InvestigationCaseReport],
) -> Vec<InvestigationToolMetrics> {
    let mut grouped = BTreeMap::new();
    for case in cases {
        grouped
            .entry(super::tool_label(case.tool).to_string())
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

fn semantic_state_case_pass(case: &InvestigationCaseReport) -> Option<bool> {
    case.semantic_state_matches_expectation
        .or(case.semantic_state_present)
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
