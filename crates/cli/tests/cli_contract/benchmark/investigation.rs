use assert_cmd::cargo::cargo_bin_cmd;
use std::path::PathBuf;
use tempfile::tempdir;

use crate::investigation_fixture::write_investigation_benchmark_fixture;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

#[test]
fn investigation_benchmark_reports_metrics_and_threshold_verdict() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let (dataset_path, thresholds_path) = write_investigation_benchmark_fixture(project.path());

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "investigation-benchmark",
            "--dataset",
            dataset_path.to_str().expect("utf-8 dataset path"),
            "--thresholds",
            thresholds_path.to_str().expect("utf-8 thresholds path"),
            "--auto-index",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("benchmark output should be JSON");
    assert!(
        payload["case_count"]
            .as_u64()
            .is_some_and(|count| count >= 4)
    );
    assert!(
        payload["per_tool_metrics"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(payload["per_tool_metrics"].as_array().is_some_and(|items| {
        items.iter().all(|item| {
            item["tool"].is_string()
                && item["case_count"].is_number()
                && item["passed_cases"].is_number()
                && item["pass_rate"].is_number()
                && item["unsupported_case_rate"].is_number()
                && item["latency_p50_ms"].is_number()
                && item["latency_p95_ms"].is_number()
        })
    }));
    assert!(payload["cases"].as_array().is_some_and(|cases| {
        cases.iter().all(|case| {
            case["id"].is_string()
                && case["tool"].is_string()
                && case["fixture"].is_string()
                && case["pass"].is_boolean()
                && case["assertion_pass_count"].is_number()
                && case["assertion_total_count"].is_number()
                && case["capability_status"].is_string()
                && case["expected_capability_status"].is_string()
                && case["unsupported_sources"].is_array()
                && case["privacy_failures"].is_number()
                && case["latency_ms"].is_number()
                && case["notes"].is_array()
        })
    }));
    assert!(payload["unsupported_behavior_summary"].is_array());
    assert!(payload["privacy_failures"].is_number());
    assert!(payload["threshold_verdict"].is_object());
}

#[test]
fn investigation_benchmark_enforce_gates_requires_thresholds() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let (dataset_path, _) = write_investigation_benchmark_fixture(project.path());

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "investigation-benchmark",
            "--dataset",
            dataset_path.to_str().expect("utf-8 dataset path"),
            "--auto-index",
            "--enforce-gates",
        ])
        .assert()
        .code(1);

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("error output should be JSON");
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert!(
        payload["error"]
            .as_str()
            .unwrap_or_default()
            .contains("--enforce-gates requires --thresholds")
    );
}

#[test]
fn investigation_benchmark_passes_curated_acceptance_thresholds_on_repo_fixture() {
    let temp = tempdir().expect("temp dir");
    let db_path = temp.path().join(".rmu/index.db");
    let root = workspace_root();
    let thresholds_path = root.join("baseline/investigation/stage9/thresholds.json");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            root.to_str().expect("utf-8 root"),
            "--db-path",
            db_path.to_str().expect("utf-8 db"),
            "--json",
            "investigation-benchmark",
            "--dataset",
            root.join("baseline/investigation/stage9/investigation_dataset.json")
                .to_str()
                .expect("utf-8 dataset"),
            "--thresholds",
            thresholds_path.to_str().expect("utf-8 thresholds"),
            "--auto-index",
        ])
        .assert()
        .success();

    let payload: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("benchmark payload");
    let thresholds: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&thresholds_path).expect("read thresholds"))
            .expect("parse thresholds");

    assert_eq!(
        payload["privacy_failures"],
        serde_json::json!(thresholds["privacy_failures"].as_u64().unwrap_or(0))
    );

    let metrics = payload["per_tool_metrics"]
        .as_array()
        .expect("per_tool_metrics should be array");
    let symbol_body = metrics
        .iter()
        .find(|item| item["tool"] == "symbol_body")
        .expect("symbol_body metrics");
    assert!(
        symbol_body["pass_rate"]
            .as_f64()
            .expect("symbol_body pass_rate")
            >= thresholds["symbol_body_supported_success"]
                .as_f64()
                .expect("symbol body threshold")
    );
    assert!(
        symbol_body["body_anchor_precision"]
            .as_f64()
            .expect("body_anchor_precision")
            >= thresholds["body_anchor_precision_min"]
                .as_f64()
                .expect("body anchor threshold")
    );

    let explain_threshold = thresholds["explain_evidence_coverage_min"]
        .as_f64()
        .expect("explain_evidence_coverage_min threshold");
    for metric in metrics {
        assert!(
            metric["explain_evidence_coverage"]
                .as_f64()
                .expect("explain_evidence_coverage")
                >= explain_threshold,
            "tool {} explain_evidence_coverage below threshold",
            metric["tool"].as_str().unwrap_or("<unknown>")
        );
    }
}

