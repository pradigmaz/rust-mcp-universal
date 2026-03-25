use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow};
use rmu_core::{
    InvestigationBenchmarkDataset, InvestigationBenchmarkReport, InvestigationThresholdVerdict,
    InvestigationThresholds, sanitize_path_text, sanitize_value_for_privacy,
};

use super::investigation_benchmark_compare::{build_diff_report, load_baseline_report};
use super::investigation_benchmark_eval::{
    build_tool_metrics, evaluate_thresholds, run_case, tool_label,
};
use super::*;
use rusqlite::Connection;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct NavigationLatencyTarget {
    p95_ms: f32,
}

#[derive(Debug, Deserialize)]
struct NavigationLatencyBaseline {
    status: String,
    per_tool_latency_targets: std::collections::BTreeMap<String, NavigationLatencyTarget>,
}

pub(crate) fn run_investigation_benchmark(
    engine: &Engine,
    json: bool,
    args: InvestigationBenchmarkArgs,
) -> Result<()> {
    let InvestigationBenchmarkArgs {
        dataset,
        limit,
        auto_index,
        privacy_mode,
        baseline_report,
        thresholds,
        enforce_gates,
    } = args;
    let limit = require_min("limit", limit, 1)?;
    if enforce_gates && thresholds.is_none() {
        return Err(anyhow!(
            "`investigation-benchmark` --enforce-gates requires --thresholds"
        ));
    }
    let dataset_raw = fs::read_to_string(&dataset)?;
    let dataset_payload: InvestigationBenchmarkDataset = serde_json::from_str(&dataset_raw)?;
    let required_paths = dataset_payload
        .cases
        .iter()
        .filter_map(|case| match case.seed_kind {
            rmu_core::ConceptSeedKind::Path => Some(case.seed.trim().to_string()),
            rmu_core::ConceptSeedKind::PathLine => case
                .seed
                .trim()
                .rsplit_once(':')
                .map(|(path, line)| (path.trim(), line.trim()))
                .filter(|(path, line)| !path.is_empty() && line.parse::<usize>().is_ok())
                .map(|(path, _)| path.to_string()),
            rmu_core::ConceptSeedKind::Query | rmu_core::ConceptSeedKind::Symbol => {
                Path::new(case.seed.trim())
                    .extension()
                    .is_some()
                    .then(|| case.seed.trim().to_string())
            }
        })
        .collect::<Vec<_>>();
    let _ = engine.ensure_mixed_index_ready_for_paths(auto_index, &required_paths)?;
    let thresholds_payload = thresholds
        .as_deref()
        .map(fs::read_to_string)
        .transpose()?
        .map(|raw| serde_json::from_str::<InvestigationThresholds>(&raw))
        .transpose()?;
    let mut cases = Vec::with_capacity(dataset_payload.cases.len());
    for case in &dataset_payload.cases {
        prepare_case_environment(engine, case)?;
        cases.push(run_case(engine, case, limit, privacy_mode)?);
    }
    let mut per_tool_metrics = build_tool_metrics(&cases);
    let navigation_latency_baseline = load_navigation_latency_baseline(&engine.project_root)?;
    let navigation_latency_baseline_status = navigation_latency_baseline
        .as_ref()
        .map(|baseline| baseline.status.clone());
    apply_navigation_latency_baseline(&mut per_tool_metrics, navigation_latency_baseline.as_ref());
    let privacy_failures = cases.iter().map(|case| case.privacy_failures).sum();
    let unsupported_behavior_summary = cases
        .iter()
        .filter(|case| !case.unsupported_sources.is_empty())
        .map(|case| {
            format!(
                "{}:{}:{}",
                case.id,
                tool_label(case.tool),
                case.unsupported_sources.join("|")
            )
        })
        .collect::<Vec<_>>();
    let threshold_verdict = thresholds_payload
        .as_ref()
        .map(|thresholds| evaluate_thresholds(thresholds, &per_tool_metrics, privacy_failures));
    let baseline_payload = baseline_report
        .as_deref()
        .map(load_baseline_report)
        .transpose()?;
    let diff = baseline_payload.as_ref().map(|baseline| {
        build_diff_report(
            baseline,
            &report_without_diff(
                &dataset,
                limit,
                &per_tool_metrics,
                &cases,
                &unsupported_behavior_summary,
                privacy_failures,
                threshold_verdict.clone(),
                navigation_latency_baseline_status.clone(),
            ),
        )
    });
    let threshold_verdict = merge_threshold_failures(threshold_verdict, diff.as_ref());
    let report = InvestigationBenchmarkReport {
        dataset_path: dataset.display().to_string(),
        limit,
        case_count: cases.len(),
        per_tool_metrics,
        cases,
        unsupported_behavior_summary,
        privacy_failures,
        threshold_verdict,
        navigation_latency_baseline_status,
        diff,
    };
    if enforce_gates
        && report
            .threshold_verdict
            .as_ref()
            .is_some_and(|verdict| !verdict.passed)
    {
        let failure = report
            .threshold_verdict
            .as_ref()
            .map(|verdict| verdict.failures.join("; "))
            .unwrap_or_else(|| "thresholds failed".to_string());
        return Err(anyhow!("investigation-benchmark gates failed: {failure}"));
    }
    if json {
        let mut payload = serde_json::to_value(&report)?;
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        print_json(serde_json::to_string_pretty(&payload))?;
    } else {
        print_line(format!(
            "dataset={}, cases={}, limit={}",
            sanitize_path_text(privacy_mode, &report.dataset_path),
            report.case_count,
            report.limit
        ));
        for metric in &report.per_tool_metrics {
            print_line(format!(
                "tool={}, cases={}, pass_rate={:.2}, unsupported_rate={:.2}, latency_ms.p50={:.2}, latency_ms.p95={:.2}",
                tool_label(metric.tool),
                metric.case_count,
                metric.pass_rate,
                metric.unsupported_case_rate,
                metric.latency_p50_ms,
                metric.latency_p95_ms
            ));
        }
        print_line(format!("privacy_failures={}", report.privacy_failures));
        if let Some(status) = &report.navigation_latency_baseline_status {
            print_line(format!("navigation_latency_baseline.status={status}"));
        }
        if let Some(verdict) = &report.threshold_verdict {
            print_line(format!("thresholds.passed={}", verdict.passed));
            if !verdict.failures.is_empty() {
                print_line(format!(
                    "thresholds.failures={}",
                    verdict.failures.join(" | ")
                ));
            }
        }
        if let Some(diff) = &report.diff {
            print_line(format!(
                "diff.case_count={} -> {}",
                diff.baseline_case_count, diff.current_case_count
            ));
            print_line(format!(
                "diff.regressed_metrics={}, diff.improved_metrics={}",
                diff.regressed_metrics.len(),
                diff.improved_metrics.len()
            ));
            if !diff.regression_failures.is_empty() {
                print_line(format!(
                    "diff.regression_failures={}",
                    diff.regression_failures.join(" | ")
                ));
            }
        }
    }
    Ok(())
}

