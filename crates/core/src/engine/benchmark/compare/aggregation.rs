use crate::model::QueryBenchmarkReport;

use super::parsing;

pub(super) fn median_report(runs: &[QueryBenchmarkReport]) -> QueryBenchmarkReport {
    if runs.is_empty() {
        return parsing::empty_report();
    }

    let first = &runs[0];
    QueryBenchmarkReport {
        dataset_path: first.dataset_path.clone(),
        k: first.k,
        query_count: first.query_count,
        recall_at_k: median_metric(runs, |report| report.recall_at_k),
        mrr_at_k: median_metric(runs, |report| report.mrr_at_k),
        ndcg_at_k: median_metric(runs, |report| report.ndcg_at_k),
        avg_estimated_tokens: median_metric(runs, |report| report.avg_estimated_tokens),
        latency_p50_ms: median_metric(runs, |report| report.latency_p50_ms),
        latency_p95_ms: median_metric(runs, |report| report.latency_p95_ms),
    }
}

fn median_metric(
    runs: &[QueryBenchmarkReport],
    selector: impl Fn(&QueryBenchmarkReport) -> f32,
) -> f32 {
    let values = runs.iter().map(selector).collect::<Vec<_>>();
    super::super::metrics::percentile(&values, 50.0)
}
