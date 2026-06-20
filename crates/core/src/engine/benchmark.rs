use std::path::Path;

use anyhow::{Result, bail};

use super::Engine;
use crate::model::{
    QueryBenchmarkComparisonOptions, QueryBenchmarkComparisonReport, QueryBenchmarkMultiRunReport,
    QueryBenchmarkOptions, QueryBenchmarkReport,
};

#[path = "benchmark/compare.rs"]
mod compare;
#[path = "benchmark/dataset.rs"]
mod dataset;
#[path = "benchmark/metrics.rs"]
mod metrics;
#[path = "benchmark/run.rs"]
mod run;

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
        run::run_query_benchmark(self, &dataset_path_string, &dataset, options)
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

        let baseline_runs = run::run_query_benchmark_runs(
            self,
            &dataset_path_string,
            &dataset,
            baseline_options,
            runs,
        )?;
        let candidate_runs = run::run_query_benchmark_runs(
            self,
            &dataset_path_string,
            &dataset,
            candidate_options,
            runs,
        )?;
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
}
