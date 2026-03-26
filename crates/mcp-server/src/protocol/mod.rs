mod parse;
mod response;
mod validation;

use anyhow::Result;
use rmu_core::sanitize_error_message;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::ServerState;
use crate::rpc_tools::{
    handle_tool_call, is_invalid_params_error, is_tool_domain_error, tool_error_result, tools_list,
};
use crate::state::SessionLifecycle;

pub(crate) use parse::process_raw_message;
pub(crate) use response::parse_error_response;

pub(crate) const PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Debug, Deserialize)]
pub(crate) struct RpcRequest {
    #[allow(dead_code)]
    pub(crate) jsonrpc: Option<String>,
    pub(crate) id: Option<Value>,
    pub(crate) method: String,
    pub(crate) params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RpcResponse {
    jsonrpc: &'static str,
    pub(crate) id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<RpcError>,
}

pub(crate) type RpcResponseEnvelope = RpcResponse;

#[derive(Debug, Serialize)]
pub(crate) struct RpcError {
    pub(crate) code: i64,
    message: String,
}

pub(crate) fn handle_request(req: RpcRequest, state: &mut ServerState) -> RpcResponse {
    let id = req.id.clone();
    let result: Result<Value> = match req.method.as_str() {
        "initialize" => {
            if req.id.is_none() {
                return response::invalid_request_response(
                    "initialize must be a request with an `id`".to_string(),
                    id,
                );
            }
            if state.lifecycle() != SessionLifecycle::Uninitialized {
                return response::invalid_request_response(
                    "initialize is only allowed before the MCP session starts".to_string(),
                    id,
                );
            }
            if let Err(message) = validation::validate_initialize_params(req.params.as_ref()) {
                return response::invalid_params_response(message, id);
            }
            state.set_lifecycle(SessionLifecycle::AwaitingInitialized);
            Ok(json!({
                "protocolVersion": resolve_protocol_version(req.params.as_ref()),
                "serverInfo": {"name": "rmu-mcp-server", "version": "0.1.0"},
                "capabilities": {
                    "tools": {"listChanged": false}
                }
            }))
        }
        "notifications/initialized" => {
            if req.id.is_some() {
                return response::invalid_request_response(
                    "notifications/initialized must be sent as a notification".to_string(),
                    id,
                );
            }
            if state.lifecycle() != SessionLifecycle::AwaitingInitialized {
                return response::invalid_request_response(
                    "notifications/initialized is only allowed after initialize".to_string(),
                    id,
                );
            }
            state.set_lifecycle(SessionLifecycle::Running);
            Ok(json!({}))
        }
        "ping" => {
            if state.lifecycle() != SessionLifecycle::Running {
                return response::invalid_request_response(
                    "ping is only available after MCP initialization completes".to_string(),
                    id,
                );
            }
            Ok(json!({}))
        }
        "tools/list" => {
            if state.lifecycle() != SessionLifecycle::Running {
                return response::invalid_request_response(
                    "tools/list is only available after MCP initialization completes".to_string(),
                    id,
                );
            }
            Ok(tools_list())
        }
        "tools/call" => {
            if state.lifecycle() != SessionLifecycle::Running {
                return response::invalid_request_response(
                    "tools/call is only available after MCP initialization completes".to_string(),
                    id,
                );
            }
            if let Err(message) = validation::validate_tools_call_params(req.params.as_ref()) {
                return response::invalid_params_response(message, id);
            }
            let privacy_mode =
                validation::extract_privacy_mode_from_tools_call_params(req.params.as_ref());
            Ok(match handle_tool_call(req.params, state) {
                Ok(value) => value,
                Err(err) => {
                    let message = err.to_string();
                    if is_invalid_params_error(&err) {
                        return response::invalid_params_response(message, id);
                    }
                    if !is_tool_domain_error(&err) {
                        return response::internal_error_response(message, id);
                    }
                    tool_error_result(sanitize_error_message(privacy_mode, &message))
                }
            })
        }
        "shutdown" => {
            if req.id.is_none() {
                return response::invalid_request_response(
                    "shutdown must be a request with an `id`".to_string(),
                    id,
                );
            }
            if state.lifecycle() != SessionLifecycle::Running {
                return response::invalid_request_response(
                    "shutdown is only available after MCP initialization completes".to_string(),
                    id,
                );
            }
            state.set_lifecycle(SessionLifecycle::ShutdownRequested);
            Ok(json!({}))
        }
        "exit" => {
            if req.id.is_some() {
                return response::invalid_request_response(
                    "exit must be sent as a notification".to_string(),
                    id,
                );
            }
            if state.lifecycle() != SessionLifecycle::ShutdownRequested {
                return response::invalid_request_response(
                    "exit is only allowed after shutdown".to_string(),
                    id,
                );
            }
            state.request_exit();
            Ok(json!({}))
        }
        _ => return response::method_not_found_response(id, req.method),
    };

    match result {
        Ok(value) => response::success_response(id, value),
        Err(err) => response::internal_error_response(err.to_string(), id),
    }
}

fn resolve_protocol_version(params: Option<&Value>) -> String {
    params
        .and_then(Value::as_object)
        .and_then(|object| object.get("protocolVersion"))
        .and_then(Value::as_str)
        .filter(|version| !version.trim().is_empty())
        .unwrap_or(PROTOCOL_VERSION)
        .to_string()
}
