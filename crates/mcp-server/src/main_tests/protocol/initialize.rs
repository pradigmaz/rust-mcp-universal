use super::*;

#[test]
fn initialize_response_contains_protocol_version() {
    let mut state = default_state();
    let req = RpcRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(initialize_params()),
    };

    let response = handle_request(req, &mut state);
    let result = response.result.expect("initialize has result");
    assert_eq!(result["protocolVersion"], json!(PROTOCOL_VERSION));
    assert_eq!(
        result["capabilities"],
        json!({"tools": {"listChanged": false}})
    );
    assert!(result["capabilities"]["resources"].is_null());
    assert!(result["capabilities"]["prompts"].is_null());
    assert!(result["capabilities"]["logging"].is_null());
}

#[test]
fn initialize_accepts_supported_legacy_client_protocol_version() {
    let mut state = default_state();
    let req = RpcRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {"name": "probe", "version": "0.0.1"}
        })),
    };

    let response = handle_request(req, &mut state);
    let result = response.result.expect("initialize must succeed");
    assert_eq!(result["protocolVersion"], json!("2025-03-26"));
}

#[test]
fn initialize_accepts_unknown_but_non_empty_client_protocol_version() {
    let mut state = default_state();
    let req = RpcRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": "2099-01-01",
            "capabilities": {},
            "clientInfo": {"name": "probe", "version": "0.0.1"}
        })),
    };

    let response = handle_request(req, &mut state);
    let result = response.result.expect("initialize must succeed");
    assert_eq!(result["protocolVersion"], json!("2099-01-01"));
}

#[test]
fn initialize_rejects_empty_client_protocol_version() {
    let mut state = default_state();
    let req = RpcRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": "   ",
            "capabilities": {},
            "clientInfo": {"name": "probe", "version": "0.0.1"}
        })),
    };

    let response = handle_request(req, &mut state);
    assert_eq!(response.error.expect("error expected").code, -32602);
}

#[test]
fn initialize_rejects_missing_required_fields() {
    let mut state = default_state();
    let req = RpcRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {}
        })),
    };

    let response = handle_request(req, &mut state);
    assert_eq!(response.error.expect("error expected").code, -32602);
}

#[test]
fn initialize_binds_single_workspace_root() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    let project_dir = std::env::temp_dir().join(format!("rmu-mcp-init-root-{unique}"));
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");

    let mut state = default_state();
    let req = RpcRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {"name": "probe", "version": "0.0.1"},
            "workspaceFolders": [
                {"uri": format!("file:///{}", project_dir.display().to_string().replace('\\', "/"))}
            ]
        })),
    };

    let response = handle_request(req, &mut state);
    assert!(response.error.is_none());
    assert!(matches!(
        state.binding(),
        crate::state::ProjectBinding::Bound {
            source: crate::state::ProjectBindingSource::InitializeRoots,
            ..
        }
    ));

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn initialize_marks_multiple_workspace_roots_ambiguous() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    let project_a = std::env::temp_dir().join(format!("rmu-mcp-init-ambiguous-a-{unique}"));
    let project_b = std::env::temp_dir().join(format!("rmu-mcp-init-ambiguous-b-{unique}"));
    fs::create_dir_all(&project_a).expect("create project a");
    fs::create_dir_all(&project_b).expect("create project b");

    let mut state = default_state();
    let req = RpcRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {"name": "probe", "version": "0.0.1"},
            "roots": [
                {"path": project_a.display().to_string()},
                {"path": project_b.display().to_string()}
            ]
        })),
    };

    let response = handle_request(req, &mut state);
    assert!(response.error.is_none());
    assert!(matches!(
        state.binding(),
        crate::state::ProjectBinding::Ambiguous {
            source: crate::state::ProjectBindingSource::InitializeRoots,
            ..
        }
    ));

    let _ = fs::remove_dir_all(project_a);
    let _ = fs::remove_dir_all(project_b);
}

#[test]
fn duplicate_initialize_is_rejected() {
    let mut state = default_state();
    let first = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(initialize_params()),
        },
        &mut state,
    );
    assert!(first.error.is_none());

    let second = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!(2)),
            method: "initialize".to_string(),
            params: Some(initialize_params()),
        },
        &mut state,
    );
    assert_eq!(second.error.expect("error expected").code, -32600);
}

#[test]
fn tools_list_before_initialize_is_rejected() {
    let mut state = default_state();
    let response = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        },
        &mut state,
    );
    assert_eq!(response.error.expect("error expected").code, -32600);
}

#[test]
fn shutdown_and_exit_follow_lifecycle() {
    let mut state = running_state();
    let shutdown = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!(10)),
            method: "shutdown".to_string(),
            params: None,
        },
        &mut state,
    );
    assert!(shutdown.error.is_none());

    let exit = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: None,
            method: "exit".to_string(),
            params: None,
        },
        &mut state,
    );
    assert!(exit.error.is_none());
    assert!(state.should_exit());
}
