use crate::model::{
    QueryBenchmarkGateCategoryResult, QueryBenchmarkGateEvaluation, QueryBenchmarkGateMetricResult,
    QueryBenchmarkGateThresholds, QueryBenchmarkReport,
};

pub(super) fn evaluate_gates(
    baseline: &QueryBenchmarkReport,
    candidate: &QueryBenchmarkReport,
    thresholds: QueryBenchmarkGateThresholds,
    fail_fast: bool,
) -> QueryBenchmarkGateEvaluation {
    let mut categories = Vec::with_capacity(3);
    let mut first_failed_category = None;

    let quality = evaluate_quality_gate(baseline, candidate, thresholds);
    if !quality.pass {
        first_failed_category = Some(quality.category.clone());
    }
    categories.push(quality);

    if fail_fast && first_failed_category.is_some() {
        categories.push(skipped_gate("latency"));
        categories.push(skipped_gate("token_cost"));
    } else {
        let latency = evaluate_latency_gate(baseline, candidate, thresholds);
        if !latency.pass && first_failed_category.is_none() {
            first_failed_category = Some(latency.category.clone());
        }
        let latency_failed = !latency.pass;
        categories.push(latency);

        if fail_fast && latency_failed {
            categories.push(skipped_gate("token_cost"));
        } else {
            let token_cost = evaluate_token_cost_gate(baseline, candidate, thresholds);
            if !token_cost.pass && first_failed_category.is_none() {
                first_failed_category = Some(token_cost.category.clone());
            }
            categories.push(token_cost);
        }
    }

    QueryBenchmarkGateEvaluation {
        fail_fast,
        overall_pass: first_failed_category.is_none(),
        first_failed_category,
        categories,
    }
}

fn evaluate_quality_gate(
    baseline: &QueryBenchmarkReport,
    candidate: &QueryBenchmarkReport,
    thresholds: QueryBenchmarkGateThresholds,
) -> QueryBenchmarkGateCategoryResult {
    let min_recall = baseline.recall_at_k * (1.0 - thresholds.quality_max_drop_ratio);
    let min_mrr = baseline.mrr_at_k * (1.0 - thresholds.quality_max_drop_ratio);
    let min_ndcg = baseline.ndcg_at_k * (1.0 - thresholds.quality_max_drop_ratio);

    let metrics = vec![
        min_metric(
            "recall_at_k",
            min_recall,
            baseline.recall_at_k,
            candidate.recall_at_k,
        ),
        min_metric("mrr_at_k", min_mrr, baseline.mrr_at_k, candidate.mrr_at_k),
        min_metric(
            "ndcg_at_k",
            min_ndcg,
            baseline.ndcg_at_k,
            candidate.ndcg_at_k,
        ),
    ];
    let pass = metrics.iter().all(|metric| metric.pass);

    QueryBenchmarkGateCategoryResult {
        category: "quality".to_string(),
        pass,
        skipped: false,
        metrics,
    }
}

fn evaluate_latency_gate(
    baseline: &QueryBenchmarkReport,
    candidate: &QueryBenchmarkReport,
    thresholds: QueryBenchmarkGateThresholds,
) -> QueryBenchmarkGateCategoryResult {
    let max_p50 = baseline.latency_p50_ms * (1.0 + thresholds.latency_p50_max_increase_ratio);
    let max_p95_by_ratio =
        baseline.latency_p95_ms * (1.0 + thresholds.latency_p95_max_increase_ratio);
    let max_p95_by_absolute = baseline.latency_p95_ms + thresholds.latency_p95_max_increase_ms;
    let max_p95 = max_p95_by_ratio.min(max_p95_by_absolute);

    let metrics = vec![
        max_metric(
            "latency_p50_ms",
            max_p50,
            baseline.latency_p50_ms,
            candidate.latency_p50_ms,
        ),
        max_metric(
            "latency_p95_ms",
            max_p95,
            baseline.latency_p95_ms,
            candidate.latency_p95_ms,
        ),
    ];
    let pass = metrics.iter().all(|metric| metric.pass);

    QueryBenchmarkGateCategoryResult {
        category: "latency".to_string(),
        pass,
        skipped: false,
        metrics,
    }
}

fn evaluate_token_cost_gate(
    baseline: &QueryBenchmarkReport,
    candidate: &QueryBenchmarkReport,
    thresholds: QueryBenchmarkGateThresholds,
) -> QueryBenchmarkGateCategoryResult {
    let max_tokens =
        baseline.avg_estimated_tokens * (1.0 + thresholds.token_cost_max_increase_ratio);
    let token_growth = candidate.avg_estimated_tokens - baseline.avg_estimated_tokens;
    let quality_uplift_required = token_growth <= 0.0 || has_quality_uplift(baseline, candidate);

    let metrics = vec![
        max_metric(
            "avg_estimated_tokens",
            max_tokens,
            baseline.avg_estimated_tokens,
            candidate.avg_estimated_tokens,
        ),
        QueryBenchmarkGateMetricResult {
            metric: "quality_uplift_for_token_growth".to_string(),
            threshold_kind: "bool".to_string(),
            threshold: if token_growth > 0.0 { 1.0 } else { 0.0 },
            baseline: quality_score(baseline),
            candidate: quality_score(candidate),
            pass: quality_uplift_required,
        },
    ];
    let pass = metrics.iter().all(|metric| metric.pass);

    QueryBenchmarkGateCategoryResult {
        category: "token_cost".to_string(),
        pass,
        skipped: false,
        metrics,
    }
}

fn min_metric(
    metric: &str,
    threshold: f32,
    baseline: f32,
    candidate: f32,
) -> QueryBenchmarkGateMetricResult {
    QueryBenchmarkGateMetricResult {
        metric: metric.to_string(),
        threshold_kind: "min".to_string(),
        threshold,
        baseline,
        candidate,
        pass: candidate >= threshold,
    }
}

fn max_metric(
    metric: &str,
    threshold: f32,
    baseline: f32,
    candidate: f32,
) -> QueryBenchmarkGateMetricResult {
    QueryBenchmarkGateMetricResult {
        metric: metric.to_string(),
        threshold_kind: "max".to_string(),
        threshold,
        baseline,
        candidate,
        pass: candidate <= threshold,
    }
}

fn skipped_gate(category: &str) -> QueryBenchmarkGateCategoryResult {
    QueryBenchmarkGateCategoryResult {
        category: category.to_string(),
        pass: true,
        skipped: true,
        metrics: Vec::new(),
    }
}

fn quality_score(report: &QueryBenchmarkReport) -> f32 {
    (report.recall_at_k + report.mrr_at_k + report.ndcg_at_k) / 3.0
}

fn has_quality_uplift(baseline: &QueryBenchmarkReport, candidate: &QueryBenchmarkReport) -> bool {
    candidate.recall_at_k > baseline.recall_at_k
        || candidate.mrr_at_k > baseline.mrr_at_k
        || candidate.ndcg_at_k > baseline.ndcg_at_k
}
