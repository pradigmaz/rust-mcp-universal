use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

use super::super::{write_query_benchmark_fixture, write_utf8_bom_json};

#[test]
fn query_benchmark_accepts_utf8_bom_baseline_and_thresholds_files() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let (dataset_path, baseline_path) = write_query_benchmark_fixture(project.path());
    let thresholds_path = project.path().join("release_rollback_thresholds.json");

    write_utf8_bom_json(
        &baseline_path,
        r#"{
  "generated_at_utc": "2026-03-03T12:23:41Z",
  "median": {
    "recall_at_k": 0.2,
    "mrr_at_k": 0.2,
    "ndcg_at_k": 0.2,
    "avg_estimated_tokens": 1200.0,
    "latency_p50_ms": 15.0,
    "latency_p95_ms": 20.0
  }
}"#,
    );
    write_utf8_bom_json(
        &thresholds_path,
        r#"{
  "generated_at_utc": "2026-03-03T12:24:07Z",
  "min": {
    "recall_at_k": 0.01
  },
  "max": {
    "latency_p95_ms": 500.0
  }
}"#,
    );

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "query-benchmark",
            "--dataset",
            dataset_path.to_str().expect("utf-8 dataset path"),
            "--baseline",
            baseline_path.to_str().expect("utf-8 baseline path"),
            "--thresholds",
            thresholds_path.to_str().expect("utf-8 thresholds path"),
            "--runs",
            "1",
            "--k",
            "5",
            "--limit",
            "10",
            "--auto-index",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("query-benchmark should output JSON");
    assert!(payload["baseline"]["metrics"]["recall_at_k"].is_number());
    assert!(payload["thresholds"]["configured"]["min"]["recall_at_k"].is_number());
}
