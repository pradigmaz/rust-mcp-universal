use crate::model::{
    QueryBenchmarkDiffReport, QueryBenchmarkGateEvaluation, QueryBenchmarkGateThresholds,
    QueryBenchmarkReport,
};

#[path = "compare/aggregation.rs"]
mod aggregation;
#[path = "compare/gates.rs"]
mod gates;
#[path = "compare/output.rs"]
mod output;
#[path = "compare/parsing.rs"]
mod parsing;

pub(super) fn median_report(runs: &[QueryBenchmarkReport]) -> QueryBenchmarkReport {
    aggregation::median_report(runs)
}

pub(super) fn build_diff_report(
    baseline: &QueryBenchmarkReport,
    candidate: &QueryBenchmarkReport,
) -> QueryBenchmarkDiffReport {
    output::build_diff_report(baseline, candidate)
}

pub(super) fn evaluate_gates(
    baseline: &QueryBenchmarkReport,
    candidate: &QueryBenchmarkReport,
    thresholds: QueryBenchmarkGateThresholds,
    fail_fast: bool,
) -> QueryBenchmarkGateEvaluation {
    gates::evaluate_gates(baseline, candidate, thresholds, fail_fast)
}

#[cfg(test)]
#[path = "compare/tests.rs"]
mod tests;
