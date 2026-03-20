use std::path::Path;

use anyhow::Result;
use serde_json::{Value, json};

use crate::{
    MigrationMode, PrivacyMode, QueryBenchmarkReport, RollbackSignals, RolloutPhase,
    SemanticFailMode, recommend_rollback, stable_cycles_observed,
};

use super::metrics::BenchmarkMetrics;
use super::thresholds::{GateEvaluation, ThresholdConfig};

#[allow(clippy::too_many_arguments)]
pub fn build_benchmark_diff_payload(
    candidate_report: &QueryBenchmarkReport,
    baseline_path: &Path,
    thresholds_path: Option<&Path>,
    candidate_runs: &[QueryBenchmarkReport],
    baseline_metrics: &BenchmarkMetrics,
    candidate_median: &BenchmarkMetrics,
    diff: &Value,
    thresholds_config: Option<&ThresholdConfig>,
    run_gate_checks: &[Option<GateEvaluation>],
    candidate_gate_eval: Option<&GateEvaluation>,
    enforce_gates: bool,
    vector_layer_enabled: bool,
    rollout_phase: RolloutPhase,
    semantic_fail_mode: SemanticFailMode,
    privacy_mode: PrivacyMode,
    migration_mode: MigrationMode,
) -> Result<Value> {
    let run_evaluations = run_gate_checks
        .iter()
        .map(|evaluation| {
            evaluation
                .as_ref()
                .map(GateEvaluation::to_value)
                .unwrap_or(Value::Null)
        })
        .collect::<Vec<_>>();
    let run_passes = run_gate_checks
        .iter()
        // Missing evaluation means no threshold configured for that run; treat as non-failing.
        .map(|evaluation| evaluation.as_ref().is_none_or(|item| item.passed))
        .collect::<Vec<_>>();
    let stable_observed = stable_cycles_observed(&run_passes);
    let ready_for_next_wave = stable_observed >= 2 && run_passes.len() >= 2;
    let (quality_regression, latency_regression, token_cost_regression) = candidate_gate_eval
        .map(GateEvaluation::failure_categories)
        .unwrap_or((false, false, false));
    let rollback = recommend_rollback(&RollbackSignals {
        quality_regression,
        latency_regression,
        token_cost_regression,
        privacy_violation: false,
        error_spike: false,
    });

    Ok(json!({
        "mode": "baseline_vs_candidate",
        "dataset_path": candidate_report.dataset_path,
        "query_count": candidate_report.query_count,
        "k": candidate_report.k,
        "runs_count": candidate_runs.len(),
        "median_rule": format!("median_of_{}_runs", candidate_runs.len()),
        "baseline": {
            "path": baseline_path.display().to_string(),
            "metrics": baseline_metrics.to_value()
        },
        "candidate": {
            "runs": serde_json::to_value(candidate_runs)?,
            "median": candidate_median.to_value()
        },
        "diff": diff,
        "thresholds": {
            "path": thresholds_path.map(|path| path.display().to_string()),
            "configured": thresholds_config.map(ThresholdConfig::to_value),
            "run_evaluations": run_evaluations,
            "candidate_median_evaluation": candidate_gate_eval.map(GateEvaluation::to_value),
            "passed": candidate_gate_eval.map(|evaluation| evaluation.passed)
        },
        "enforce_gates": enforce_gates,
        "feature_flags": {
            "vector_layer_enabled": vector_layer_enabled,
            "rollout_phase": rollout_phase.as_str(),
            "semantic_fail_mode": semantic_fail_mode.as_str(),
            "privacy_mode": privacy_mode.as_str(),
            "migration_mode": migration_mode.as_str()
        },
        "rollout": {
            "stable_cycles_required": 2,
            "stable_cycles_observed": stable_observed,
            "ready_for_next_wave": ready_for_next_wave,
            "waves": ["shadow", "canary_5", "canary_25", "full_100"]
        },
        "rollback": serde_json::to_value(rollback)?
    }))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::build_benchmark_diff_payload;
    use crate::{
        MigrationMode, PrivacyMode, QueryBenchmarkReport, RolloutPhase, SemanticFailMode,
        benchmark_compare::{metrics::BenchmarkMetrics, thresholds::GateEvaluation},
    };

    fn sample_report() -> QueryBenchmarkReport {
        QueryBenchmarkReport {
            dataset_path: "dataset.jsonl".to_string(),
            k: 10,
            query_count: 3,
            recall_at_k: 0.82,
            mrr_at_k: 0.76,
            ndcg_at_k: 0.79,
            avg_estimated_tokens: 120.0,
            latency_p50_ms: 24.0,
            latency_p95_ms: 46.0,
        }
    }

    fn sample_metrics() -> BenchmarkMetrics {
        BenchmarkMetrics {
            recall_at_k: 0.8,
            mrr_at_k: 0.74,
            ndcg_at_k: 0.77,
            avg_estimated_tokens: 122.0,
            latency_p50_ms: 25.0,
            latency_p95_ms: 48.0,
        }
    }

    #[test]
    fn no_threshold_gate_evaluations_are_not_counted_as_failures() {
        let candidate_report = sample_report();
        let candidate_runs = vec![
            candidate_report.clone(),
            candidate_report.clone(),
            candidate_report.clone(),
        ];
        let run_gate_checks: Vec<Option<GateEvaluation>> = vec![None, None, None];

        let payload = build_benchmark_diff_payload(
            &candidate_report,
            Path::new("baseline.json"),
            None,
            &candidate_runs,
            &sample_metrics(),
            &sample_metrics(),
            &serde_json::json!({}),
            None,
            &run_gate_checks,
            None,
            false,
            true,
            RolloutPhase::Canary25,
            SemanticFailMode::FailOpen,
            PrivacyMode::Off,
            MigrationMode::Auto,
        )
        .expect("payload should build without thresholds");

        assert_eq!(
            payload["thresholds"]["run_evaluations"],
            serde_json::json!([null, null, null])
        );
        assert_eq!(
            payload["rollout"]["stable_cycles_observed"].as_u64(),
            Some(3)
        );
        assert_eq!(
            payload["rollout"]["ready_for_next_wave"].as_bool(),
            Some(true)
        );
    }
}
