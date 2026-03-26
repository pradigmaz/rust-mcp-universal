use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::Result;
use rmu_core::{
    InvestigationBenchmarkDiffReport, InvestigationBenchmarkReport, InvestigationBenchmarkTool,
    InvestigationMetricChange, InvestigationToolMetricDelta,
};

use super::investigation_benchmark_eval::tool_label;

const METRIC_EPSILON: f32 = 0.000_001;

#[derive(Clone, Copy)]
enum MetricExpectation {
    HigherIsBetter,
    LowerIsBetter,
}

impl MetricExpectation {
    fn as_str(self) -> &'static str {
        match self {
            Self::HigherIsBetter => "higher_is_better",
            Self::LowerIsBetter => "lower_is_better",
        }
    }

    fn classify_change(self, metric: &str, baseline: f32, current: f32) -> MetricChangeKind {
        let delta = current - baseline;
        let tolerance = metric_tolerance(metric, baseline);
        match self {
            Self::HigherIsBetter if delta < -(METRIC_EPSILON + tolerance) => {
                MetricChangeKind::Regression
            }
            Self::HigherIsBetter if delta > METRIC_EPSILON + tolerance => {
                MetricChangeKind::Improvement
            }
            Self::LowerIsBetter if delta > METRIC_EPSILON + tolerance => {
                MetricChangeKind::Regression
            }
            Self::LowerIsBetter if delta < -(METRIC_EPSILON + tolerance) => {
                MetricChangeKind::Improvement
            }
            _ => MetricChangeKind::Stable,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MetricChangeKind {
    Regression,
    Improvement,
    Stable,
}

pub(super) fn load_baseline_report(path: &Path) -> Result<InvestigationBenchmarkReport> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(Into::into)
}

pub(super) fn build_diff_report(
    baseline: &InvestigationBenchmarkReport,
    current: &InvestigationBenchmarkReport,
) -> InvestigationBenchmarkDiffReport {
    let baseline_metrics = metrics_by_tool(&baseline.per_tool_metrics);
    let current_metrics = metrics_by_tool(&current.per_tool_metrics);

    let mut per_tool_deltas = Vec::new();
    let mut regressed_metrics = Vec::new();
    let mut improved_metrics = Vec::new();
    let mut regression_failures = Vec::new();

    for tool in ordered_tools() {
        match (baseline_metrics.get(&tool), current_metrics.get(&tool)) {
            (Some(baseline_tool), Some(current_tool)) => {
                let mut metrics = Vec::new();
                for (metric, baseline_value, current_value, expectation) in
                    metric_samples(baseline_tool, current_tool)
                {
                    let Some(baseline_value) = baseline_value else {
                        continue;
                    };
                    let Some(current_value) = current_value else {
                        regression_failures.push(format!(
                            "{} missing metric {} in current report",
                            tool_label(tool),
                            metric
                        ));
                        continue;
                    };
                    let change = InvestigationMetricChange {
                        tool,
                        metric: metric.to_string(),
                        expectation: expectation.as_str().to_string(),
                        baseline: baseline_value,
                        current: current_value,
                        delta: current_value - baseline_value,
                        delta_ratio: diff_ratio(baseline_value, current_value),
                    };
                    match expectation.classify_change(metric, baseline_value, current_value) {
                        MetricChangeKind::Regression => {
                            regression_failures.push(format!(
                                "{} {} regressed: {:.6} -> {:.6}",
                                tool_label(tool),
                                metric,
                                baseline_value,
                                current_value
                            ));
                            regressed_metrics.push(change.clone());
                        }
                        MetricChangeKind::Improvement => improved_metrics.push(change.clone()),
                        MetricChangeKind::Stable => {}
                    }
                    metrics.push(change);
                }
                per_tool_deltas.push(InvestigationToolMetricDelta { tool, metrics });
            }
            (Some(_), None) => regression_failures.push(format!(
                "current report is missing tool {} present in baseline",
                tool_label(tool)
            )),
            (None, Some(_)) | (None, None) => {}
        }
    }

    compare_case_sets(baseline, current, &mut regression_failures);
    if current.privacy_failures > baseline.privacy_failures {
        regression_failures.push(format!(
            "privacy_failures regressed: {} -> {}",
            baseline.privacy_failures, current.privacy_failures
        ));
    }

    InvestigationBenchmarkDiffReport {
        baseline_case_count: baseline.case_count,
        current_case_count: current.case_count,
        per_tool_deltas,
        regressed_metrics,
        improved_metrics,
        regression_failures,
    }
}

fn metric_tolerance(metric: &str, baseline: f32) -> f32 {
    match metric {
        "latency_p50_ms" | "latency_p95_ms" => baseline.abs().mul_add(0.15, 0.0).max(25.0),
        "body_request_p95_ratio" => baseline.abs().mul_add(0.20, 0.0).max(1.0),
        _ => 0.0,
    }
}

fn metrics_by_tool(
    metrics: &[rmu_core::InvestigationToolMetrics],
) -> BTreeMap<InvestigationBenchmarkTool, &rmu_core::InvestigationToolMetrics> {
    metrics.iter().map(|metric| (metric.tool, metric)).collect()
}

fn ordered_tools() -> [InvestigationBenchmarkTool; 5] {
    [
        InvestigationBenchmarkTool::SymbolBody,
        InvestigationBenchmarkTool::RouteTrace,
        InvestigationBenchmarkTool::ConstraintEvidence,
        InvestigationBenchmarkTool::ConceptCluster,
        InvestigationBenchmarkTool::DivergenceReport,
    ]
}

fn metric_samples(
    baseline: &rmu_core::InvestigationToolMetrics,
    current: &rmu_core::InvestigationToolMetrics,
) -> [(&'static str, Option<f32>, Option<f32>, MetricExpectation); 20] {
    [
        (
            "pass_rate",
            Some(baseline.pass_rate),
            Some(current.pass_rate),
            MetricExpectation::HigherIsBetter,
        ),
        (
            "unsupported_case_rate",
            Some(baseline.unsupported_case_rate),
            Some(current.unsupported_case_rate),
            MetricExpectation::LowerIsBetter,
        ),
        (
            "latency_p50_ms",
            Some(baseline.latency_p50_ms),
            Some(current.latency_p50_ms),
            MetricExpectation::LowerIsBetter,
        ),
        (
            "latency_p95_ms",
            Some(baseline.latency_p95_ms),
            Some(current.latency_p95_ms),
            MetricExpectation::LowerIsBetter,
        ),
        (
            "body_anchor_precision",
            baseline.body_anchor_precision,
            current.body_anchor_precision,
            MetricExpectation::HigherIsBetter,
        ),
        (
            "body_request_p95_ratio",
            baseline.body_request_p95_ratio,
            current.body_request_p95_ratio,
            MetricExpectation::LowerIsBetter,
        ),
        (
            "route_trace_success_at_1",
            baseline.route_trace_success_at_1,
            current.route_trace_success_at_1,
            MetricExpectation::HigherIsBetter,
        ),
        (
            "route_trace_success_at_3",
            baseline.route_trace_success_at_3,
            current.route_trace_success_at_3,
            MetricExpectation::HigherIsBetter,
        ),
        (
            "segment_type_precision",
            baseline.segment_type_precision,
            current.segment_type_precision,
            MetricExpectation::HigherIsBetter,
        ),
        (
            "constraint_evidence_precision",
            baseline.constraint_evidence_precision,
            current.constraint_evidence_precision,
            MetricExpectation::HigherIsBetter,
        ),
        (
            "constraint_source_recall",
            baseline.constraint_source_recall,
            current.constraint_source_recall,
            MetricExpectation::HigherIsBetter,
        ),
        (
            "variant_recall_at_3",
            baseline.variant_recall_at_3,
            current.variant_recall_at_3,
            MetricExpectation::HigherIsBetter,
        ),
        (
            "top_variant_precision",
            baseline.top_variant_precision,
            current.top_variant_precision,
            MetricExpectation::HigherIsBetter,
        ),
        (
            "variant_rank_consistency",
            baseline.variant_rank_consistency,
            current.variant_rank_consistency,
            MetricExpectation::HigherIsBetter,
        ),
        (
            "semantic_state_coverage",
            baseline.semantic_state_coverage,
            current.semantic_state_coverage,
            MetricExpectation::HigherIsBetter,
        ),
        (
            "semantic_fail_open_visibility",
            baseline.semantic_fail_open_visibility,
            current.semantic_fail_open_visibility,
            MetricExpectation::HigherIsBetter,
        ),
        (
            "low_signal_semantic_false_penalty_rate",
            baseline.low_signal_semantic_false_penalty_rate,
            current.low_signal_semantic_false_penalty_rate,
            MetricExpectation::LowerIsBetter,
        ),
        (
            "divergence_signal_precision",
            baseline.divergence_signal_precision,
            current.divergence_signal_precision,
            MetricExpectation::HigherIsBetter,
        ),
        (
            "false_positive_divergence_rate",
            baseline.false_positive_divergence_rate,
            current.false_positive_divergence_rate,
            MetricExpectation::LowerIsBetter,
        ),
        (
            "explain_evidence_coverage",
            baseline.explain_evidence_coverage,
            current.explain_evidence_coverage,
            MetricExpectation::HigherIsBetter,
        ),
    ]
}

fn compare_case_sets(
    baseline: &InvestigationBenchmarkReport,
    current: &InvestigationBenchmarkReport,
    regression_failures: &mut Vec<String>,
) {
    let baseline_ids = baseline
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    let current_ids = current
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    if baseline.case_count != current.case_count {
        regression_failures.push(format!(
            "case_count mismatch: {} -> {}",
            baseline.case_count, current.case_count
        ));
    }
    if baseline_ids != current_ids {
        let removed = baseline_ids
            .difference(&current_ids)
            .copied()
            .collect::<Vec<_>>();
        let added = current_ids
            .difference(&baseline_ids)
            .copied()
            .collect::<Vec<_>>();
        regression_failures.push(format!(
            "case_id set changed: removed=[{}], added=[{}]",
            removed.join("|"),
            added.join("|")
        ));
    }
    let baseline_status = baseline
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case.expected_capability_status.as_str()))
        .collect::<BTreeMap<_, _>>();
    for case in &current.cases {
        if let Some(expected_status) = baseline_status.get(case.id.as_str())
            && *expected_status != case.expected_capability_status.as_str()
        {
            regression_failures.push(format!(
                "case {} expected_capability_status changed: {} -> {}",
                case.id, expected_status, case.expected_capability_status
            ));
        }
    }
}

fn diff_ratio(baseline: f32, current: f32) -> Option<f32> {
    if baseline.abs() <= f32::EPSILON {
        if (current - baseline).abs() <= f32::EPSILON {
            Some(0.0)
        } else {
            None
        }
    } else {
        Some((current - baseline) / baseline)
    }
}
