use std::fs;

use rmu_core::set_thread_running_binary_timestamps_override_for_tests;
use rusqlite::Connection;
use serde_json::json;

use crate::{ServerState, process_raw_message};

use super::temp_dir;

#[test]
fn tools_call_search_candidates_uses_fallback_path_through_rpc_stack() {
    let project_dir = temp_dir("rmu-mcp-tests-rpc-fallback");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/uni.rs"),
        "pub fn unicode_probe() { let s = \"Cafe\\u{301}\"; println!(\"{s}\"); }\n",
    )
    .expect("write file");

    let mut state = ServerState::new(
        Some(project_dir.clone()),
        Some(project_dir.join(".rmu/index.db")),
    );
    let _ = process_raw_message(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": crate::PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "0.0.1"}
            }
        })
        .to_string(),
        &mut state,
    );
    let _ = process_raw_message(
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        &mut state,
    );

    // Single-character query produces an empty FTS token set, so search must rely on LIKE fallback.
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "tools/call",
        "params": {
            "name": "search_candidates",
            "arguments": {
                "query": "É",
                "limit": 5,
                "auto_index": true
            }
        }
    })
    .to_string();

    let response = process_raw_message(&raw, &mut state).expect("response expected");
    assert_eq!(response.id, Some(json!(99)));
    assert!(response.error.is_none());

    let result = response.result.expect("result expected");
    assert_eq!(result["isError"], json!(false));
    let hits = result["structuredContent"]["hits"]
        .as_array()
        .expect("hits should be array");
    assert!(
        hits.iter()
            .filter_map(|hit| hit["path"].as_str())
            .any(|path: &str| path.ends_with("src/uni.rs") || path == "src/uni.rs")
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn tools_call_compatibility_errors_return_structured_details() {
    let project_dir = temp_dir("rmu-mcp-tests-compatibility");
    fs::create_dir_all(project_dir.join(".rmu")).expect("create rmu dir");
    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path).expect("open db");
    conn.execute_batch("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);")
        .expect("create meta");
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)",
        rusqlite::params!["schema_version", "999"],
    )
    .expect("insert schema version");
    drop(conn);

    let mut state = ServerState::new(Some(project_dir.clone()), Some(db_path));
    let _ = process_raw_message(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": crate::PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "0.0.1"}
            }
        })
        .to_string(),
        &mut state,
    );
    let _ = process_raw_message(
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        &mut state,
    );

    let response = process_raw_message(
        &json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "symbol_body",
                "arguments": {
                    "seed": "anything",
                    "seed_kind": "query"
                }
            }
        })
        .to_string(),
        &mut state,
    )
    .expect("response expected");

    let result = response.result.expect("result expected");
    assert_eq!(result["isError"], json!(true));
    assert_eq!(
        result["structuredContent"]["code"],
        json!("E_COMPATIBILITY")
    );
    assert_eq!(
        result["structuredContent"]["details"]["kind"],
        json!("compatibility")
    );
    assert!(result["structuredContent"]["details"]["running_binary_version"].is_string());
    assert!(result["structuredContent"]["details"]["running_binary_stale"].is_boolean());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn tools_call_preflight_returns_structured_incompatible_status() {
    let project_dir = temp_dir("rmu-mcp-tests-preflight-rpc");
    fs::create_dir_all(project_dir.join(".rmu")).expect("create rmu dir");
    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path).expect("open db");
    conn.execute_batch("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);")
        .expect("create meta");
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)",
        rusqlite::params!["schema_version", "999"],
    )
    .expect("insert schema version");
    drop(conn);

    let mut state = ServerState::new(Some(project_dir.clone()), Some(db_path));
    let _ = process_raw_message(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": crate::PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "0.0.1"}
            }
        })
        .to_string(),
        &mut state,
    );
    let _ = process_raw_message(
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        &mut state,
    );

    let response = process_raw_message(
        &json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "preflight",
                "arguments": {}
            }
        })
        .to_string(),
        &mut state,
    )
    .expect("response expected");

    assert_eq!(response.id, Some(json!(8)));
    assert!(response.error.is_none());

    let result = response.result.expect("result expected");
    assert_eq!(result["isError"], json!(false));
    assert_eq!(result["structuredContent"]["status"], json!("incompatible"));
    assert!(result["structuredContent"]["running_binary_version"].is_string());
    assert!(result["structuredContent"]["running_binary_stale"].is_boolean());
    assert!(
        result["structuredContent"]["errors"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn tools_call_preflight_and_runtime_guard_report_stale_running_binary() {
    let _override = set_thread_running_binary_timestamps_override_for_tests(1000, 4001);

    let project_dir = temp_dir("rmu-mcp-tests-stale-running-binary");
    let mut state = ServerState::new(
        Some(project_dir.clone()),
        Some(project_dir.join(".rmu/index.db")),
    );
    let _ = process_raw_message(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": crate::PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "0.0.1"}
            }
        })
        .to_string(),
        &mut state,
    );
    let _ = process_raw_message(
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        &mut state,
    );

    let preflight_response = process_raw_message(
        &json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "name": "preflight",
                "arguments": {}
            }
        })
        .to_string(),
        &mut state,
    )
    .expect("response expected");
    let preflight_result = preflight_response.result.expect("preflight result");
    assert_eq!(preflight_result["isError"], json!(false));
    assert_eq!(
        preflight_result["structuredContent"]["status"],
        json!("incompatible")
    );
    assert_eq!(
        preflight_result["structuredContent"]["running_binary_stale"],
        json!(true)
    );

    let tool_response = process_raw_message(
        &json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "search_candidates",
                "arguments": {
                    "query": "anything",
                    "limit": 5
                }
            }
        })
        .to_string(),
        &mut state,
    )
    .expect("response expected");
    let tool_result = tool_response.result.expect("tool result");
    assert_eq!(tool_result["isError"], json!(true));
    assert_eq!(
        tool_result["structuredContent"]["code"],
        json!("E_COMPATIBILITY")
    );
    assert_eq!(
        tool_result["structuredContent"]["details"]["running_binary_stale"],
        json!(true)
    );
    assert!(tool_result["structuredContent"]["details"]["running_binary_version"].is_string());

    let _ = fs::remove_dir_all(project_dir);
}
