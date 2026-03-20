use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

use super::super::write_query_benchmark_fixture;

#[test]
fn query_benchmark_accepts_stage0_style_summary_with_median_metrics() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let (dataset_path, baseline_path) = write_query_benchmark_fixture(project.path());

    std::fs::write(
        &baseline_path,
        r#"{
  "generated_at_utc": "2026-03-03T12:23:41Z",
  "benchmark_command": "rmu-cli --json query-benchmark ...",
  "runs_count": 2,
  "median_rule": "median_of_2_runs",
  "runs": [
    {
      "run": 1,
      "dataset_path": "query_benchmark_dataset.json",
      "k": 5,
      "query_count": 1,
      "recall_at_k": 0.2,
      "mrr_at_k": 0.2,
      "ndcg_at_k": 0.2,
      "avg_estimated_tokens": 1000.0,
      "latency_p50_ms": 15.0,
      "latency_p95_ms": 18.0
    },
    {
      "run": 2,
      "dataset_path": "query_benchmark_dataset.json",
      "k": 5,
      "query_count": 1,
      "recall_at_k": 0.3,
      "mrr_at_k": 0.3,
      "ndcg_at_k": 0.3,
      "avg_estimated_tokens": 1100.0,
      "latency_p50_ms": 17.0,
      "latency_p95_ms": 22.0
    }
  ],
  "median": {
    "recall_at_k": 0.25,
    "mrr_at_k": 0.25,
    "ndcg_at_k": 0.25,
    "avg_estimated_tokens": 1050.0,
    "latency_p50_ms": 16.0,
    "latency_p95_ms": 20.0
  }
}"#,
    )
    .expect("write stage0-style baseline summary");

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
    let recall = payload["baseline"]["metrics"]["recall_at_k"]
        .as_f64()
        .expect("baseline recall must be numeric");
    assert!((recall - 0.25).abs() < 1e-6);
}
