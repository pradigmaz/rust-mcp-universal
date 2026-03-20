use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

use super::super::write_query_benchmark_fixture;

#[test]
fn query_benchmark_baseline_mode_outputs_machine_readable_diff_json() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let (dataset_path, baseline_path) = write_query_benchmark_fixture(project.path());

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
    assert_eq!(payload["mode"], serde_json::json!("baseline_vs_candidate"));
    assert_eq!(payload["runs_count"], serde_json::json!(1));
    assert!(payload["baseline"]["metrics"]["recall_at_k"].is_number());
    assert!(payload["candidate"]["median"]["recall_at_k"].is_number());
    assert!(payload["diff"]["recall_at_k"]["delta_abs"].is_number());
    assert!(
        payload["candidate"]["runs"]
            .as_array()
            .is_some_and(|runs| runs.len() == 1)
    );
}
