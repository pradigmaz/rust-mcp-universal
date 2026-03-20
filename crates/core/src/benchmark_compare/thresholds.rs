use std::path::Path;

use anyhow::Result;
use serde_json::{Map, Value, json};

use super::metrics::BenchmarkMetrics;

mod load;
mod normalize;
mod parse;
mod validate;

#[derive(Debug, Clone, Default)]
pub struct ThresholdConfig {
    min_recall_at_k: Option<f32>,
    min_mrr_at_k: Option<f32>,
    min_ndcg_at_k: Option<f32>,
    max_avg_estimated_tokens: Option<f32>,
    max_latency_p50_ms: Option<f32>,
    max_latency_p95_ms: Option<f32>,
}

impl ThresholdConfig {
    fn has_thresholds(&self) -> bool {
        self.min_recall_at_k.is_some()
            || self.min_mrr_at_k.is_some()
            || self.min_ndcg_at_k.is_some()
            || self.max_avg_estimated_tokens.is_some()
            || self.max_latency_p50_ms.is_some()
            || self.max_latency_p95_ms.is_some()
    }

    pub(crate) fn to_value(&self) -> Value {
        let mut min_map = Map::new();
        insert_optional_metric(&mut min_map, "recall_at_k", self.min_recall_at_k);
        insert_optional_metric(&mut min_map, "mrr_at_k", self.min_mrr_at_k);
        insert_optional_metric(&mut min_map, "ndcg_at_k", self.min_ndcg_at_k);

        let mut max_map = Map::new();
        insert_optional_metric(
            &mut max_map,
            "avg_estimated_tokens",
            self.max_avg_estimated_tokens,
        );
        insert_optional_metric(&mut max_map, "latency_p50_ms", self.max_latency_p50_ms);
        insert_optional_metric(&mut max_map, "latency_p95_ms", self.max_latency_p95_ms);

        json!({
            "min": Value::Object(min_map),
            "max": Value::Object(max_map)
        })
    }

    pub fn evaluate(&self, metrics: &BenchmarkMetrics) -> GateEvaluation {
        self.evaluate_against_baseline(metrics, metrics)
    }

    pub fn evaluate_against_baseline(
        &self,
        baseline: &BenchmarkMetrics,
        metrics: &BenchmarkMetrics,
    ) -> GateEvaluation {
        let mut checks = Vec::new();
        if let Some(threshold) = self.min_recall_at_k {
            checks.push(GateCheck::new(
                "recall_at_k",
                ">=",
                metrics.recall_at_k,
                threshold,
            ));
        }
        if let Some(threshold) = self.min_mrr_at_k {
            checks.push(GateCheck::new(
                "mrr_at_k",
                ">=",
                metrics.mrr_at_k,
                threshold,
            ));
        }
        if let Some(threshold) = self.min_ndcg_at_k {
            checks.push(GateCheck::new(
                "ndcg_at_k",
                ">=",
                metrics.ndcg_at_k,
                threshold,
            ));
        }
        if let Some(threshold) = self.max_avg_estimated_tokens {
            checks.push(GateCheck::new(
                "avg_estimated_tokens",
                "<=",
                metrics.avg_estimated_tokens,
                threshold,
            ));
        }
        if let Some(threshold) = self.max_latency_p50_ms {
            checks.push(GateCheck::new(
                "latency_p50_ms",
                "<=",
                metrics.latency_p50_ms,
                threshold,
            ));
        }
        if let Some(threshold) = self.max_latency_p95_ms {
            checks.push(GateCheck::new(
                "latency_p95_ms",
                "<=",
                metrics.latency_p95_ms,
                threshold,
            ));
        }
        if self.max_avg_estimated_tokens.is_some()
            && metrics.avg_estimated_tokens > baseline.avg_estimated_tokens
        {
            let quality_uplift = metrics.recall_at_k > baseline.recall_at_k
                || metrics.mrr_at_k > baseline.mrr_at_k
                || metrics.ndcg_at_k > baseline.ndcg_at_k;
            checks.push(GateCheck::new(
                "token_cost_requires_quality_uplift",
                "==",
                if quality_uplift { 1.0 } else { 0.0 },
                1.0,
            ));
        }

        let passed = checks.iter().all(|check| check.passed);
        GateEvaluation { passed, checks }
    }
}

#[derive(Debug, Clone)]
struct GateCheck {
    metric: &'static str,
    comparator: &'static str,
    actual: f32,
    threshold: f32,
    passed: bool,
}

impl GateCheck {
    fn new(metric: &'static str, comparator: &'static str, actual: f32, threshold: f32) -> Self {
        let passed = match comparator {
            ">=" => actual >= threshold,
            "<=" => actual <= threshold,
            "==" => (actual - threshold).abs() <= f32::EPSILON,
            _ => false,
        };
        Self {
            metric,
            comparator,
            actual,
            threshold,
            passed,
        }
    }

