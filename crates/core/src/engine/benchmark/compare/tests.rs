use super::{build_diff_report, evaluate_gates, median_report};
use crate::model::{QueryBenchmarkGateThresholds, QueryBenchmarkReport};

fn report(
    recall_at_k: f32,
    mrr_at_k: f32,
    ndcg_at_k: f32,
    avg_estimated_tokens: f32,
    latency_p50_ms: f32,
    latency_p95_ms: f32,
) -> QueryBenchmarkReport {
    QueryBenchmarkReport {
        dataset_path: "dataset.json".to_string(),
        k: 10,
        query_count: 12,
        recall_at_k,
        mrr_at_k,
        ndcg_at_k,
        avg_estimated_tokens,
        latency_p50_ms,
        latency_p95_ms,
    }
}

#[test]
fn median_report_uses_median_per_metric() {
    let runs = vec![
        report(0.2, 0.1, 0.3, 200.0, 40.0, 70.0),
        report(0.8, 0.7, 0.9, 600.0, 10.0, 20.0),
        report(0.5, 0.4, 0.6, 400.0, 20.0, 30.0),
    ];

    let median = median_report(&runs);
    assert!((median.recall_at_k - 0.5).abs() < 1e-6);
    assert!((median.mrr_at_k - 0.4).abs() < 1e-6);
    assert!((median.ndcg_at_k - 0.6).abs() < 1e-6);
    assert!((median.avg_estimated_tokens - 400.0).abs() < 1e-6);
    assert!((median.latency_p50_ms - 20.0).abs() < 1e-6);
    assert!((median.latency_p95_ms - 30.0).abs() < 1e-6);
}

#[test]
fn diff_report_computes_delta_and_ratio() {
    let baseline = report(0.5, 0.4, 0.3, 100.0, 20.0, 30.0);
    let candidate = report(0.6, 0.2, 0.3, 150.0, 25.0, 45.0);

    let diff = build_diff_report(&baseline, &candidate);
    assert!((diff.recall_at_k.delta - 0.1).abs() < 1e-6);
    assert!((diff.mrr_at_k.delta + 0.2).abs() < 1e-6);
    assert!((diff.avg_estimated_tokens.delta - 50.0).abs() < 1e-6);
    assert_eq!(diff.ndcg_at_k.delta_ratio, Some(0.0));
    assert_eq!(diff.latency_p95_ms.delta_ratio, Some(0.5));
}

#[test]
fn gate_evaluation_fail_fast_skips_remaining_categories() {
    let baseline = report(0.8, 0.7, 0.6, 100.0, 20.0, 30.0);
    let candidate = report(0.8, 0.7, 0.6, 100.0, 20.0, 30.0);
    let thresholds = QueryBenchmarkGateThresholds {
        quality_max_drop_ratio: -0.1,
        latency_p50_max_increase_ratio: 1.0,
        latency_p95_max_increase_ratio: 1.0,
        latency_p95_max_increase_ms: 100.0,
        token_cost_max_increase_ratio: 1.0,
    };

    let gates = evaluate_gates(&baseline, &candidate, thresholds, true);
    assert!(!gates.overall_pass);
    assert_eq!(gates.first_failed_category.as_deref(), Some("quality"));
    assert!(!gates.categories[0].pass);
    assert!(!gates.categories[0].skipped);
    assert!(gates.categories[1].skipped);
    assert!(gates.categories[2].skipped);
}

#[test]
fn gate_evaluation_blocks_latency_p95_regression_above_threshold() {
    let baseline = report(0.8, 0.7, 0.6, 100.0, 20.0, 30.0);
    let candidate = report(0.8, 0.7, 0.6, 100.0, 20.0, 40.0);
    let thresholds = QueryBenchmarkGateThresholds {
        quality_max_drop_ratio: 0.1,
        latency_p50_max_increase_ratio: 1.0,
        latency_p95_max_increase_ratio: 0.2,
        latency_p95_max_increase_ms: 5.0,
        token_cost_max_increase_ratio: 1.0,
    };

    let gates = evaluate_gates(&baseline, &candidate, thresholds, true);
    assert!(!gates.overall_pass);
    assert_eq!(gates.first_failed_category.as_deref(), Some("latency"));
    assert!(!gates.categories[1].pass);
    assert!(!gates.categories[1].skipped);
    assert!(gates.categories[2].skipped);
}

#[test]
fn gate_evaluation_blocks_token_growth_without_quality_uplift() {
    let baseline = report(0.8, 0.7, 0.6, 100.0, 20.0, 30.0);
    let candidate = report(0.8, 0.7, 0.6, 110.0, 20.0, 30.0);
    let thresholds = QueryBenchmarkGateThresholds {
        quality_max_drop_ratio: 0.1,
        latency_p50_max_increase_ratio: 1.0,
        latency_p95_max_increase_ratio: 1.0,
        latency_p95_max_increase_ms: 100.0,
        token_cost_max_increase_ratio: 0.2,
    };

    let gates = evaluate_gates(&baseline, &candidate, thresholds, false);
    assert!(!gates.overall_pass);
    assert_eq!(gates.first_failed_category.as_deref(), Some("token_cost"));
    assert!(!gates.categories[2].pass);
    let has_uplift_metric_failure = gates.categories[2]
        .metrics
        .iter()
        .any(|metric| metric.metric == "quality_uplift_for_token_growth" && !metric.pass);
    assert!(has_uplift_metric_failure);
}

#[test]
fn gate_evaluation_allows_token_growth_with_quality_uplift_within_threshold() {
    let baseline = report(0.8, 0.7, 0.6, 100.0, 20.0, 30.0);
    let candidate = report(0.81, 0.7, 0.6, 110.0, 20.0, 30.0);
    let thresholds = QueryBenchmarkGateThresholds {
        quality_max_drop_ratio: 0.1,
        latency_p50_max_increase_ratio: 1.0,
        latency_p95_max_increase_ratio: 1.0,
        latency_p95_max_increase_ms: 100.0,
        token_cost_max_increase_ratio: 0.2,
    };

    let gates = evaluate_gates(&baseline, &candidate, thresholds, false);
    assert!(gates.overall_pass);
    assert!(gates.first_failed_category.is_none());
}
