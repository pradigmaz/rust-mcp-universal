use assert_cmd::cargo::cargo_bin_cmd;
use std::path::PathBuf;
use tempfile::tempdir;

use crate::investigation_fixture::{
    write_cluster_and_divergence_fixture, write_route_and_constraint_fixture,
    write_symbol_body_fixture,
};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

#[test]
fn symbol_body_returns_body_items() {
    let project = tempdir().expect("temp dir");
    write_symbol_body_fixture(project.path());

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--json",
            "symbol-body",
            "--seed",
            "inspect_body",
            "--seed-kind",
            "symbol",
            "--auto-index",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should contain JSON");
    assert_eq!(payload["capability_status"], serde_json::json!("supported"));
    assert!(payload["ambiguity_status"].is_string());
    assert!(
        payload["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(payload["items"][0]["resolution_kind"].is_string());
}

#[test]
fn route_trace_and_constraint_evidence_return_variant_data() {
    let project = tempdir().expect("temp dir");
    write_route_and_constraint_fixture(project.path());
    let project_path = project.path().to_str().expect("utf-8 path");

    let route_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "route-trace",
            "--seed",
            "resolve_lab",
            "--seed-kind",
            "query",
            "--auto-index",
        ])
        .assert()
        .success();
    let route_payload: serde_json::Value =
        serde_json::from_slice(&route_assert.get_output().stdout).expect("route payload");
    assert!(route_payload["best_route"]["segments"].is_array());
    assert!(route_payload["alternate_routes"].is_array());
    assert!(route_payload["unresolved_gaps"].is_array());

    let constraint_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
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
    let constraint_payload: serde_json::Value =
        serde_json::from_slice(&constraint_assert.get_output().stdout).expect("constraint payload");
    assert!(
        constraint_payload["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
}

#[test]
fn concept_cluster_and_divergence_report_emit_analysis_objects() {
    let project = tempdir().expect("temp dir");
    write_cluster_and_divergence_fixture(project.path());
    let project_path = project.path().to_str().expect("utf-8 path");

    let cluster_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "concept-cluster",
            "--seed",
            "origin_resolution",
            "--seed-kind",
            "query",
            "--auto-index",
        ])
        .assert()
        .success();
    let cluster_payload: serde_json::Value =
        serde_json::from_slice(&cluster_assert.get_output().stdout).expect("cluster payload");
    assert!(cluster_payload["cluster_summary"]["variant_count"].is_number());
    assert!(cluster_payload["cluster_summary"]["expansion_policy"].is_object());
    assert!(cluster_payload["variants"][0]["semantic_state"].is_string());
    assert!(cluster_payload["variants"][0]["score_model"].is_string());
    assert!(cluster_payload["variants"][0]["score_breakdown"].is_object());

    let contract_trace_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "contract-trace",
            "--seed",
            "origin_resolution",
            "--seed-kind",
            "query",
            "--auto-index",
        ])
        .assert()
        .success();
    let contract_trace_payload: serde_json::Value =
        serde_json::from_slice(&contract_trace_assert.get_output().stdout).expect("contract trace payload");
    assert!(contract_trace_payload["chain"].is_array());
    assert!(contract_trace_payload["contract_breaks"].is_array());
    assert!(contract_trace_payload["actionability"]["next_steps"].is_array());
    assert!(contract_trace_payload["manual_review_required"].is_boolean());

    let divergence_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "divergence-report",
            "--seed",
            "origin_resolution",
            "--seed-kind",
            "query",
            "--auto-index",
        ])
        .assert()
        .success();
    let divergence_payload: serde_json::Value =
        serde_json::from_slice(&divergence_assert.get_output().stdout).expect("divergence payload");
    assert!(divergence_payload["consensus_axes"].is_array());
    assert!(divergence_payload["divergence_axes"].is_array());
    assert!(divergence_payload["divergence_signals"].is_array());
    assert_eq!(
        divergence_payload["surface_kind"],
        serde_json::json!("divergence_explainability")
    );
    assert!(divergence_payload["overall_severity"].is_string());
    assert!(divergence_payload["manual_review_required"].is_boolean());
    assert!(divergence_payload["summary"].is_string());
    assert!(divergence_payload["shared_evidence"].is_array());
    assert!(divergence_payload["unknowns"].is_array());
    assert!(divergence_payload["recommended_followups"].is_array());
    if let Some(first_signal) = divergence_payload["divergence_signals"]
        .as_array()
        .and_then(|signals| signals.first())
    {
        assert!(first_signal["evidence_strength"].is_string());
        assert!(first_signal["classification_reason"].is_string());
    }
}