    fn to_value(&self) -> Value {
        json!({
            "metric": self.metric,
            "comparator": self.comparator,
            "actual": self.actual,
            "threshold": self.threshold,
            "passed": self.passed
        })
    }
}

#[derive(Debug, Clone)]
pub struct GateEvaluation {
    pub passed: bool,
    checks: Vec<GateCheck>,
}

impl GateEvaluation {
    pub(crate) fn to_value(&self) -> Value {
        let checks = self
            .checks
            .iter()
            .map(GateCheck::to_value)
            .collect::<Vec<_>>();
        json!({
            "passed": self.passed,
            "checks": checks
        })
    }

    pub fn failure_summary(&self) -> String {
        let failed = self
            .checks
            .iter()
            .filter(|check| !check.passed)
            .map(|check| {
                format!(
                    "{} {} {} (actual={:.6})",
                    check.metric, check.comparator, check.threshold, check.actual
                )
            })
            .collect::<Vec<_>>();
        if failed.is_empty() {
            "all gates passed".to_string()
        } else {
            failed.join("; ")
        }
    }

    pub(crate) fn failure_categories(&self) -> (bool, bool, bool) {
        let mut quality = false;
        let mut latency = false;
        let mut token_cost = false;

        for check in &self.checks {
            if check.passed {
                continue;
            }
            match check.metric {
                "recall_at_k" | "mrr_at_k" | "ndcg_at_k" => quality = true,
                "latency_p50_ms" | "latency_p95_ms" => latency = true,
                "avg_estimated_tokens" | "token_cost_requires_quality_uplift" => {
                    token_cost = true;
                }
                _ => {}
            }
        }

        (quality, latency, token_cost)
    }
}

fn insert_optional_metric(target: &mut Map<String, Value>, name: &str, value: Option<f32>) {
    if let Some(value) = value {
        let _ = target.insert(name.to_string(), json!(value));
    }
}

pub fn load_thresholds(path: &Path) -> Result<ThresholdConfig> {
    let value = load::load_thresholds_value(path)?;
    let sections = parse::parse_sections(&value);
    let source = load::source_label(path);

    validate::validate_section_shapes(&sections, &source)?;
    let config = normalize::normalize_thresholds(&sections, &source)?;
    validate::validate_supported_metrics(&config, path)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::{BenchmarkMetrics, ThresholdConfig};

    fn metrics(
        recall_at_k: f32,
        mrr_at_k: f32,
        ndcg_at_k: f32,
        avg_estimated_tokens: f32,
        latency_p50_ms: f32,
        latency_p95_ms: f32,
    ) -> BenchmarkMetrics {
        BenchmarkMetrics {
            recall_at_k,
            mrr_at_k,
            ndcg_at_k,
            avg_estimated_tokens,
            latency_p50_ms,
            latency_p95_ms,
        }
    }

    #[test]
    fn evaluate_against_baseline_flags_latency_p95_regression() {
        let config = ThresholdConfig {
            min_recall_at_k: None,
            min_mrr_at_k: None,
            min_ndcg_at_k: None,
            max_avg_estimated_tokens: None,
            max_latency_p50_ms: None,
            max_latency_p95_ms: Some(20.0),
        };
        let baseline = metrics(0.9, 0.9, 0.9, 100.0, 10.0, 15.0);
        let candidate = metrics(0.9, 0.9, 0.9, 100.0, 10.0, 22.0);

        let eval = config.evaluate_against_baseline(&baseline, &candidate);
        assert!(!eval.passed);
        assert!(
            eval.failure_summary().contains("latency_p95_ms <= 20"),
            "unexpected summary: {}",
            eval.failure_summary()
        );
    }

    #[test]
    fn evaluate_against_baseline_blocks_token_growth_without_quality_uplift() {
        let config = ThresholdConfig {
            min_recall_at_k: None,
            min_mrr_at_k: None,
            min_ndcg_at_k: None,
            max_avg_estimated_tokens: Some(2_000.0),
            max_latency_p50_ms: None,
            max_latency_p95_ms: None,
        };
        let baseline = metrics(1.0, 1.0, 1.0, 100.0, 10.0, 15.0);
        let candidate_same_quality = metrics(1.0, 1.0, 1.0, 120.0, 10.0, 15.0);
        let candidate_with_uplift = metrics(1.0, 1.0, 1.01, 120.0, 10.0, 15.0);

        let blocked = config.evaluate_against_baseline(&baseline, &candidate_same_quality);
        assert!(!blocked.passed);
        assert!(
            blocked
                .failure_summary()
                .contains("token_cost_requires_quality_uplift == 1"),
            "unexpected summary: {}",
            blocked.failure_summary()
        );

        let allowed = config.evaluate_against_baseline(&baseline, &candidate_with_uplift);
        assert!(allowed.passed);
    }
}
