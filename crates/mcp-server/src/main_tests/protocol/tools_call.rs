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
fn tools_call_invalid_privacy_mode_suggests_off() {
    let mut state = running_state();

    let raw = r#"{"jsonrpc":"2.0","id":41,"method":"tools/call","params":{"name":"search_candidates","arguments":{"query":"x","privacy_mode":"repo-only"}}}"#;
    let response = expect_single_response(raw, &mut state);
    let error = response.error.expect("error expected");
    let error_json = serde_json::to_value(&error).expect("serialize error");
    assert_eq!(error.code, -32602);
    assert!(
        error_json["message"]
            .as_str()
            .is_some_and(|message| message.contains("use `off` for unsanitized output"))
    );
    assert!(
        error_json["message"]
            .as_str()
            .is_some_and(|message| message.contains("repo-only"))
    );
}

#[test]
fn tools_call_rule_violations_invalid_sort_by_points_to_path_prefix() {
    let mut state = running_state();

    let raw = r#"{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{"name":"rule_violations","arguments":{"sort_by":"path"}}}"#;
    let response = expect_single_response(raw, &mut state);
    let error = response.error.expect("error expected");
    let error_json = serde_json::to_value(&error).expect("serialize error");
    assert_eq!(error.code, -32602);
    assert!(
        error_json["message"]
            .as_str()
            .is_some_and(|message| message.contains("metric_value"))
    );
    assert!(
        error_json["message"]
            .as_str()
            .is_some_and(|message| message.contains("path_prefix"))
    );
}

#[test]
fn project_scoped_tools_fail_when_project_is_not_bound() {
    let mut state = default_state();
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

    let response = expect_single_response(
        r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"search_candidates","arguments":{"query":"x"}}}"#,
        &mut state,
    );
    assert!(response.error.is_none());
    let result = response.result.expect("result expected");
    assert_eq!(result["isError"], json!(true));
    assert_eq!(
        result["structuredContent"]["code"],
        json!("E_PROJECT_NOT_BOUND")
    );
}

