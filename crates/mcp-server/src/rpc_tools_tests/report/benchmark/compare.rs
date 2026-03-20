use std::fs;

use serde_json::json;

use super::contracts::assert_query_benchmark_compare_contracts;
use super::*;

#[test]
fn query_benchmark_compare_mode_returns_diff_and_matches_local_json_schemas() {
    let project_dir = temp_dir("rmu-mcp-tests-benchmark-compare");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn benchmark_compare_symbol() -> i32 { 202 }\n",
    )
    .expect("write file");

    let dataset_path = project_dir.join("dataset.json");
    fs::write(
        &dataset_path,
        r#"{
            "queries": [
                {
                    "query": "benchmark_compare_symbol",
                    "qrels": [
                        {"path": "src/lib.rs", "relevance": 1.0}
                    ]
                }
            ]
        }"#,
    )
    .expect("write benchmark dataset");

    let baseline_path = project_dir.join("baseline.json");
    fs::write(
        &baseline_path,
        r#"{
            "recall_at_k": 0.01,
            "mrr_at_k": 0.01,
            "ndcg_at_k": 0.01,
            "avg_estimated_tokens": 2000.0,
            "latency_p50_ms": 500.0,
            "latency_p95_ms": 800.0
        }"#,
    )
    .expect("write baseline");

    let thresholds_path = project_dir.join("thresholds.json");
    fs::write(
        &thresholds_path,
        r#"{
            "min": {
                "recall_at_k": 0.0
            },
            "max": {
                "latency_p95_ms": 2000.0
            }
        }"#,
    )
    .expect("write thresholds");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "query_benchmark",
            "arguments": {
                "dataset_path": dataset_path.display().to_string(),
                "k": 5,
                "limit": 5,
                "auto_index": true,
                "baseline": baseline_path.display().to_string(),
                "thresholds": thresholds_path.display().to_string(),
                "runs": 2
            }
        })),
        &mut state,
    )
    .expect("query_benchmark compare mode should succeed");

    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert_eq!(structured["mode"], json!("baseline_vs_candidate"));
    assert_eq!(structured["runs_count"], json!(2));
    assert!(structured["baseline"]["metrics"]["recall_at_k"].is_number());
    assert!(structured["candidate"]["median"]["recall_at_k"].is_number());
    assert!(
        structured["candidate"]["runs"]
            .as_array()
            .is_some_and(|runs| runs.len() == 2)
    );
    assert!(structured["diff"]["recall_at_k"]["delta_abs"].is_number());
    assert!(structured["thresholds"]["run_evaluations"].is_array());
    assert_eq!(structured["enforce_gates"], json!(false));

    assert_query_benchmark_compare_contracts(&result);

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn query_benchmark_compare_mode_enforce_gates_returns_fail_fast_error() {
    let project_dir = temp_dir("rmu-mcp-tests-benchmark-compare-fail-fast");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn benchmark_compare_fail_symbol() -> i32 { 303 }\n",
    )
    .expect("write file");

    let dataset_path = project_dir.join("dataset.json");
    fs::write(
        &dataset_path,
        r#"{
            "queries": [
                {
                    "query": "benchmark_compare_fail_symbol",
                    "qrels": [
                        {"path": "src/lib.rs", "relevance": 1.0}
                    ]
                }
            ]
        }"#,
    )
    .expect("write benchmark dataset");

    let baseline_path = project_dir.join("baseline.json");
    fs::write(
        &baseline_path,
        r#"{
            "recall_at_k": 0.0,
            "mrr_at_k": 0.0,
            "ndcg_at_k": 0.0,
            "avg_estimated_tokens": 5000.0,
            "latency_p50_ms": 1000.0,
            "latency_p95_ms": 1500.0
        }"#,
    )
    .expect("write baseline");

    let thresholds_path = project_dir.join("thresholds.json");
    fs::write(
        &thresholds_path,
        r#"{
            "min": {
                "recall_at_k": 1.1
            }
        }"#,
    )
    .expect("write thresholds");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let err = handle_tool_call(
        Some(json!({
            "name": "query_benchmark",
            "arguments": {
                "dataset_path": dataset_path.display().to_string(),
                "k": 5,
                "limit": 5,
                "auto_index": true,
                "baseline": baseline_path.display().to_string(),
                "thresholds": thresholds_path.display().to_string(),
                "runs": 2,
                "enforce_gates": true
            }
        })),
        &mut state,
    )
    .expect_err("query_benchmark compare mode should fail-fast when gates fail");

    assert!(
        err.to_string()
            .contains("query_benchmark fail-fast at run 1")
    );
    let _ = fs::remove_dir_all(project_dir);
}
