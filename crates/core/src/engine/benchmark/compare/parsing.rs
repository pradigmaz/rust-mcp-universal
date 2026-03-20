use crate::model::QueryBenchmarkReport;

pub(super) fn empty_report() -> QueryBenchmarkReport {
    QueryBenchmarkReport {
        dataset_path: String::new(),
        k: 1,
        query_count: 0,
        recall_at_k: 0.0,
        mrr_at_k: 0.0,
        ndcg_at_k: 0.0,
        avg_estimated_tokens: 0.0,
        latency_p50_ms: 0.0,
        latency_p95_ms: 0.0,
    }
}
