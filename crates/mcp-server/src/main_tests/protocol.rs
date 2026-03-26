use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use crate::{
    PROTOCOL_VERSION, RpcRequest, RpcResponse, ServerState, handle_request, parse_error_response,
    process_raw_message,
};

fn expect_single_response(raw: &str, state: &mut ServerState) -> RpcResponse {
    process_raw_message(raw, state).expect("response expected")
}

fn default_state() -> ServerState {
    ServerState::new(None, None)
}

fn initialize_params() -> serde_json::Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {},
        "clientInfo": {
            "name": "test-client",
            "version": "0.0.1"
        }
    })
}

fn initialize_params_with_project(project_path: &str) -> serde_json::Value {
    let mut params = initialize_params();
    params["projectPath"] = json!(project_path);
    params
}

fn running_state() -> ServerState {
    let mut state = default_state();
    let project_path = std::env::current_dir()
        .expect("current dir should exist")
        .display()
        .to_string();
    let response = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(initialize_params_with_project(&project_path)),
        },
        &mut state,
    );
    assert!(
        response.error.is_none(),
        "initialize must succeed in test setup"
    );
    let response = handle_request(
        RpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: None,
            method: "notifications/initialized".to_string(),
            params: None,
        },
        &mut state,
    );
    assert!(
        response.error.is_none(),
        "initialized notification must succeed in test setup"
    );
    state
}

mod batch;
mod initialize;
mod invalid;
mod notifications;
mod tools_call;
