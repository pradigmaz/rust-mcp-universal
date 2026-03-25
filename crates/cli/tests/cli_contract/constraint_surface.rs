use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::json;
use tempfile::tempdir;

use crate::investigation_fixture::write_route_and_constraint_fixture;

#[test]
fn cli_constraint_evidence_returns_stage5_canonical_fields() {
    let project = tempdir().expect("temp dir");
    write_route_and_constraint_fixture(project.path());

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--json",
            "constraint-evidence",
            "--seed",
            "resolve_lab",
            "--seed-kind",
            "query",
            "--auto-index",
        ])
        .assert()
        .success();

    let payload: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("constraint payload");
    let item = &payload["items"][0];
    assert!(item["constraint_kind"].is_string());
    assert!(item["source_kind"].is_string());
    assert!(item["path"].is_string());
    assert!(item["line_start"].is_number());
    assert!(item["line_end"].is_number());
    assert!(item["excerpt"].is_string());
    assert!(item["confidence"].is_number());
    assert!(item["normalized_key"].is_string());
}

#[test]
fn cli_constraint_evidence_privacy_masks_stage5_canonical_fields() {
    let project = tempdir().expect("temp dir");
    write_route_and_constraint_fixture(project.path());

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--privacy-mode",
            "mask",
            "--json",
            "constraint-evidence",
            "--seed",
            "resolve_lab",
            "--seed-kind",
            "query",
            "--auto-index",
        ])
        .assert()
        .success();

    let payload: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("constraint payload");
    let item = &payload["items"][0];
    assert!(
        item["path"]
            .as_str()
            .is_some_and(|value| value.starts_with("<masked:"))
    );
    assert_eq!(item["excerpt"], json!("<redacted-content>"));
    assert_eq!(item["normalized_key"], json!("<redacted-content>"));
}
