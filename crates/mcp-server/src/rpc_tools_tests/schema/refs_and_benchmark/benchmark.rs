use serde_json::json;

use super::super::*;

#[test]
fn query_benchmark_report_schema_supports_legacy_and_compare_branches() {
    let schema = load_schema("query_benchmark_report.schema.json");

    let legacy_payload = json!({
        "dataset_path": "/tmp/dataset.json",
        "k": 5,
        "query_count": 1,
        "recall_at_k": 1.0,
        "mrr_at_k": 1.0,
        "ndcg_at_k": 1.0,
        "avg_estimated_tokens": 42.0,
        "latency_p50_ms": 3.5,
        "latency_p95_ms": 6.7
    });
    assert_required_structure(&legacy_payload, &schema, "benchmark.oneOf.legacy");

    let compare_payload = json!({
        "mode": "baseline_vs_candidate",
        "dataset_path": "/tmp/dataset.json",
        "query_count": 1,
        "k": 5,
        "runs_count": 2,
        "median_rule": "median_of_2_runs",
        "baseline": {
            "path": "/tmp/baseline.json",
            "metrics": {
                "recall_at_k": 0.5,
                "mrr_at_k": 0.4,
                "ndcg_at_k": 0.3,
                "avg_estimated_tokens": 100.0,
                "latency_p50_ms": 20.0,
                "latency_p95_ms": 30.0
            }
        },
        "candidate": {
            "runs": [
                {
                    "dataset_path": "/tmp/dataset.json",
                    "k": 5,
                    "query_count": 1,
                    "recall_at_k": 0.6,
                    "mrr_at_k": 0.5,
                    "ndcg_at_k": 0.4,
                    "avg_estimated_tokens": 90.0,
                    "latency_p50_ms": 18.0,
                    "latency_p95_ms": 28.0
                },
                {
                    "dataset_path": "/tmp/dataset.json",
                    "k": 5,
                    "query_count": 1,
                    "recall_at_k": 0.7,
                    "mrr_at_k": 0.6,
                    "ndcg_at_k": 0.5,
                    "avg_estimated_tokens": 85.0,
                    "latency_p50_ms": 17.0,
                    "latency_p95_ms": 27.0
                }
            ],
            "median": {
                "recall_at_k": 0.65,
                "mrr_at_k": 0.55,
                "ndcg_at_k": 0.45,
                "avg_estimated_tokens": 87.5,
                "latency_p50_ms": 17.5,
                "latency_p95_ms": 27.5
            }
        },
        "diff": {
            "recall_at_k": {
                "baseline": 0.5,
                "candidate": 0.65,
                "delta_abs": 0.15,
                "delta_pct": 30.0,
                "direction": "higher_is_better"
            },
            "mrr_at_k": {
                "baseline": 0.4,
                "candidate": 0.55,
                "delta_abs": 0.15,
                "delta_pct": 37.5,
                "direction": "higher_is_better"
            },
            "ndcg_at_k": {
                "baseline": 0.3,
                "candidate": 0.45,
                "delta_abs": 0.15,
                "delta_pct": 50.0,
                "direction": "higher_is_better"
            },
            "avg_estimated_tokens": {
                "baseline": 100.0,
                "candidate": 87.5,
                "delta_abs": -12.5,
                "delta_pct": -12.5,
                "direction": "lower_is_better"
            },
            "latency_p50_ms": {
                "baseline": 20.0,
                "candidate": 17.5,
                "delta_abs": -2.5,
                "delta_pct": -12.5,
                "direction": "lower_is_better"
            },
            "latency_p95_ms": {
                "baseline": 30.0,
                "candidate": 27.5,
                "delta_abs": -2.5,
                "delta_pct": -8.333333,
                "direction": "lower_is_better"
            }
        },
        "thresholds": {
            "path": "/tmp/thresholds.json",
            "configured": {
                "min": {
                    "recall_at_k": 0.1
                },
                "max": {
                    "latency_p95_ms": 100.0
                }
            },
            "run_evaluations": [
                {
                    "passed": true,
                    "checks": [
                        {
                            "metric": "recall_at_k",
                            "comparator": ">=",
                            "actual": 0.6,
                            "threshold": 0.1,
                            "passed": true
                        }
                    ]
                },
                null
            ],
            "candidate_median_evaluation": {
                "passed": true,
                "checks": []
            },
            "passed": true
        },
        "enforce_gates": false
    });
    assert_required_structure(&compare_payload, &schema, "benchmark.oneOf.compare");

    let mut invalid_compare = compare_payload.clone();
    invalid_compare["mode"] = json!("unexpected_mode");
    assert_schema_rejects(
        &invalid_compare,
        &schema,
        "benchmark.oneOf.compare.invalid_mode",
    );
}
