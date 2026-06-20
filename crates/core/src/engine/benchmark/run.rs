use std::time::Instant;

use anyhow::Result;

use super::metrics;
use crate::engine::Engine;
use crate::model::{
    QueryBenchmarkDataset, QueryBenchmarkOptions, QueryBenchmarkReport, QueryOptions,
};

pub(super) fn run_query_benchmark_runs(
    engine: &Engine,
    dataset_path: &str,
    dataset: &QueryBenchmarkDataset,
    options: QueryBenchmarkOptions,
    runs: usize,
) -> Result<Vec<QueryBenchmarkReport>> {
    let mut reports = Vec::with_capacity(runs);
    for _ in 0..runs {
        reports.push(run_query_benchmark(engine, dataset_path, dataset, options)?);
    }
    Ok(reports)
}

pub(super) fn run_query_benchmark(
    engine: &Engine,
    dataset_path: &str,
    dataset: &QueryBenchmarkDataset,
    options: QueryBenchmarkOptions,
) -> Result<QueryBenchmarkReport> {
    let k = options.k.max(1);
    let limit = options.limit.max(1);
    let query_count = dataset.queries.len();
    if query_count == 0 {
        return Ok(empty_report(dataset_path, k));
    }

    let mut recall_sum = 0.0_f32;
    let mut mrr_sum = 0.0_f32;
    let mut ndcg_sum = 0.0_f32;
    let mut token_sum = 0.0_f32;
    let mut latencies_ms = Vec::with_capacity(query_count);

    for case in &dataset.queries {
        let query_options = QueryOptions {
            query: case.query.clone(),
            limit,
            detailed: false,
            semantic: options.semantic,
            semantic_fail_mode: options.semantic_fail_mode,
            privacy_mode: options.privacy_mode,
            context_mode: None,
            agent_intent_mode: None,
        };

        let started = Instant::now();
        let execution = engine.search_with_meta(&query_options)?;
        latencies_ms.push(started.elapsed().as_secs_f32() * 1000.0);

        let context = engine.context_for_hits_with_chunks(
            &case.query,
            &execution.hits,
            Some(&execution.chunk_by_path),
            None,
            options.max_chars,
            options.max_tokens,
        )?;
        token_sum += context.estimated_tokens as f32;

        recall_sum += metrics::recall_at_k(&execution.hits, &case.qrels, k);
        mrr_sum += metrics::mrr_at_k(&execution.hits, &case.qrels, k);
        ndcg_sum += metrics::ndcg_at_k(&execution.hits, &case.qrels, k);
    }

    Ok(QueryBenchmarkReport {
        dataset_path: dataset_path.to_string(),
        k,
        query_count,
        recall_at_k: recall_sum / query_count as f32,
        mrr_at_k: mrr_sum / query_count as f32,
        ndcg_at_k: ndcg_sum / query_count as f32,
        avg_estimated_tokens: token_sum / query_count as f32,
        latency_p50_ms: metrics::percentile(&latencies_ms, 50.0),
        latency_p95_ms: metrics::percentile(&latencies_ms, 95.0),
    })
}

fn empty_report(dataset_path: &str, k: usize) -> QueryBenchmarkReport {
    QueryBenchmarkReport {
        dataset_path: dataset_path.to_string(),
        k,
        query_count: 0,
        recall_at_k: 0.0,
        mrr_at_k: 0.0,
        ndcg_at_k: 0.0,
        avg_estimated_tokens: 0.0,
        latency_p50_ms: 0.0,
        latency_p95_ms: 0.0,
    }
}
