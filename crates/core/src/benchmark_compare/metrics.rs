use std::cmp::Ordering;
use std::path::Path;

use anyhow::{Result, bail};
use serde_json::{Value, json};

use crate::QueryBenchmarkReport;

use super::parsing::{parse_required_metric, read_json_file};

#[derive(Debug, Clone, Copy)]
pub struct BenchmarkMetrics {
    pub recall_at_k: f32,
    pub mrr_at_k: f32,
    pub ndcg_at_k: f32,
    pub avg_estimated_tokens: f32,
    pub latency_p50_ms: f32,
    pub latency_p95_ms: f32,
}

impl BenchmarkMetrics {
    pub fn from_report(report: &QueryBenchmarkReport) -> Self {
        Self {
            recall_at_k: report.recall_at_k,
            mrr_at_k: report.mrr_at_k,
            ndcg_at_k: report.ndcg_at_k,
            avg_estimated_tokens: report.avg_estimated_tokens,
            latency_p50_ms: report.latency_p50_ms,
            latency_p95_ms: report.latency_p95_ms,
        }
    }

    pub(crate) fn to_value(self) -> Value {
        json!({
            "recall_at_k": self.recall_at_k,
            "mrr_at_k": self.mrr_at_k,
            "ndcg_at_k": self.ndcg_at_k,
            "avg_estimated_tokens": self.avg_estimated_tokens,
            "latency_p50_ms": self.latency_p50_ms,
            "latency_p95_ms": self.latency_p95_ms
        })
    }
}

fn metrics_from_json_object(value: &Value, source: &str) -> Result<BenchmarkMetrics> {
    Ok(BenchmarkMetrics {
        recall_at_k: parse_required_metric(value, "recall_at_k", source)?,
        mrr_at_k: parse_required_metric(value, "mrr_at_k", source)?,
        ndcg_at_k: parse_required_metric(value, "ndcg_at_k", source)?,
        avg_estimated_tokens: parse_required_metric(value, "avg_estimated_tokens", source)?,
        latency_p50_ms: parse_required_metric(value, "latency_p50_ms", source)?,
        latency_p95_ms: parse_required_metric(value, "latency_p95_ms", source)?,
    })
}

pub fn load_baseline_metrics(path: &Path) -> Result<BenchmarkMetrics> {
    let value = read_json_file(path, "baseline")?;
    let source = format!("baseline file `{}`", path.display());
    if let Ok(metrics) = metrics_from_json_object(&value, &source) {
        return Ok(metrics);
    }
    if let Some(median) = value.get("median") {
        let median_source = format!("{source}.median");
        return metrics_from_json_object(median, &median_source);
    }

    bail!(
        "baseline file `{}` must contain metrics at top-level or under `median`",
        path.display()
    );
}

pub fn median_metrics_from_runs(reports: &[QueryBenchmarkReport]) -> Result<BenchmarkMetrics> {
    if reports.is_empty() {
        bail!("query_benchmark requires at least one run to compute median");
    }

    let recall_values = reports.iter().map(|r| r.recall_at_k).collect::<Vec<_>>();
    let mrr_values = reports.iter().map(|r| r.mrr_at_k).collect::<Vec<_>>();
    let ndcg_values = reports.iter().map(|r| r.ndcg_at_k).collect::<Vec<_>>();
    let token_values = reports
        .iter()
        .map(|r| r.avg_estimated_tokens)
        .collect::<Vec<_>>();
    let latency_p50_values = reports.iter().map(|r| r.latency_p50_ms).collect::<Vec<_>>();
    let latency_p95_values = reports.iter().map(|r| r.latency_p95_ms).collect::<Vec<_>>();

    Ok(BenchmarkMetrics {
        recall_at_k: median_f32(recall_values),
        mrr_at_k: median_f32(mrr_values),
        ndcg_at_k: median_f32(ndcg_values),
        avg_estimated_tokens: median_f32(token_values),
        latency_p50_ms: median_f32(latency_p50_values),
        latency_p95_ms: median_f32(latency_p95_values),
    })
}

fn median_f32(mut values: Vec<f32>) -> f32 {
    values.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(Ordering::Equal));
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        values[middle]
    } else {
        (values[middle - 1] + values[middle]) / 2.0
    }
}

pub fn build_metrics_diff(baseline: &BenchmarkMetrics, candidate: &BenchmarkMetrics) -> Value {
    json!({
        "recall_at_k": metric_diff_entry(
            baseline.recall_at_k,
            candidate.recall_at_k,
            "higher_is_better"
        ),
        "mrr_at_k": metric_diff_entry(
            baseline.mrr_at_k,
            candidate.mrr_at_k,
            "higher_is_better"
        ),
        "ndcg_at_k": metric_diff_entry(
            baseline.ndcg_at_k,
            candidate.ndcg_at_k,
            "higher_is_better"
        ),
        "avg_estimated_tokens": metric_diff_entry(
            baseline.avg_estimated_tokens,
            candidate.avg_estimated_tokens,
            "lower_is_better"
        ),
        "latency_p50_ms": metric_diff_entry(
            baseline.latency_p50_ms,
            candidate.latency_p50_ms,
            "lower_is_better"
        ),
        "latency_p95_ms": metric_diff_entry(
            baseline.latency_p95_ms,
            candidate.latency_p95_ms,
            "lower_is_better"
        )
    })
}

fn metric_diff_entry(baseline: f32, candidate: f32, direction: &str) -> Value {
    let delta_abs = candidate - baseline;
    let delta_pct = if baseline.abs() <= f32::EPSILON {
        None
    } else {
        Some((delta_abs / baseline) * 100.0)
    };
    json!({
        "baseline": baseline,
        "candidate": candidate,
        "delta_abs": delta_abs,
        "delta_pct": delta_pct,
        "direction": direction
    })
}
