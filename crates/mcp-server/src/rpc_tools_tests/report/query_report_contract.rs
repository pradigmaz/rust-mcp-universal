use std::fs;

use serde_json::json;

use super::*;

#[test]
fn query_report_returns_mcp_envelope_with_required_fields() {
    let project_dir = temp_dir("rmu-mcp-tests-report");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn query_report_symbol() -> i32 { 42 }\n",
    )
    .expect("write file");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "query_report",
            "arguments": {
                "query": "query_report_symbol",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("query_report should succeed");

    assert_eq!(result["isError"], json!(false));
    assert!(result["content"].is_array());
    assert_eq!(result["content"][0]["type"], json!("text"));
    assert!(result["content"][0]["text"].is_string());

    let structured = &result["structuredContent"];
    assert!(structured["query_id"].is_string());
    assert!(structured["timestamp_utc"].is_string());
    assert!(structured["project_root"].is_string());
    assert!(structured["budget"].is_object());
    assert!(structured["resolved_mode"].is_string());
    assert!(structured["mode_source"].is_string());
    assert!(structured["retrieval_pipeline"].is_array());
    assert!(structured["selected_context"].is_array());
    assert!(structured["provenance"].is_object());
    assert!(structured["confidence"].is_object());
    assert!(structured["confidence"]["signals"].is_object());
    assert!(structured["confidence"]["signals"]["explain_coverage"].is_number());
    assert!(structured["gaps"].is_array());
    assert!(structured["index_telemetry"].is_object());
    assert!(structured["degradation_reasons"].is_array());
    assert!(structured["deepen_available"].is_boolean());
    assert!(structured["deepen_hint"].is_null() || structured["deepen_hint"].is_string());
    assert!(structured["investigation_summary"].is_object());
    assert!(structured["timings"].is_object());
    assert_eq!(
        structured["investigation_summary"]["surface_kind"],
        json!("embedded_investigation_hints")
    );
    assert!(structured["index_telemetry"]["last_index_lock_wait_ms"].is_number());
    assert!(structured["index_telemetry"]["last_embedding_cache_hits"].is_number());
    assert!(structured["index_telemetry"]["last_embedding_cache_misses"].is_number());
    assert!(structured["index_telemetry"]["chunk_coverage"].is_number());
    assert!(structured["index_telemetry"]["chunk_source"].is_string());
    assert!(structured["timings"]["search_ms"].is_number());
    assert!(structured["timings"]["context_ms"].is_number());
    assert!(structured["timings"]["investigation_ms"].is_number());
    assert!(structured["timings"]["format_ms"].is_number());
    assert!(structured["timings"]["total_ms"].is_number());
    assert!(structured["timings"]["investigation"]["route_ms"].is_number());
    assert!(structured["timings"]["investigation"]["cluster_ms"].is_number());
    if let Some(first_item) = structured["selected_context"]
        .as_array()
        .and_then(|items| items.first())
    {
        assert!(first_item["chunk_idx"].is_u64());
        assert!(first_item["start_line"].is_u64());
        assert!(first_item["end_line"].is_u64());
        assert!(first_item["chunk_source"].is_string());
        assert!(first_item["explain"].is_object());
        assert!(first_item["provenance"].is_object());
        assert!(first_item["explain"]["lexical"].is_number());
        assert!(first_item["explain"]["graph"].is_number());
        assert!(first_item["explain"]["semantic"].is_number());
        assert!(first_item["explain"]["rrf"].is_number());
        assert!(first_item["explain"]["rank_before"].is_u64());
        assert!(first_item["explain"]["rank_after"].is_u64());
        assert!(first_item["provenance"]["basis"].is_string());
        assert!(first_item["provenance"]["derivation"].is_string());
        assert!(first_item["provenance"]["freshness"].is_string());
        assert!(first_item["provenance"]["strength"].is_string());
        assert!(first_item["provenance"]["reasons"].is_array());
    }
    assert!(structured["investigation_summary"]["concept_cluster"]["variant_count"].is_u64());
    assert!(structured["investigation_summary"]["concept_cluster"]["top_variants"].is_array());
    assert!(structured["investigation_summary"]["route_trace"]["segment_kinds"].is_array());
    assert!(
        structured["investigation_summary"]["constraint_evidence"]["normalized_keys"].is_array()
    );
    if structured["investigation_summary"]["divergence"].is_object() {
        assert_eq!(
            structured["investigation_summary"]["divergence"]["surface_kind"],
            json!("divergence_preview")
        );
        assert_eq!(
            structured["investigation_summary"]["divergence"]["authoritative_tool"],
            json!("divergence_report")
        );
        assert_eq!(
            structured["investigation_summary"]["divergence"]["preview_only"],
            json!(true)
        );
        assert!(structured["investigation_summary"]["divergence"]["variants"].is_null());
        assert!(structured["investigation_summary"]["divergence"]["divergence_signals"].is_null());
    }

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn query_report_mode_source_and_provenance_contracts_are_first_class() {
    let project_dir = temp_dir("rmu-mcp-tests-report-mode-contract");
    fs::create_dir_all(project_dir.join("backend")).expect("create backend dir");
    fs::create_dir_all(project_dir.join("tests")).expect("create tests dir");
    fs::write(
        project_dir.join("backend/auth.py"),
        "def auth_boundary():\n    return 'auth boundary service'\n",
    )
    .expect("write backend");
    fs::write(
        project_dir.join("tests/test_auth.py"),
        "def test_auth_boundary():\n    return 'auth boundary tests'\n",
    )
    .expect("write tests");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let explicit = handle_tool_call(
        Some(json!({
            "name": "query_report",
            "arguments": {
                "query": "auth boundary tests",
                "mode": "test_map",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("explicit mode query_report should succeed");
    let explicit_structured = &explicit["structuredContent"];
    assert_eq!(explicit_structured["resolved_mode"], json!("test_map"));
    assert_eq!(explicit_structured["mode_source"], json!("explicit"));
    assert!(explicit_structured["provenance"]["basis"].is_string());
    assert!(explicit_structured["provenance"]["freshness"].is_string());
    assert!(explicit_structured["provenance"]["strength"].is_string());
    assert!(explicit_structured["provenance"]["reasons"].is_array());

    let inferred = handle_tool_call(
        Some(json!({
            "name": "query_report",
            "arguments": {
                "query": "auth boundary tests",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("inferred mode query_report should succeed");
    assert_eq!(
        inferred["structuredContent"]["resolved_mode"],
        json!("test_map")
    );
    assert_eq!(
        inferred["structuredContent"]["mode_source"],
        json!("inferred")
    );

    let defaulted = handle_tool_call(
        Some(json!({
            "name": "query_report",
            "arguments": {
                "query": "mystery",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("default mode query_report should succeed");
    assert_eq!(
        defaulted["structuredContent"]["resolved_mode"],
        json!("entrypoint_map")
    );
    assert_eq!(
        defaulted["structuredContent"]["mode_source"],
        json!("default")
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn query_report_payload_matches_local_json_schemas() {
    let project_dir = temp_dir("rmu-mcp-tests-schema");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn schema_contract_symbol() -> i32 { 7 }\n",
    )
    .expect("write file");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "query_report",
            "arguments": {
                "query": "schema_contract_symbol",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("query_report should succeed");

    let envelope_schema = load_schema("mcp_query_report_tool_result.schema.json");
    let report_schema = load_schema("query_report.schema.json");
    assert_required_structure(&result, &envelope_schema, "mcp_result");
    assert_required_structure(
        &result["structuredContent"],
        &report_schema,
        "mcp_result.structuredContent",
    );

    let mut invalid_minimum = result["structuredContent"].clone();
    invalid_minimum["budget"]["max_tokens"] = json!(-1);
    assert_schema_rejects(
        &invalid_minimum,
        &report_schema,
        "mcp_result.structuredContent.invalid_minimum",
    );

    let mut invalid_maximum = result["structuredContent"].clone();
    invalid_maximum["confidence"]["overall"] = json!(1.5);
    assert_schema_rejects(
        &invalid_maximum,
        &report_schema,
        "mcp_result.structuredContent.invalid_maximum",
    );

    let mut invalid_rank = result["structuredContent"].clone();
    if let Some(first_item) = invalid_rank["selected_context"]
        .as_array_mut()
        .and_then(|items| items.first_mut())
    {
        first_item["explain"]["rank_before"] = json!(0);
    }
    assert_schema_rejects(
        &invalid_rank,
        &report_schema,
        "mcp_result.structuredContent.invalid_rank_before",
    );

    let _ = fs::remove_dir_all(project_dir);
}
