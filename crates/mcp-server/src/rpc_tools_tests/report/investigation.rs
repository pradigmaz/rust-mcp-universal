use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::*;
use crate::ServerState;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

#[test]
fn symbol_body_tool_returns_items() {
    let project_dir = temp_dir("rmu-mcp-tests-symbol-body");
    write_symbol_body_fixture(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "symbol_body",
            "arguments": {
                "seed": "inspect_body",
                "seed_kind": "symbol",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("symbol_body should succeed");

    assert_eq!(result["isError"], json!(false));
    assert!(
        result["structuredContent"]["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(result["structuredContent"]["ambiguity_status"].is_string());
    assert!(result["structuredContent"]["items"][0]["resolution_kind"].is_string());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn route_and_constraint_tools_return_analysis_payloads() {
    let project_dir = temp_dir("rmu-mcp-tests-route-constraint");
    write_route_and_constraint_fixture(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let route_result = handle_tool_call(
        Some(json!({
            "name": "route_trace",
            "arguments": {
                "seed": "resolve_lab",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("route_trace should succeed");
    assert!(route_result["structuredContent"]["best_route"]["segments"].is_array());
    assert!(route_result["structuredContent"]["alternate_routes"].is_array());
    assert!(route_result["structuredContent"]["unresolved_gaps"].is_array());

    let constraint_result = handle_tool_call(
        Some(json!({
            "name": "constraint_evidence",
            "arguments": {
                "seed": "resolve_lab",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("constraint_evidence should succeed");
    assert!(
        constraint_result["structuredContent"]["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn concept_cluster_and_divergence_tools_return_expected_shapes() {
    let project_dir = temp_dir("rmu-mcp-tests-cluster-divergence");
    write_cluster_and_divergence_fixture(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let cluster_result = handle_tool_call(
        Some(json!({
            "name": "concept_cluster",
            "arguments": {
                "seed": "origin_resolution",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("concept_cluster should succeed");
    assert!(cluster_result["structuredContent"]["cluster_summary"]["variant_count"].is_number());
    assert!(cluster_result["structuredContent"]["cluster_summary"]["expansion_policy"].is_object());
    assert!(cluster_result["structuredContent"]["variants"][0]["semantic_state"].is_string());
    assert!(cluster_result["structuredContent"]["variants"][0]["score_model"].is_string());
    assert!(cluster_result["structuredContent"]["variants"][0]["score_breakdown"].is_object());

    let divergence_result = handle_tool_call(
        Some(json!({
            "name": "divergence_report",
            "arguments": {
                "seed": "origin_resolution",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("divergence_report should succeed");
    assert!(divergence_result["structuredContent"]["consensus_axes"].is_array());
    assert!(divergence_result["structuredContent"]["divergence_axes"].is_array());
    assert!(divergence_result["structuredContent"]["divergence_signals"].is_array());
    assert_eq!(
        divergence_result["structuredContent"]["surface_kind"],
        json!("divergence_explainability")
    );
    assert!(divergence_result["structuredContent"]["overall_severity"].is_string());
    assert!(divergence_result["structuredContent"]["manual_review_required"].is_boolean());
    assert!(divergence_result["structuredContent"]["summary"].is_string());
    assert!(divergence_result["structuredContent"]["shared_evidence"].is_array());
    assert!(divergence_result["structuredContent"]["unknowns"].is_array());
    assert!(divergence_result["structuredContent"]["recommended_followups"].is_array());
    if let Some(first_signal) = divergence_result["structuredContent"]["divergence_signals"]
        .as_array()
        .and_then(|signals| signals.first())
    {
        assert!(first_signal["evidence_strength"].is_string());
        assert!(first_signal["classification_reason"].is_string());
    }

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn preflight_tool_returns_structured_status_payload() {
    let project_dir = temp_dir("rmu-mcp-tests-preflight");
    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "preflight",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("preflight should succeed");

    assert_eq!(result["isError"], json!(false));
    assert!(result["structuredContent"]["status"].is_string());
    assert!(result["structuredContent"]["running_binary_version"].is_string());
    assert!(result["structuredContent"]["running_binary_stale"].is_boolean());
    assert_eq!(
        result["structuredContent"]["binding_status"],
        json!("bound")
    );
    assert_eq!(result["structuredContent"]["binding_source"], json!("cli"));
    assert!(result["structuredContent"]["resolved_project_path"].is_string());
    assert!(result["structuredContent"]["resolved_db_path"].is_string());
    assert_eq!(result["structuredContent"]["db_pinned"], json!(true));
    assert!(result["structuredContent"]["binding_errors"].is_array());
    assert!(
        result["structuredContent"]
            .get("stale_process_probe_binary_path")
            .is_none()
            || result["structuredContent"]["stale_process_probe_binary_path"].is_string()
    );
    assert!(result["structuredContent"]["same_binary_other_pids"].is_array());
    assert!(result["structuredContent"]["warnings"].is_array());
    assert!(result["structuredContent"]["errors"].is_array());
    assert!(result["structuredContent"]["safe_recovery_hint"].is_string());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn preflight_tool_reports_unbound_binding_without_touching_cwd_project() {
    let mut state = ServerState::new(None, None);
    let result = handle_tool_call(
        Some(json!({
            "name": "preflight",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("preflight should succeed even when unbound");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["binding_status"],
        json!("unbound")
    );
    assert!(result["structuredContent"]["resolved_project_path"].is_null());
    assert!(result["structuredContent"]["resolved_db_path"].is_null());
    assert!(result["structuredContent"]["binding_errors"].is_array());
}

#[test]
fn route_trace_tool_auto_indexes_repo_fixture_with_mixed_scope_when_needed() {
    let temp = temp_dir("rmu-mcp-tests-route-trace-repo-fixture");
    let db_path = temp.join(".rmu/index.db");
    let root = workspace_root();
    let mut state = state_for(root.clone(), Some(db_path));

    let result = handle_tool_call(
        Some(json!({
            "name": "route_trace",
            "arguments": {
                "seed": "baseline/investigation/fixtures/mixed_app/src/services/origin_service.rs",
                "seed_kind": "path",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("route_trace on repo fixture should succeed");

    assert_eq!(result["isError"], json!(false));
    assert!(result["structuredContent"]["best_route"]["segments"].is_array());

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn investigation_tool_payloads_match_local_json_schemas() {
    let symbol_schema = load_schema("symbol_body.schema.json");
    let symbol_envelope = load_schema("mcp_symbol_body_tool_result.schema.json");
    let route_schema = load_schema("route_trace.schema.json");
    let route_envelope = load_schema("mcp_route_trace_tool_result.schema.json");
    let constraint_schema = load_schema("constraint_evidence.schema.json");
    let constraint_envelope = load_schema("mcp_constraint_evidence_tool_result.schema.json");
    let cluster_schema = load_schema("concept_cluster.schema.json");
    let cluster_envelope = load_schema("mcp_concept_cluster_tool_result.schema.json");
    let divergence_schema = load_schema("divergence_report.schema.json");
    let divergence_envelope = load_schema("mcp_divergence_report_tool_result.schema.json");

    validate_schema_keyword_coverage(&symbol_schema, "investigation.symbol_body.schema")
        .expect("keywords");
    validate_schema_keyword_coverage(&symbol_envelope, "investigation.symbol_body.envelope")
        .expect("keywords");
    validate_schema_keyword_coverage(&route_schema, "investigation.route_trace.schema")
        .expect("keywords");
    validate_schema_keyword_coverage(&route_envelope, "investigation.route_trace.envelope")
        .expect("keywords");
    validate_schema_keyword_coverage(&constraint_schema, "investigation.constraint.schema")
        .expect("keywords");
    validate_schema_keyword_coverage(&constraint_envelope, "investigation.constraint.envelope")
        .expect("keywords");
    validate_schema_keyword_coverage(&cluster_schema, "investigation.cluster.schema")
        .expect("keywords");
    validate_schema_keyword_coverage(&cluster_envelope, "investigation.cluster.envelope")
        .expect("keywords");
    validate_schema_keyword_coverage(&divergence_schema, "investigation.divergence.schema")
        .expect("keywords");
    validate_schema_keyword_coverage(&divergence_envelope, "investigation.divergence.envelope")
        .expect("keywords");

    let symbol_project = temp_dir("rmu-mcp-tests-schema-symbol-body");
    write_symbol_body_fixture(&symbol_project);
    let mut symbol_state = state_for(
        symbol_project.clone(),
        Some(symbol_project.join(".rmu/index.db")),
    );
    let symbol_result = handle_tool_call(
        Some(json!({
            "name": "symbol_body",
            "arguments": {
                "seed": "inspect_body",
                "seed_kind": "symbol",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut symbol_state,
    )
    .expect("symbol_body should succeed");
    assert_required_structure(
        &symbol_result,
        &symbol_envelope,
        "investigation.symbol_body.envelope",
    );
    assert_required_structure(
        &symbol_result["structuredContent"],
        &symbol_schema,
        "investigation.symbol_body.structured",
    );

    let route_project = temp_dir("rmu-mcp-tests-schema-route-constraint");
    write_route_and_constraint_fixture(&route_project);
    let mut route_state = state_for(
        route_project.clone(),
        Some(route_project.join(".rmu/index.db")),
    );
    let route_result = handle_tool_call(
        Some(json!({
            "name": "route_trace",
            "arguments": {
                "seed": "resolve_lab",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut route_state,
    )
    .expect("route_trace should succeed");
    assert_required_structure(
        &route_result,
        &route_envelope,
        "investigation.route_trace.envelope",
    );
    assert_required_structure(
        &route_result["structuredContent"],
        &route_schema,
        "investigation.route_trace.structured",
    );

    let constraint_result = handle_tool_call(
        Some(json!({
            "name": "constraint_evidence",
            "arguments": {
                "seed": "resolve_lab",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut route_state,
    )
    .expect("constraint_evidence should succeed");
    assert_required_structure(
        &constraint_result,
        &constraint_envelope,
        "investigation.constraint.envelope",
    );
    assert_required_structure(
        &constraint_result["structuredContent"],
        &constraint_schema,
        "investigation.constraint.structured",
    );

    let cluster_project = temp_dir("rmu-mcp-tests-schema-cluster-divergence");
    write_cluster_and_divergence_fixture(&cluster_project);
    let mut cluster_state = state_for(
        cluster_project.clone(),
        Some(cluster_project.join(".rmu/index.db")),
    );
    let cluster_result = handle_tool_call(
        Some(json!({
            "name": "concept_cluster",
            "arguments": {
                "seed": "origin_resolution",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut cluster_state,
    )
    .expect("concept_cluster should succeed");
    assert_required_structure(
        &cluster_result,
        &cluster_envelope,
        "investigation.cluster.envelope",
    );
    assert_required_structure(
        &cluster_result["structuredContent"],
        &cluster_schema,
        "investigation.cluster.structured",
    );

    let divergence_result = handle_tool_call(
        Some(json!({
            "name": "divergence_report",
            "arguments": {
                "seed": "origin_resolution",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut cluster_state,
    )
    .expect("divergence_report should succeed");
    assert_required_structure(
        &divergence_result,
        &divergence_envelope,
        "investigation.divergence.envelope",
    );
    assert_required_structure(
        &divergence_result["structuredContent"],
        &divergence_schema,
        "investigation.divergence.structured",
    );

    let _ = fs::remove_dir_all(symbol_project);
    let _ = fs::remove_dir_all(route_project);
    let _ = fs::remove_dir_all(cluster_project);
}

#[test]
fn investigation_tools_mask_or_hash_textual_content_under_privacy_mode() {
    let symbol_project = temp_dir("rmu-mcp-tests-symbol-body-privacy");
    write_symbol_body_fixture(&symbol_project);
    let mut symbol_state = state_for(
        symbol_project.clone(),
        Some(symbol_project.join(".rmu/index.db")),
    );
    let symbol_result = handle_tool_call(
        Some(json!({
            "name": "symbol_body",
            "arguments": {
                "seed": "inspect_body",
                "seed_kind": "symbol",
                "limit": 5,
                "auto_index": true,
                "privacy_mode": "hash"
            }
        })),
        &mut symbol_state,
    )
    .expect("symbol_body should succeed");
    let symbol_content = &symbol_result["structuredContent"];
    assert!(
        symbol_content["seed"]["seed"]
            .as_str()
            .is_some_and(|value| value.starts_with("<query-hash:"))
    );
    assert!(
        symbol_content["items"][0]["anchor"]["path"]
            .as_str()
            .is_some_and(|value| value.starts_with("<hash:"))
    );
    assert!(
        symbol_content["items"][0]["signature"]
            .as_str()
            .is_some_and(|value| value.starts_with("<content-hash:"))
    );
    assert!(
        symbol_content["items"][0]["body"]
            .as_str()
            .is_some_and(|value| value.starts_with("<content-hash:"))
    );

    let route_project = temp_dir("rmu-mcp-tests-route-privacy");
    write_route_and_constraint_fixture(&route_project);
    let mut route_state = state_for(
        route_project.clone(),
        Some(route_project.join(".rmu/index.db")),
    );

    let route_result = handle_tool_call(
        Some(json!({
            "name": "route_trace",
            "arguments": {
                "seed": "resolve_lab",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true,
                "privacy_mode": "mask"
            }
        })),
        &mut route_state,
    )
    .expect("route_trace should succeed");
    let route_content = &route_result["structuredContent"];
    assert_eq!(route_content["seed"]["seed"], json!("<redacted-query>"));
    assert!(
        route_content["best_route"]["segments"][0]["path"]
            .as_str()
            .is_some_and(|value| value.starts_with("<masked:"))
    );
    assert_eq!(
        route_content["best_route"]["segments"][0]["evidence"],
        json!("<redacted-content>")
    );
    if let Some(gaps) = route_content["unresolved_gaps"].as_array() {
        for gap in gaps {
            if let Some(path) = gap["last_resolved_path"].as_str() {
                assert!(path.starts_with("<masked:"));
            }
        }
    }

    let constraint_result = handle_tool_call(
        Some(json!({
            "name": "constraint_evidence",
            "arguments": {
                "seed": "resolve_lab",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true,
                "privacy_mode": "mask"
            }
        })),
        &mut route_state,
    )
    .expect("constraint_evidence should succeed");
    let constraint_content = &constraint_result["structuredContent"];
    assert!(
        constraint_content["items"][0]["source_path"]
            .as_str()
            .is_some_and(|value| value.starts_with("<masked:"))
    );
    assert_eq!(
        constraint_content["items"][0]["normalized_text"],
        json!("<redacted-content>")
    );

    let cluster_project = temp_dir("rmu-mcp-tests-cluster-divergence-privacy");
    write_cluster_and_divergence_fixture(&cluster_project);
    let mut cluster_state = state_for(
        cluster_project.clone(),
        Some(cluster_project.join(".rmu/index.db")),
    );

    let cluster_result = handle_tool_call(
        Some(json!({
            "name": "concept_cluster",
            "arguments": {
                "seed": "origin_resolution",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true,
                "privacy_mode": "mask"
            }
        })),
        &mut cluster_state,
    )
    .expect("concept_cluster should succeed");
    let cluster_content = &cluster_result["structuredContent"];
    assert_eq!(cluster_content["seed"]["seed"], json!("<redacted-query>"));
    assert!(
        cluster_content["variants"][0]["entry_anchor"]["path"]
            .as_str()
            .is_some_and(|value| value.starts_with("<masked:"))
    );

    let divergence_result = handle_tool_call(
        Some(json!({
            "name": "divergence_report",
            "arguments": {
                "seed": "origin_resolution",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true,
                "privacy_mode": "mask"
            }
        })),
        &mut cluster_state,
    )
    .expect("divergence_report should succeed");
    let divergence_content = &divergence_result["structuredContent"];
    assert_eq!(divergence_content["summary"], json!("<redacted-content>"));
    if let Some(shared_evidence) = divergence_content["shared_evidence"].as_array() {
        for item in shared_evidence {
            assert_eq!(item, &json!("<redacted-content>"));
        }
    }
    if let Some(missing_evidence) = divergence_content["missing_evidence"].as_array() {
        for item in missing_evidence {
            assert_eq!(item, &json!("<redacted-content>"));
        }
    }
    if let Some(unknowns) = divergence_content["unknowns"].as_array() {
        for item in unknowns {
            assert_eq!(item, &json!("<redacted-content>"));
        }
    }
    let divergence_signals = divergence_content["divergence_signals"]
        .as_array()
        .expect("divergence_signals should be array");
    assert!(
        !divergence_signals.is_empty(),
        "fixture should produce at least one divergence signal"
    );
    for signal in divergence_signals {
        assert_eq!(signal["summary"], json!("<redacted-content>"));
    }
    if let Some(followups) = divergence_content["recommended_followups"].as_array() {
        for item in followups {
            assert_eq!(item, &json!("<redacted-content>"));
        }
    }

    let _ = fs::remove_dir_all(symbol_project);
    let _ = fs::remove_dir_all(route_project);
    let _ = fs::remove_dir_all(cluster_project);
}
