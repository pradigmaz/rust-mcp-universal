use crate::model::{QueryBenchmarkDiffReport, QueryBenchmarkMetricDiff, QueryBenchmarkReport};

pub(super) fn build_diff_report(
    baseline: &QueryBenchmarkReport,
    candidate: &QueryBenchmarkReport,
) -> QueryBenchmarkDiffReport {
    QueryBenchmarkDiffReport {
        recall_at_k: metric_diff(baseline.recall_at_k, candidate.recall_at_k),
        mrr_at_k: metric_diff(baseline.mrr_at_k, candidate.mrr_at_k),
        ndcg_at_k: metric_diff(baseline.ndcg_at_k, candidate.ndcg_at_k),
        avg_estimated_tokens: metric_diff(
            baseline.avg_estimated_tokens,
            candidate.avg_estimated_tokens,
        ),
        latency_p50_ms: metric_diff(baseline.latency_p50_ms, candidate.latency_p50_ms),
        latency_p95_ms: metric_diff(baseline.latency_p95_ms, candidate.latency_p95_ms),
    }
}

fn metric_diff(baseline: f32, candidate: f32) -> QueryBenchmarkMetricDiff {
    let delta = candidate - baseline;
    let delta_ratio = if baseline.abs() <= f32::EPSILON {
        if delta.abs() <= f32::EPSILON {
            Some(0.0)
        } else {
            None
        }
    } else {
        Some(delta / baseline)
    };

    QueryBenchmarkMetricDiff {
        baseline,
        candidate,
        delta,
        delta_ratio,
    }
}
