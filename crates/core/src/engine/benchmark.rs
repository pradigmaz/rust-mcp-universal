use std::{path::Path, time::Instant};

use anyhow::{Result, bail};

use super::Engine;
use crate::model::{
    QueryBenchmarkComparisonOptions, QueryBenchmarkComparisonReport, QueryBenchmarkDataset,
    QueryBenchmarkMultiRunReport, QueryBenchmarkOptions, QueryBenchmarkReport, QueryOptions,
};

#[path = "benchmark/compare.rs"]
mod compare;
#[path = "benchmark/dataset.rs"]
mod dataset;
#[path = "benchmark/metrics.rs"]
mod metrics;

impl Engine {
    pub fn query_benchmark(
        &self,
        dataset_path: &Path,
        options: QueryBenchmarkOptions,
    ) -> Result<QueryBenchmarkReport> {
        self.query_benchmark_with_auto_index(
            dataset_path,
            QueryBenchmarkOptions {
                auto_index: true,
                ..options
            },
        )
    }

    pub fn query_benchmark_with_auto_index(
        &self,
        dataset_path: &Path,
        options: QueryBenchmarkOptions,
    ) -> Result<QueryBenchmarkReport> {
        let _ = self.ensure_index_ready_with_policy(options.auto_index)?;
        let dataset_path_string = dataset_path.display().to_string();
        let dataset = dataset::load_benchmark_dataset(dataset_path)?;
        self.run_query_benchmark(&dataset_path_string, &dataset, options)
    }

    pub fn query_benchmark_baseline_vs_candidate(
        &self,
        dataset_path: &Path,
        options: QueryBenchmarkComparisonOptions,
    ) -> Result<QueryBenchmarkComparisonReport> {
        let baseline_k = options.baseline.k.max(1);
        let candidate_k = options.candidate.k.max(1);
        if baseline_k != candidate_k {
            bail!(
                "baseline-vs-candidate requires identical `k` values, got baseline={} and candidate={}",
                baseline_k,
                candidate_k
            );
        }

        let auto_index = options.baseline.auto_index || options.candidate.auto_index;
        let _ = self.ensure_index_ready_with_policy(auto_index)?;
        let runs = options.runs.max(1);
        let dataset_path_string = dataset_path.display().to_string();
        let dataset = dataset::load_benchmark_dataset(dataset_path)?;

        let baseline_options = QueryBenchmarkOptions {
            auto_index: false,
            ..options.baseline
        };
        let candidate_options = QueryBenchmarkOptions {
            auto_index: false,
            ..options.candidate
        };

        let baseline_runs =
            self.run_query_benchmark_runs(&dataset_path_string, &dataset, baseline_options, runs)?;
        let candidate_runs =
            self.run_query_benchmark_runs(&dataset_path_string, &dataset, candidate_options, runs)?;
        let baseline_median = compare::median_report(&baseline_runs);
        let candidate_median = compare::median_report(&candidate_runs);

        let diff = compare::build_diff_report(&baseline_median, &candidate_median);
        let gates = compare::evaluate_gates(
            &baseline_median,
            &candidate_median,
            options.gate_thresholds,
            options.fail_fast,
        );

        Ok(QueryBenchmarkComparisonReport {
            dataset_path: dataset_path_string,
            runs_count: runs,
            median_rule: format!("median_of_{}_runs", runs),
            baseline: QueryBenchmarkMultiRunReport {
                runs: baseline_runs,
                median: baseline_median,
            },
            candidate: QueryBenchmarkMultiRunReport {
                runs: candidate_runs,
                median: candidate_median,
            },
            diff,
            gates,
        })
    }

    fn run_query_benchmark_runs(
        &self,
        dataset_path: &str,
        dataset: &QueryBenchmarkDataset,
        options: QueryBenchmarkOptions,
        runs: usize,
    ) -> Result<Vec<QueryBenchmarkReport>> {
        let mut reports = Vec::with_capacity(runs);
        for _ in 0..runs {
            reports.push(self.run_query_benchmark(dataset_path, dataset, options)?);
        }
        Ok(reports)
    }

    fn run_query_benchmark(
        &self,
        dataset_path: &str,
        dataset: &QueryBenchmarkDataset,
        options: QueryBenchmarkOptions,
    ) -> Result<QueryBenchmarkReport> {
        let k = options.k.max(1);
        let limit = options.limit.max(1);
        let query_count = dataset.queries.len();
        if query_count == 0 {
            return Ok(QueryBenchmarkReport {
                dataset_path: dataset_path.to_string(),
                k,
                query_count: 0,
                recall_at_k: 0.0,
                mrr_at_k: 0.0,
                ndcg_at_k: 0.0,
                avg_estimated_tokens: 0.0,
                latency_p50_ms: 0.0,
                latency_p95_ms: 0.0,
            });
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
            let execution = self.search_with_meta(&query_options)?;
            let elapsed_ms = started.elapsed().as_secs_f32() * 1000.0;
            latencies_ms.push(elapsed_ms);

            let context = self.context_for_hits_with_chunks(
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
}
