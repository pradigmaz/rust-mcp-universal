use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

use super::super::write_query_benchmark_fixture;

#[test]
fn query_benchmark_enforce_gates_triggers_fail_fast_error() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let (dataset_path, baseline_path) = write_query_benchmark_fixture(project.path());
    let thresholds_path = project.path().join("release_rollback_thresholds.json");
    std::fs::write(
        &thresholds_path,
        r#"{
  "min": {
    "recall_at_k": 1.10
  }
}"#,
    )
    .expect("write thresholds");

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
            "3",
            "--enforce-gates",
            "--auto-index",
        ])
        .assert()
        .code(1);

    assert!(assert.get_output().stderr.is_empty());
    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("error output should be JSON");
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert_eq!(payload["code"], serde_json::json!("E_RUNTIME"));
    assert!(
        payload["error"]
            .as_str()
            .unwrap_or_default()
            .contains("query-benchmark fail-fast at run 1")
    );
}

#[test]
fn query_benchmark_enforce_gates_fails_on_latency_p95_regression() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let (dataset_path, baseline_path) = write_query_benchmark_fixture(project.path());
    let thresholds_path = project.path().join("release_rollback_thresholds_p95.json");
    std::fs::write(
        &thresholds_path,
        r#"{
  "max": {
    "latency_p95_ms": 0.0
  }
}"#,
    )
    .expect("write thresholds");

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
            "--enforce-gates",
            "--auto-index",
        ])
        .assert()
        .code(1);

    assert!(assert.get_output().stderr.is_empty());
    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("error output should be JSON");
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert_eq!(payload["code"], serde_json::json!("E_RUNTIME"));
    assert!(
        payload["error"]
            .as_str()
            .unwrap_or_default()
            .contains("latency_p95_ms <=")
    );
}

#[test]
fn query_benchmark_enforce_gates_blocks_token_growth_without_quality_uplift() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let (dataset_path, baseline_path) = write_query_benchmark_fixture(project.path());
    std::fs::write(
        &baseline_path,
        r#"{
  "median": {
    "recall_at_k": 1.0,
    "mrr_at_k": 1.0,
    "ndcg_at_k": 1.0,
    "avg_estimated_tokens": 1.0,
    "latency_p50_ms": 15.0,
    "latency_p95_ms": 20.0
  }
}"#,
    )
    .expect("override baseline");
    let thresholds_path = project
        .path()
        .join("release_rollback_thresholds_token_guard.json");
    std::fs::write(
        &thresholds_path,
        r#"{
  "max": {
    "avg_estimated_tokens": 10000.0
  }
}"#,
    )
    .expect("write thresholds");

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
            "--enforce-gates",
            "--auto-index",
        ])
        .assert()
        .code(1);

    assert!(assert.get_output().stderr.is_empty());
    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("error output should be JSON");
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert_eq!(payload["code"], serde_json::json!("E_RUNTIME"));
    assert!(
        payload["error"]
            .as_str()
            .unwrap_or_default()
            .contains("token_cost_requires_quality_uplift")
    );
}
