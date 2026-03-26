use std::fs;

use serde_json::json;

use super::*;

#[test]
fn db_maintenance_runs_and_reports_selected_operations() {
    let project_dir = temp_dir("rmu-mcp-tests-db-maintenance");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn db_maintenance_symbol() {}\n",
    )
    .expect("write file");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let _indexed = handle_tool_call(
        Some(json!({
            "name": "semantic_index",
            "arguments": {"reindex": true}
        })),
        &mut state,
    )
    .expect("semantic_index should succeed");

    let result = handle_tool_call(
        Some(json!({
            "name": "db_maintenance",
            "arguments": {
                "integrity_check": true,
                "checkpoint": true,
                "stats": true,
                "prune": true
            }
        })),
        &mut state,
    )
    .expect("db_maintenance should succeed");

    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert!(structured["db_path"].is_string());
    assert_eq!(structured["options"]["integrity_check"], json!(true));
    assert_eq!(structured["options"]["checkpoint"], json!(true));
    assert_eq!(structured["options"]["stats"], json!(true));
    assert_eq!(structured["options"]["prune"], json!(true));
    assert!(structured["integrity_ok"].is_boolean());
    assert!(structured["checkpoint"].is_object());
    assert!(structured["stats"].is_object());
    assert!(structured["prune"].is_object());
    assert!(structured["stats"]["total_size_bytes"].is_number());
    assert!(structured["stats"]["approx_free_bytes"].is_number());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn db_maintenance_hash_privacy_mode_sanitizes_db_path() {
    let project_dir = temp_dir("rmu-mcp-tests-db-maintenance-privacy");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn db_maintenance_privacy_symbol() {}\n",
    )
    .expect("write file");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let _indexed = handle_tool_call(
        Some(json!({
            "name": "semantic_index",
            "arguments": {"reindex": true}
        })),
        &mut state,
    )
    .expect("semantic_index should succeed");

    let result = handle_tool_call(
        Some(json!({
            "name": "db_maintenance",
            "arguments": {
                "stats": true,
                "privacy_mode": "hash"
            }
        })),
        &mut state,
    )
    .expect("db_maintenance should succeed");

    assert_eq!(result["isError"], json!(false));
    let db_path = result["structuredContent"]["db_path"]
        .as_str()
        .expect("db_path should be string");
    assert!(db_path.starts_with("<hash:"));
    assert!(db_path.ends_with('>'));
    assert!(!db_path.contains(&project_dir.display().to_string()));

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn agent_bootstrap_without_query_returns_workspace_only() {
    let project_dir = temp_dir("rmu-mcp-tests-bootstrap-empty");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn bootstrap_symbol() -> i32 { 1 }\n",
    )
    .expect("write file");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "agent_bootstrap",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("agent_bootstrap should succeed");

    assert_eq!(result["isError"], json!(false));
    assert!(result["structuredContent"]["brief"]["index_status"]["files"].is_number());
    assert!(result["structuredContent"]["query_bundle"].is_null());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn agent_bootstrap_with_query_returns_context_bundle() {
    let project_dir = temp_dir("rmu-mcp-tests-bootstrap-query");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn bootstrap_query_symbol() -> i32 { 77 }\n",
    )
    .expect("write file");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "agent_bootstrap",
            "arguments": {
                "query": "bootstrap_query_symbol",
                "limit": 5,
                "semantic": true,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("agent_bootstrap should succeed");

    assert_eq!(result["isError"], json!(false));
    let bundle = &result["structuredContent"]["query_bundle"];
    assert!(bundle.is_object());
    assert!(bundle["hits"].is_array());
    assert!(bundle["context"].is_object());
    assert!(bundle["report"].is_null());
    assert!(bundle["investigation_summary"].is_null());
    assert!(result["structuredContent"]["timings"]["total_ms"].is_u64());
    assert!(result["structuredContent"]["timings"]["search_ms"].is_u64());

    let detailed = handle_tool_call(
        Some(json!({
            "name": "agent_bootstrap",
            "arguments": {
                "query": "bootstrap_query_symbol",
                "limit": 5,
                "semantic": true,
                "auto_index": true,
                "include_report": true,
                "include_investigation_summary": true
            }
        })),
        &mut state,
    )
    .expect("agent_bootstrap with opt-in surfaces should succeed");

    assert_eq!(detailed["isError"], json!(false));
    let detailed_bundle = &detailed["structuredContent"]["query_bundle"];
    assert!(detailed_bundle["report"].is_object());
    assert!(detailed_bundle["investigation_summary"].is_object());
    assert!(
        detailed_bundle["report"]["investigation_summary"].is_object()
            || detailed_bundle["report"]["investigation_summary"].is_null()
    );
    assert!(detailed_bundle["report"]["timings"].is_object());
    assert!(detailed_bundle["report"]["timings"]["search_ms"].is_number());
    assert!(detailed_bundle["report"]["timings"]["investigation"]["route_ms"].is_number());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn agent_bootstrap_auto_index_uses_mixed_profile_and_skips_docs() {
    let project_dir = temp_dir("rmu-mcp-tests-bootstrap-auto-index-mixed");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::create_dir_all(project_dir.join("docs")).expect("create docs");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn bootstrap_code_only_symbol() -> i32 { 11 }\n",
    )
    .expect("write src");
    fs::write(
        project_dir.join("docs/design.md"),
        "bootstrap_docs_only_marker\n",
    )
    .expect("write docs");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "agent_bootstrap",
            "arguments": {
                "query": "bootstrap_docs_only_marker",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("agent_bootstrap should succeed");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["query_bundle"]["hits"],
        json!([])
    );
    assert_eq!(
        result["structuredContent"]["brief"]["index_status"]["files"],
        json!(1)
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn query_tools_require_non_empty_index_by_default() {
    let project_dir = temp_dir("rmu-mcp-tests-no-auto-index-default");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn default_policy_symbol() -> i32 { 5 }\n",
    )
    .expect("write file");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let err = handle_tool_call(
        Some(json!({
            "name": "search_candidates",
            "arguments": {
                "query": "default_policy_symbol"
            }
        })),
        &mut state,
    )
    .expect_err("default policy should reject empty index");
    assert!(err.to_string().contains("index is empty"));

    let _ = fs::remove_dir_all(project_dir);
}
