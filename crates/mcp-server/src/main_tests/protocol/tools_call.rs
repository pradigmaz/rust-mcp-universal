use super::*;

#[test]
fn tools_call_request_level_validation_uses_invalid_params_error() {
    let mut state = running_state();

    let missing_name =
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"arguments":{}}}"#;
    let response = expect_single_response(missing_name, &mut state);
    assert_eq!(response.error.expect("error expected").code, -32602);

    let bad_arguments_type = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_candidates","arguments":"bad"}}"#;
    let response = expect_single_response(bad_arguments_type, &mut state);
    assert_eq!(response.error.expect("error expected").code, -32602);
}

#[test]
fn unknown_tool_uses_invalid_params_error() {
    let mut state = running_state();

    let raw = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"definitely_missing_tool","arguments":{}}}"#;
    let response = expect_single_response(raw, &mut state);
    assert_eq!(response.error.expect("error expected").code, -32602);
}

#[test]
fn tools_call_argument_validation_errors_use_invalid_params_error() {
    let mut state = running_state();

    let empty_project_path = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"set_project_path","arguments":{"project_path":""}}}"#;
    let response = expect_single_response(empty_project_path, &mut state);
    assert_eq!(response.error.expect("error expected").code, -32602);

    let extra_field = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"search_candidates","arguments":{"query":"x","extra":1}}}"#;
    let response = expect_single_response(extra_field, &mut state);
    assert_eq!(response.error.expect("error expected").code, -32602);

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    let missing_path = std::env::temp_dir().join(format!("rmu-mcp-tests-missing-{unique}"));
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "set_project_path",
            "arguments": {
                "project_path": missing_path.display().to_string()
            }
        }
    })
    .to_string();
    let response = expect_single_response(&raw, &mut state);
    assert_eq!(response.error.expect("error expected").code, -32602);
}

#[test]
fn tools_call_oversized_limit_uses_invalid_params_error() {
    if usize::BITS < 64 {
        return;
    }

    let mut state = running_state();

    let raw = format!(
        r#"{{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{{"name":"search_candidates","arguments":{{"query":"x","limit":{}}}}}}}"#,
        i64::MAX as u128 + 1
    );
    let response = expect_single_response(&raw, &mut state);
    assert_eq!(response.error.expect("error expected").code, -32602);
}

#[test]
fn tools_call_runtime_errors_are_sanitized_with_hash_privacy_mode() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    let project_dir = std::env::temp_dir().join(format!("rmu-mcp-tests-protocol-privacy-{unique}"));
    fs::create_dir_all(&project_dir).expect("create temp dir");

    let baseline_path = project_dir.join("missing-baseline.json");
    let baseline_path_text = baseline_path.display().to_string();

    let mut state = ServerState::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let _ = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(initialize_params()),
        },
        &mut state,
    );
    let _ = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: None,
            method: "notifications/initialized".to_string(),
            params: None,
        },
        &mut state,
    );

    let req = RpcRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(json!(501)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "query_benchmark",
            "arguments": {
                "dataset_path": "dataset.json",
                "baseline": baseline_path_text,
                "privacy_mode": "hash"
            }
        })),
    };

    let response = handle_request(req, &mut state);
    assert!(response.error.is_none());
    let result = response
        .result
        .expect("tools/call should return result envelope");
    assert_eq!(result["isError"], json!(true));
    let message = result["structuredContent"]["error"]
        .as_str()
        .expect("error message should be string");
    assert!(message.contains("privacy_mode=hash"));
    assert!(message.contains("fingerprint="));
    assert!(!message.contains(&baseline_path.display().to_string()));

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn tools_call_set_project_path_accepts_unicode_path_values() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    let project_dir =
        std::env::temp_dir().join(format!("rmu-mcp-tests-protocol-unicode-путь-{unique}"));
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/main.rs"),
        "fn unicode_path_probe() {}\n",
    )
    .expect("write fixture");

    let mut state = ServerState::new(PathBuf::from("."), Some(project_dir.join(".rmu/index.db")));
    let _ = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(initialize_params()),
        },
        &mut state,
    );
    let _ = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: None,
            method: "notifications/initialized".to_string(),
            params: None,
        },
        &mut state,
    );

    let set_path = json!({
        "jsonrpc": "2.0",
        "id": 600,
        "method": "tools/call",
        "params": {
            "name": "set_project_path",
            "arguments": {
                "project_path": project_dir.display().to_string()
            }
        }
    })
    .to_string();
    let response = expect_single_response(&set_path, &mut state);
    assert!(response.error.is_none());
    assert_eq!(
        response.result.expect("result expected")["structuredContent"]["project_path"],
        json!(project_dir.display().to_string())
    );

    let search = json!({
        "jsonrpc": "2.0",
        "id": 601,
        "method": "tools/call",
        "params": {
            "name": "search_candidates",
            "arguments": {
                "query": "unicode_path_probe",
                "limit": 5,
                "auto_index": true
            }
        }
    })
    .to_string();
    let response = expect_single_response(&search, &mut state);
    assert!(response.error.is_none());
    let result = response.result.expect("search result expected");
    assert_eq!(result["isError"], json!(false));
    let hits = result["structuredContent"]["hits"]
        .as_array()
        .expect("hits should be array");
    assert!(
        hits.iter()
            .filter_map(|hit| hit["path"].as_str())
            .any(|path| path == "src/main.rs" || path.ends_with("src/main.rs"))
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn tools_call_set_project_path_rejects_file_paths() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    let project_dir = std::env::temp_dir().join(format!("rmu-mcp-tests-protocol-file-{unique}"));
    fs::create_dir_all(&project_dir).expect("create temp dir");
    let file_path = project_dir.join("single.rs");
    fs::write(&file_path, "fn not_a_directory() {}\n").expect("write fixture");

    let mut state = ServerState::new(PathBuf::from("."), Some(project_dir.join(".rmu/index.db")));
    let _ = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(initialize_params()),
        },
        &mut state,
    );
    let _ = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: None,
            method: "notifications/initialized".to_string(),
            params: None,
        },
        &mut state,
    );

    let set_path = json!({
        "jsonrpc": "2.0",
        "id": 602,
        "method": "tools/call",
        "params": {
            "name": "set_project_path",
            "arguments": {
                "project_path": file_path.display().to_string()
            }
        }
    })
    .to_string();
    let response = expect_single_response(&set_path, &mut state);
    assert_eq!(response.error.expect("error expected").code, -32602);

    let _ = fs::remove_dir_all(project_dir);
}