#[test]
fn investigation_cli_payloads_match_local_result_schemas() {
    let symbol_schema = super::load_schema("symbol_body.schema.json");
    let route_schema = super::load_schema("route_trace.schema.json");
    let constraint_schema = super::load_schema("constraint_evidence.schema.json");
    let cluster_schema = super::load_schema("concept_cluster.schema.json");
    let contract_trace_schema = super::load_schema("contract_trace.schema.json");
    let divergence_schema = super::load_schema("divergence_report.schema.json");

    super::validate_schema_keyword_coverage(&symbol_schema, "cli.symbol_body.schema");
    super::validate_schema_keyword_coverage(&route_schema, "cli.route_trace.schema");
    super::validate_schema_keyword_coverage(&constraint_schema, "cli.constraint.schema");
    super::validate_schema_keyword_coverage(&cluster_schema, "cli.cluster.schema");
    super::validate_schema_keyword_coverage(&contract_trace_schema, "cli.contract_trace.schema");
    super::validate_schema_keyword_coverage(&divergence_schema, "cli.divergence.schema");

    let symbol_project = tempdir().expect("temp dir");
    write_symbol_body_fixture(symbol_project.path());
    let symbol_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            symbol_project.path().to_str().expect("utf-8 path"),
            "--json",
            "symbol-body",
            "--seed",
            "inspect_body",
            "--seed-kind",
            "symbol",
            "--auto-index",
        ])
        .assert()
        .success();
    let symbol_payload: serde_json::Value =
        serde_json::from_slice(&symbol_assert.get_output().stdout).expect("symbol payload");
    super::assert_required_structure(&symbol_payload, &symbol_schema, "cli.symbol_body.payload");

    let route_project = tempdir().expect("temp dir");
    write_route_and_constraint_fixture(route_project.path());
    let route_project_path = route_project.path().to_str().expect("utf-8 path");

    let route_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            route_project_path,
            "--json",
            "route-trace",
            "--seed",
            "resolve_lab",
            "--seed-kind",
            "query",
            "--auto-index",
        ])
        .assert()
        .success();
    let route_payload: serde_json::Value =
        serde_json::from_slice(&route_assert.get_output().stdout).expect("route payload");
    super::assert_required_structure(&route_payload, &route_schema, "cli.route_trace.payload");

    let constraint_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            route_project_path,
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
    let constraint_payload: serde_json::Value =
        serde_json::from_slice(&constraint_assert.get_output().stdout).expect("constraint payload");
    super::assert_required_structure(
        &constraint_payload,
        &constraint_schema,
        "cli.constraint_evidence.payload",
    );

    let cluster_project = tempdir().expect("temp dir");
    write_cluster_and_divergence_fixture(cluster_project.path());
    let cluster_project_path = cluster_project.path().to_str().expect("utf-8 path");

    let cluster_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            cluster_project_path,
            "--json",
            "concept-cluster",
            "--seed",
            "origin_resolution",
            "--seed-kind",
            "query",
            "--auto-index",
        ])
        .assert()
        .success();
    let cluster_payload: serde_json::Value =
        serde_json::from_slice(&cluster_assert.get_output().stdout).expect("cluster payload");
    super::assert_required_structure(
        &cluster_payload,
        &cluster_schema,
        "cli.concept_cluster.payload",
    );

    let contract_trace_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            cluster_project_path,
            "--json",
            "contract-trace",
            "--seed",
            "origin_resolution",
            "--seed-kind",
            "query",
            "--auto-index",
        ])
        .assert()
        .success();
    let contract_trace_payload: serde_json::Value =
        serde_json::from_slice(&contract_trace_assert.get_output().stdout).expect("contract payload");
    super::assert_required_structure(
        &contract_trace_payload,
        &contract_trace_schema,
        "cli.contract_trace.payload",
    );

    let divergence_assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            cluster_project_path,
            "--json",
            "divergence-report",
            "--seed",
            "origin_resolution",
            "--seed-kind",
            "query",
            "--auto-index",
        ])
        .assert()
        .success();
    let divergence_payload: serde_json::Value =
        serde_json::from_slice(&divergence_assert.get_output().stdout).expect("divergence payload");
    super::assert_required_structure(
        &divergence_payload,
        &divergence_schema,
        "cli.divergence_report.payload",
    );
}

#[test]
fn route_trace_does_not_claim_supported_when_seed_path_is_missing_from_current_index_scope() {
    let temp = tempdir().expect("temp dir");
    let db_path = temp.path().join(".rmu/index.db");
    let root = workspace_root();
    let root_text = root.to_str().expect("utf-8 root");
    let db_text = db_path.to_str().expect("utf-8 db");

    cargo_bin_cmd!("rmu-cli")
        .args(["--project-path", root_text, "--db-path", db_text, "index"])
        .assert()
        .success();

    let fixture_seed = "baseline/investigation/fixtures/mixed_app/src/services/origin_service.rs";
    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            root_text,
            "--db-path",
            db_text,
            "--json",
            "route-trace",
            "--seed",
            fixture_seed,
            "--seed-kind",
            "path",
        ])
        .assert()
        .success();

    let payload: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("route payload");
    assert_ne!(payload["capability_status"], serde_json::json!("supported"));
    assert!(payload["best_route"]["segments"].is_array());
    assert!(
        payload["unresolved_gaps"]
            .as_array()
            .is_some_and(|gaps| !gaps.is_empty())
    );
}
