use std::fs;

use serde_json::json;

use super::contracts::assert_query_benchmark_baseline_contracts;
use super::*;

#[test]
fn query_benchmark_returns_metrics_and_matches_local_json_schemas() {
    let project_dir = temp_dir("rmu-mcp-tests-benchmark");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn benchmark_contract_symbol() -> i32 { 101 }\n",
    )
    .expect("write file");

    let dataset_path = project_dir.join("dataset.json");
    fs::write(
        &dataset_path,
        r#"{
            "queries": [
                {
                    "query": "benchmark_contract_symbol",
                    "qrels": [
                        {"path": "src/lib.rs", "relevance": 1.0}
                    ]
                }
            ]
        }"#,
    )
    .expect("write benchmark dataset");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "query_benchmark",
            "arguments": {
                "dataset_path": dataset_path.display().to_string(),
                "k": 5,
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("query_benchmark should succeed");

    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert!(structured["dataset_path"].is_string());
    assert_eq!(structured["query_count"], json!(1));
    let recall = structured["recall_at_k"]
        .as_f64()
        .expect("recall_at_k should be number");
    let mrr = structured["mrr_at_k"]
        .as_f64()
        .expect("mrr_at_k should be number");
    let ndcg = structured["ndcg_at_k"]
        .as_f64()
        .expect("ndcg_at_k should be number");
    assert!((0.0..=1.0).contains(&recall));
    assert!((0.0..=1.0).contains(&mrr));
    assert!((0.0..=1.0).contains(&ndcg));
    assert!(recall > 0.0);
    assert!(mrr > 0.0);
    assert!(ndcg > 0.0);
    assert!(structured["avg_estimated_tokens"].is_number());
    assert!(structured["latency_p50_ms"].is_number());
    assert!(structured["latency_p95_ms"].is_number());

    assert_query_benchmark_baseline_contracts(&result);

    let _ = fs::remove_dir_all(project_dir);
}
