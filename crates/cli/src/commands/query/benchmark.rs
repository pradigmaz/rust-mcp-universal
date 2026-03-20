use super::*;
use rmu_core::{
    BenchmarkMetrics, GateEvaluation, build_benchmark_diff_payload, build_metrics_diff,
    load_baseline_metrics, load_thresholds, median_metrics_from_runs,
};

pub(crate) fn run_query_benchmark(
    engine: &Engine,
    json: bool,
    args: QueryBenchmarkArgs,
) -> Result<()> {
    let QueryBenchmarkArgs {
        dataset,
        k,
        limit,
        semantic,
        auto_index,
        semantic_fail_mode,
        privacy_mode,
        vector_layer_enabled,
        rollout_phase,
        migration_mode,
        max_chars,
        max_tokens,
        baseline,
        thresholds,
        runs,
        enforce_gates,
    } = args;
    let k = require_min("k", k, 1)?;
    let limit = require_min("limit", limit, 1)?;
    let max_chars = require_min("max_chars", max_chars, 256)?;
    let max_tokens = require_min("max_tokens", max_tokens, 64)?;
    let runs = require_min("runs", runs, 1)?;
    let benchmark_semantic =
        semantic && vector_layer_enabled && !matches!(rollout_phase, RolloutPhase::Shadow);
    let baseline_mode_requested =
        baseline.is_some() || thresholds.is_some() || runs > 1 || enforce_gates;
    if !baseline_mode_requested {
        let report = engine.query_benchmark_with_auto_index(
            &dataset,
            QueryBenchmarkOptions::new(k, limit, benchmark_semantic, max_chars, max_tokens)
                .with_auto_index(auto_index)
                .with_semantic_fail_mode(semantic_fail_mode)
                .with_privacy_mode(privacy_mode),
        )?;

        if json {
            let mut payload = serde_json::to_value(&report)?;
            sanitize_value_for_privacy(privacy_mode, &mut payload);
            print_json(serde_json::to_string_pretty(&payload))?;
        } else {
            print_line(format!(
                "dataset={}, query_count={}, k={}",
                sanitize_path_text(privacy_mode, &report.dataset_path),
                report.query_count,
                report.k
            ));
            print_line(format!(
                "recall@{}={:.4}, mrr@{}={:.4}, ndcg@{}={:.4}",
                report.k, report.recall_at_k, report.k, report.mrr_at_k, report.k, report.ndcg_at_k
            ));
            print_line(format!(
                "token_cost.avg_est_tokens={:.2}",
                report.avg_estimated_tokens
            ));
            print_line(format!(
                "latency_ms.p50={:.2}, latency_ms.p95={:.2}",
                report.latency_p50_ms, report.latency_p95_ms
            ));
        }
        return Ok(());
    }

    let baseline_path = baseline.as_deref().ok_or_else(|| {
        anyhow!("`query-benchmark` baseline-vs-candidate mode requires --baseline")
    })?;
    let baseline_metrics = load_baseline_metrics(baseline_path)?;
    let thresholds_config = thresholds.as_deref().map(load_thresholds).transpose()?;
    if enforce_gates && thresholds_config.is_none() {
        return Err(anyhow!(
            "`query-benchmark` --enforce-gates requires --thresholds"
        ));
    }

    let options = QueryBenchmarkOptions::new(k, limit, benchmark_semantic, max_chars, max_tokens)
        .with_auto_index(auto_index)
        .with_semantic_fail_mode(semantic_fail_mode)
        .with_privacy_mode(privacy_mode);
    let mut candidate_runs = Vec::with_capacity(runs);
    let mut run_gate_checks = Vec::with_capacity(runs);

    for run_index in 1..=runs {
        let report = engine.query_benchmark_with_auto_index(&dataset, options)?;
        let run_metrics = BenchmarkMetrics::from_report(&report);
        let gate_eval = thresholds_config
            .as_ref()
            .map(|config| config.evaluate_against_baseline(&baseline_metrics, &run_metrics));

        if enforce_gates
            && gate_eval
                .as_ref()
                .is_some_and(|evaluation| !evaluation.passed)
        {
            let details = gate_eval
                .as_ref()
                .map(GateEvaluation::failure_summary)
                .unwrap_or_else(|| "thresholds are not configured".to_string());
            return Err(anyhow!(
                "query-benchmark fail-fast at run {run_index}: {details}"
            ));
        }

        candidate_runs.push(report);
        run_gate_checks.push(gate_eval);
    }

    let candidate_report = candidate_runs
        .first()
        .ok_or_else(|| anyhow!("query-benchmark produced no runs"))?;
    let candidate_median = median_metrics_from_runs(&candidate_runs)?;
    let candidate_gate_eval = thresholds_config
        .as_ref()
        .map(|config| config.evaluate_against_baseline(&baseline_metrics, &candidate_median));
    let diff = build_metrics_diff(&baseline_metrics, &candidate_median);

    if json {
        let mut payload = build_benchmark_diff_payload(
            candidate_report,
            baseline_path,
            thresholds.as_deref(),
            &candidate_runs,
            &baseline_metrics,
            &candidate_median,
            &diff,
            thresholds_config.as_ref(),
            &run_gate_checks,
            candidate_gate_eval.as_ref(),
            enforce_gates,
            vector_layer_enabled,
            rollout_phase,
            semantic_fail_mode,
            privacy_mode,
            migration_mode,
        )?;
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        print_json(serde_json::to_string_pretty(&payload))?;
    } else {
        print_line(format!(
            "dataset={}, query_count={}, k={}, runs={}",
            sanitize_path_text(privacy_mode, &candidate_report.dataset_path),
            candidate_report.query_count,
            candidate_report.k,
            runs
        ));
        print_line(format!(
            "baseline.recall@{}={:.4}, baseline.mrr@{}={:.4}, baseline.ndcg@{}={:.4}",
            candidate_report.k,
            baseline_metrics.recall_at_k,
            candidate_report.k,
            baseline_metrics.mrr_at_k,
            candidate_report.k,
            baseline_metrics.ndcg_at_k
        ));
        print_line(format!(
            "candidate.median.recall@{}={:.4}, candidate.median.mrr@{}={:.4}, candidate.median.ndcg@{}={:.4}",
            candidate_report.k,
            candidate_median.recall_at_k,
            candidate_report.k,
            candidate_median.mrr_at_k,
            candidate_report.k,
            candidate_median.ndcg_at_k
        ));
        print_line(format!(
            "candidate.median.avg_est_tokens={:.2}, latency_ms.p50={:.2}, latency_ms.p95={:.2}",
            candidate_median.avg_estimated_tokens,
            candidate_median.latency_p50_ms,
            candidate_median.latency_p95_ms
        ));
        print_line(format!(
            "delta.recall_at_k={:+.4}, delta.mrr_at_k={:+.4}, delta.ndcg_at_k={:+.4}, delta.avg_estimated_tokens={:+.2}, delta.latency_p50_ms={:+.2}, delta.latency_p95_ms={:+.2}",
            candidate_median.recall_at_k - baseline_metrics.recall_at_k,
            candidate_median.mrr_at_k - baseline_metrics.mrr_at_k,
            candidate_median.ndcg_at_k - baseline_metrics.ndcg_at_k,
            candidate_median.avg_estimated_tokens - baseline_metrics.avg_estimated_tokens,
            candidate_median.latency_p50_ms - baseline_metrics.latency_p50_ms,
            candidate_median.latency_p95_ms - baseline_metrics.latency_p95_ms
        ));
        if let Some(evaluation) = candidate_gate_eval.as_ref() {
            print_line(format!("thresholds.passed={}", evaluation.passed));
            if !evaluation.passed {
                print_line(format!(
                    "thresholds.failed={}",
                    evaluation.failure_summary()
                ));
            }
        }
    }

    if enforce_gates
        && candidate_gate_eval
            .as_ref()
            .is_some_and(|evaluation| !evaluation.passed)
    {
        let details = candidate_gate_eval
            .as_ref()
            .map(GateEvaluation::failure_summary)
            .unwrap_or_else(|| "thresholds are not configured".to_string());
        return Err(anyhow!("query-benchmark gates failed: {details}"));
    }

    Ok(())
}