#[test]
fn investigation_benchmark_compare_mode_emits_machine_readable_diff() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let (dataset_path, thresholds_path) = write_investigation_benchmark_fixture(project.path());
    let baseline_report_path = project.path().join("baseline_report.json");

    let baseline_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "investigation-benchmark",
            "--dataset",
            dataset_path.to_str().expect("utf-8 dataset path"),
            "--thresholds",
            thresholds_path.to_str().expect("utf-8 thresholds path"),
            "--auto-index",
        ])
        .assert()
        .success();
    std::fs::write(&baseline_report_path, &baseline_assert.get_output().stdout)
        .expect("write baseline report");

    let compare_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "investigation-benchmark",
            "--dataset",
            dataset_path.to_str().expect("utf-8 dataset path"),
            "--baseline-report",
            baseline_report_path
                .to_str()
                .expect("utf-8 baseline report path"),
            "--thresholds",
            thresholds_path.to_str().expect("utf-8 thresholds path"),
            "--auto-index",
        ])
        .assert()
        .success();

    let payload: serde_json::Value =
        serde_json::from_slice(&compare_assert.get_output().stdout).expect("compare payload");
    assert_eq!(
        payload["diff"]["baseline_case_count"],
        payload["case_count"]
    );
    assert_eq!(payload["diff"]["current_case_count"], payload["case_count"]);
    assert!(payload["diff"]["per_tool_deltas"].is_array());
    assert!(payload["diff"]["regressed_metrics"].is_array());
    assert!(payload["diff"]["regression_failures"].is_array());
}

#[test]
fn investigation_benchmark_compare_mode_fails_on_metric_regression_when_gates_enforced() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let (dataset_path, thresholds_path) = write_investigation_benchmark_fixture(project.path());
    let baseline_report_path = project.path().join("baseline_report.json");

    let baseline_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "investigation-benchmark",
            "--dataset",
            dataset_path.to_str().expect("utf-8 dataset path"),
            "--thresholds",
            thresholds_path.to_str().expect("utf-8 thresholds path"),
            "--auto-index",
        ])
        .assert()
        .success();
    let mut baseline_report: serde_json::Value =
        serde_json::from_slice(&baseline_assert.get_output().stdout).expect("baseline payload");
    let symbol_body = baseline_report["per_tool_metrics"]
        .as_array_mut()
        .expect("per_tool_metrics")
        .iter_mut()
        .find(|metric| metric["tool"] == "symbol_body")
        .expect("symbol_body metric");
    symbol_body["pass_rate"] = serde_json::json!(1.1);
    std::fs::write(
        &baseline_report_path,
        serde_json::to_vec_pretty(&baseline_report).expect("baseline report bytes"),
    )
    .expect("write baseline report");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "investigation-benchmark",
            "--dataset",
            dataset_path.to_str().expect("utf-8 dataset path"),
            "--baseline-report",
            baseline_report_path
                .to_str()
                .expect("utf-8 baseline report path"),
            "--thresholds",
            thresholds_path.to_str().expect("utf-8 thresholds path"),
            "--auto-index",
            "--enforce-gates",
        ])
        .assert()
        .code(1);

    let payload: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("error payload");
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert!(
        payload["error"]
            .as_str()
            .is_some_and(|message| message.contains("regressed"))
    );
}
