use std::fs;

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

    let mut state = ServerState::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
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
