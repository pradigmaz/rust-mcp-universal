use std::path::Path;

use anyhow::{Result, anyhow};
use serde_json::Value;

use rmu_core::{
    BenchmarkMetrics, Engine, GateEvaluation, MigrationMode, PrivacyMode, QueryBenchmarkOptions,
    RolloutPhase, SemanticFailMode, build_benchmark_diff_payload, build_metrics_diff,
    load_baseline_metrics, load_thresholds, median_metrics_from_runs, sanitize_value_for_privacy,
};

use crate::ServerState;
use crate::rpc_tools::errors::invalid_params_error;
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_non_empty_string, parse_optional_usize_with_min,
    parse_required_non_empty_string, reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{
    ensure_query_index_ready, parse_optional_migration_mode, parse_optional_privacy_mode,
    parse_optional_rollout_phase, parse_optional_semantic_fail_mode,
};
pub(super) fn query_benchmark(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "query_benchmark",
        &[
            "dataset_path",
            "k",
            "limit",
            "semantic",
            "auto_index",
            "semantic_fail_mode",
            "privacy_mode",
            "vector_layer_enabled",
            "rollout_phase",
            "migration_mode",
            "max_chars",
            "max_tokens",
            "baseline",
            "thresholds",
            "runs",
            "enforce_gates",
        ],
    )?;
    let dataset_path = parse_required_non_empty_string(args, "query_benchmark", "dataset_path")?;
    let k = parse_optional_usize_with_min(args, "query_benchmark", "k", 1, 10)?;
    let limit = parse_optional_usize_with_min(args, "query_benchmark", "limit", 1, 20)?;
    let semantic = parse_optional_bool(args, "query_benchmark", "semantic")?.unwrap_or(false);
    let auto_index = parse_optional_bool(args, "query_benchmark", "auto_index")?.unwrap_or(false);
    let semantic_fail_mode =
        parse_optional_semantic_fail_mode(args, "query_benchmark", "semantic_fail_mode")?
            .unwrap_or(SemanticFailMode::FailOpen);
    let privacy_mode = parse_optional_privacy_mode(args, "query_benchmark", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let vector_layer_enabled =
        parse_optional_bool(args, "query_benchmark", "vector_layer_enabled")?.unwrap_or(true);
    let rollout_phase = parse_optional_rollout_phase(args, "query_benchmark", "rollout_phase")?
        .unwrap_or(RolloutPhase::Full100);
    let migration_mode = parse_optional_migration_mode(args, "query_benchmark", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);
    let max_chars =
        parse_optional_usize_with_min(args, "query_benchmark", "max_chars", 256, 12_000)?;
    let max_tokens =
        parse_optional_usize_with_min(args, "query_benchmark", "max_tokens", 64, 3_000)?;
    let baseline = parse_optional_non_empty_string(args, "query_benchmark", "baseline")?;
    let thresholds = parse_optional_non_empty_string(args, "query_benchmark", "thresholds")?;
    let runs = parse_optional_usize_with_min(args, "query_benchmark", "runs", 1, 1)?;
    let enforce_gates =
        parse_optional_bool(args, "query_benchmark", "enforce_gates")?.unwrap_or(false);
    let compare_mode_requested =
        baseline.is_some() || thresholds.is_some() || runs > 1 || enforce_gates;

    let benchmark_semantic =
        semantic && vector_layer_enabled && !matches!(rollout_phase, RolloutPhase::Shadow);
    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )?;
    if auto_index {
        ensure_query_index_ready(&engine, true)?;
    }
    if !compare_mode_requested {
        let report = engine.query_benchmark_with_auto_index(
            Path::new(&dataset_path),
            QueryBenchmarkOptions::new(k, limit, benchmark_semantic, max_chars, max_tokens)
                .with_auto_index(false)
                .with_semantic_fail_mode(semantic_fail_mode)
                .with_privacy_mode(privacy_mode),
        )?;
        let mut payload = serde_json::to_value(report)?;
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        return tool_result(payload);
    }

    let baseline_path = baseline.ok_or_else(|| {
        invalid_params_error("query_benchmark compare mode requires non-empty `baseline`")
    })?;
    if enforce_gates && thresholds.is_none() {
        return Err(invalid_params_error(
            "query_benchmark `enforce_gates` requires non-empty `thresholds`",
        ));
    }

    let baseline_metrics = load_baseline_metrics(Path::new(&baseline_path))?;
    let thresholds_config = thresholds
        .as_deref()
        .map(|path| load_thresholds(Path::new(path)))
        .transpose()?;

    let options = QueryBenchmarkOptions::new(k, limit, benchmark_semantic, max_chars, max_tokens)
        .with_auto_index(false)
        .with_semantic_fail_mode(semantic_fail_mode)
        .with_privacy_mode(privacy_mode);
    let mut candidate_runs = Vec::with_capacity(runs);
    let mut run_gate_checks = Vec::with_capacity(runs);

    for run_index in 1..=runs {
        let report = engine.query_benchmark_with_auto_index(Path::new(&dataset_path), options)?;
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
                "query_benchmark fail-fast at run {run_index}: {details}"
            ));
        }

        candidate_runs.push(report);
        run_gate_checks.push(gate_eval);
    }

    let candidate_report = candidate_runs
        .first()
        .ok_or_else(|| anyhow!("query_benchmark produced no runs"))?;
    let candidate_median = median_metrics_from_runs(&candidate_runs)?;
    let candidate_gate_eval = thresholds_config
        .as_ref()
        .map(|config| config.evaluate_against_baseline(&baseline_metrics, &candidate_median));
    let diff = build_metrics_diff(&baseline_metrics, &candidate_median);
    let mut payload = build_benchmark_diff_payload(
        candidate_report,
        Path::new(&baseline_path),
        thresholds.as_deref().map(Path::new),
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

    if enforce_gates
        && candidate_gate_eval
            .as_ref()
            .is_some_and(|evaluation| !evaluation.passed)
    {
        let details = candidate_gate_eval
            .as_ref()
            .map(GateEvaluation::failure_summary)
            .unwrap_or_else(|| "thresholds are not configured".to_string());
        return Err(anyhow!("query_benchmark gates failed: {details}"));
    }

    tool_result(payload)
}
