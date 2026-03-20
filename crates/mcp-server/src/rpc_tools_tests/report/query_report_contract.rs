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
    assert!(structured["retrieval_pipeline"].is_array());
    assert!(structured["selected_context"].is_array());
    assert!(structured["confidence"].is_object());
    assert!(structured["confidence"]["signals"].is_object());
    assert!(structured["confidence"]["signals"]["explain_coverage"].is_number());
    assert!(structured["gaps"].is_array());
    assert!(structured["index_telemetry"].is_object());
    assert!(structured["index_telemetry"]["last_index_lock_wait_ms"].is_number());
    assert!(structured["index_telemetry"]["last_embedding_cache_hits"].is_number());
    assert!(structured["index_telemetry"]["last_embedding_cache_misses"].is_number());
    assert!(structured["index_telemetry"]["chunk_coverage"].is_number());
    assert!(structured["index_telemetry"]["chunk_source"].is_string());
    if let Some(first_item) = structured["selected_context"]
        .as_array()
        .and_then(|items| items.first())
    {
        assert!(first_item["chunk_idx"].is_u64());
        assert!(first_item["start_line"].is_u64());
        assert!(first_item["end_line"].is_u64());
        assert!(first_item["chunk_source"].is_string());
        assert!(first_item["explain"].is_object());
        assert!(first_item["explain"]["lexical"].is_number());
        assert!(first_item["explain"]["graph"].is_number());
        assert!(first_item["explain"]["semantic"].is_number());
        assert!(first_item["explain"]["rrf"].is_number());
        assert!(first_item["explain"]["rank_before"].is_u64());
        assert!(first_item["explain"]["rank_after"].is_u64());
    }

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