fn report_without_diff(
    dataset: &Path,
    limit: usize,
    per_tool_metrics: &[rmu_core::InvestigationToolMetrics],
    cases: &[rmu_core::InvestigationCaseReport],
    unsupported_behavior_summary: &[String],
    privacy_failures: usize,
    threshold_verdict: Option<InvestigationThresholdVerdict>,
    navigation_latency_baseline_status: Option<String>,
) -> InvestigationBenchmarkReport {
    InvestigationBenchmarkReport {
        dataset_path: dataset.display().to_string(),
        limit,
        case_count: cases.len(),
        per_tool_metrics: per_tool_metrics.to_vec(),
        cases: cases.to_vec(),
        unsupported_behavior_summary: unsupported_behavior_summary.to_vec(),
        privacy_failures,
        threshold_verdict,
        navigation_latency_baseline_status,
        diff: None,
    }
}

fn merge_threshold_failures(
    verdict: Option<InvestigationThresholdVerdict>,
    diff: Option<&rmu_core::InvestigationBenchmarkDiffReport>,
) -> Option<InvestigationThresholdVerdict> {
    let Some(mut verdict) = verdict else {
        return None;
    };
    if let Some(diff) = diff {
        verdict
            .failures
            .extend(diff.regression_failures.iter().cloned());
        verdict.passed = verdict.failures.is_empty();
    }
    Some(verdict)
}

fn prepare_case_environment(
    engine: &Engine,
    case: &rmu_core::InvestigationBenchmarkCase,
) -> Result<()> {
    if !case.labels.semantic_fail_open_case {
        return Ok(());
    }
    let conn = Connection::open(&engine.db_path)?;
    conn.execute("UPDATE semantic_vectors SET vector_json = '[0]'", [])?;
    Ok(())
}

fn load_navigation_latency_baseline(
    project_root: &Path,
) -> Result<Option<NavigationLatencyBaseline>> {
    let baseline_path =
        project_root.join("baseline/investigation/stage0/navigation_latency_baseline.json");
    if !baseline_path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(baseline_path)?;
    serde_json::from_str(&raw).map(Some).map_err(Into::into)
}

fn apply_navigation_latency_baseline(
    metrics: &mut [rmu_core::InvestigationToolMetrics],
    baseline: Option<&NavigationLatencyBaseline>,
) {
    let Some(baseline) = baseline else {
        return;
    };
    if baseline.status == "bootstrap_pending_refresh" {
        return;
    }
    for metric in metrics {
        if !matches!(
            metric.tool,
            rmu_core::InvestigationBenchmarkTool::SymbolBody
        ) {
            continue;
        }
        let Some(target) = baseline.per_tool_latency_targets.get("symbol_body") else {
            continue;
        };
        if target.p95_ms <= 0.0 {
            continue;
        }
        metric.body_request_p95_budget_ms = Some(target.p95_ms);
        metric.body_request_p95_ratio = Some(metric.latency_p95_ms / target.p95_ms);
    }
}