#[test]
fn preflight_reports_unbound_project_without_using_fallback_project_path() {
    let mut state = default_state();
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

    let response = expect_single_response(
        r#"{"jsonrpc":"2.0","id":81,"method":"tools/call","params":{"name":"preflight","arguments":{}}}"#,
        &mut state,
    );
    assert!(response.error.is_none());
    let result = response.result.expect("result expected");
    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["binding_status"],
        json!("unbound")
    );
    assert!(result["structuredContent"]["resolved_project_path"].is_null());
    assert!(result["structuredContent"]["resolved_db_path"].is_null());
    assert!(
        result["structuredContent"]["binding_errors"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
}

#[test]
fn project_scoped_tools_fail_when_initialize_roots_are_ambiguous() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    let project_a = std::env::temp_dir().join(format!("rmu-mcp-ambiguous-tool-a-{unique}"));
    let project_b = std::env::temp_dir().join(format!("rmu-mcp-ambiguous-tool-b-{unique}"));
    fs::create_dir_all(&project_a).expect("create project a");
    fs::create_dir_all(&project_b).expect("create project b");

    let mut state = default_state();
    let _ = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "0.0.1"},
                "roots": [
                    {"path": project_a.display().to_string()},
                    {"path": project_b.display().to_string()}
                ]
            })),
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

    let response = expect_single_response(
        r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"workspace_brief","arguments":{}}}"#,
        &mut state,
    );
    assert!(response.error.is_none());
    let result = response.result.expect("result expected");
    assert_eq!(result["isError"], json!(true));
    assert_eq!(
        result["structuredContent"]["code"],
        json!("E_PROJECT_AMBIGUOUS")
    );

    let _ = fs::remove_dir_all(project_a);
    let _ = fs::remove_dir_all(project_b);
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

    let mut state = ServerState::new(
        Some(project_dir.clone()),
        Some(project_dir.join(".rmu/index.db")),
    );
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

    let mut state = ServerState::new(
        Some(project_dir.clone()),
        Some(project_dir.join(".rmu/index.db")),
    );
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
    let result = response.result.expect("result expected");
    assert_eq!(
        result["structuredContent"]["project_path"],
        json!(project_dir.display().to_string())
    );
    assert_eq!(
        result["structuredContent"]["gitignore_created"],
        json!(false)
    );
    assert_eq!(
        result["structuredContent"]["gitignore_updated"],
        json!(false)
    );
    assert!(
        !project_dir.join(".gitignore").exists(),
        "set_project_path should not create .gitignore"
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
fn tools_call_set_project_path_rejects_pinned_db_sessions() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    let project_dir = std::env::temp_dir().join(format!("rmu-mcp-pinned-db-{unique}"));
    fs::create_dir_all(&project_dir).expect("create temp dir");

    let mut state = ServerState::new(None, Some(project_dir.join(".rmu/index.db")));
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

    let response = expect_single_response(
        &json!({
            "jsonrpc": "2.0",
            "id": 602,
            "method": "tools/call",
            "params": {
                "name": "set_project_path",
                "arguments": {
                    "project_path": project_dir.display().to_string()
                }
            }
        })
        .to_string(),
        &mut state,
    );
    assert!(response.error.is_none());
    let result = response.result.expect("result expected");
    assert_eq!(result["isError"], json!(true));
    assert_eq!(
        result["structuredContent"]["code"],
        json!("E_DB_PATH_PINNED")
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn tools_call_install_ignore_rules_defaults_to_git_info_exclude_and_is_idempotent() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    let project_dir =
        std::env::temp_dir().join(format!("rmu-mcp-tests-install-ignore-default-{unique}"));
    fs::create_dir_all(project_dir.join(".git/info")).expect("create git info dir");

    let mut state = ServerState::new(
        Some(project_dir.clone()),
        Some(project_dir.join(".rmu/index.db")),
    );
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
    let _ = expect_single_response(
        &json!({
            "jsonrpc": "2.0",
            "id": 700,
            "method": "tools/call",
            "params": {
                "name": "set_project_path",
                "arguments": {
                    "project_path": project_dir.display().to_string()
                }
            }
        })
        .to_string(),
        &mut state,
    );

    let install = json!({
        "jsonrpc": "2.0",
        "id": 701,
        "method": "tools/call",
        "params": {
            "name": "install_ignore_rules",
            "arguments": {}
        }
    })
    .to_string();
    let response = expect_single_response(&install, &mut state);
    assert!(response.error.is_none());
    let result = response.result.expect("result expected");
    assert_eq!(
        result["structuredContent"]["target"],
        json!("git-info-exclude")
    );
    assert_eq!(result["structuredContent"]["created"], json!(true));
    assert_eq!(result["structuredContent"]["updated"], json!(true));
    assert!(
        result["structuredContent"]["path"]
            .as_str()
            .unwrap_or_default()
            .ends_with(".git/info/exclude")
    );
    assert!(
        !project_dir.join(".gitignore").exists(),
        "default install should not create root .gitignore"
    );

    let exclude = fs::read_to_string(project_dir.join(".git/info/exclude")).expect("read exclude");
    assert!(exclude.contains(".rmu/"));
    assert!(exclude.contains(".codex/"));

    let second = expect_single_response(&install, &mut state);
    assert!(second.error.is_none());
    let second_result = second.result.expect("second result expected");
    assert_eq!(second_result["structuredContent"]["created"], json!(false));
    assert_eq!(second_result["structuredContent"]["updated"], json!(false));

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn tools_call_install_ignore_rules_supports_root_gitignore_target() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    let project_dir =
        std::env::temp_dir().join(format!("rmu-mcp-tests-install-ignore-root-{unique}"));
    fs::create_dir_all(&project_dir).expect("create temp dir");

    let mut state = ServerState::new(
        Some(project_dir.clone()),
        Some(project_dir.join(".rmu/index.db")),
    );
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
    let _ = expect_single_response(
        &json!({
            "jsonrpc": "2.0",
            "id": 710,
            "method": "tools/call",
            "params": {
                "name": "set_project_path",
                "arguments": {
                    "project_path": project_dir.display().to_string()
                }
            }
        })
        .to_string(),
        &mut state,
    );

    let install = json!({
        "jsonrpc": "2.0",
        "id": 711,
        "method": "tools/call",
        "params": {
            "name": "install_ignore_rules",
            "arguments": {
                "target": "root-gitignore"
            }
        }
    })
    .to_string();
    let response = expect_single_response(&install, &mut state);
    assert!(response.error.is_none());
    let result = response.result.expect("result expected");
    assert_eq!(
        result["structuredContent"]["target"],
        json!("root-gitignore")
    );
    assert_eq!(result["structuredContent"]["created"], json!(true));
    assert_eq!(result["structuredContent"]["updated"], json!(true));

    let gitignore =
        fs::read_to_string(project_dir.join(".gitignore")).expect("read root gitignore");
    assert!(gitignore.contains(".rmu/"));
    assert!(gitignore.contains(".vscode/"));

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

    let mut state = ServerState::new(
        Some(project_dir.clone()),
        Some(project_dir.join(".rmu/index.db")),
    );
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
