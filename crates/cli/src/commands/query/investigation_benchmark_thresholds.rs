use std::fmt::Display;

use rmu_core::{
    InvestigationBenchmarkTool, InvestigationThresholdVerdict, InvestigationThresholds,
    InvestigationToolMetrics,
};

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
            InvestigationBenchmarkTool::ContractTrace => {
                Some(thresholds.contract_trace_case_pass_rate)
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
            super::tool_label(tool),
            name,
            actual,
            expected
        )),
        (None, Some(_)) => failures.push(format!("{} {} missing", super::tool_label(tool), name)),
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
            super::tool_label(tool),
            name,
            actual,
            expected
        )),
        (None, Some(_)) => failures.push(format!("{} {} missing", super::tool_label(tool), name)),
        _ => {}
    }
}
