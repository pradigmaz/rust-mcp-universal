use serde_json::Value;

use crate::ServerState;

use super::response::{invalid_request_response, parse_error_response};
use super::{RpcRequest, RpcResponse, RpcResponseEnvelope, handle_request};

pub(crate) fn process_raw_message(
    raw_message: &str,
    state: &mut ServerState,
) -> Option<RpcResponseEnvelope> {
    let raw_value: Value = match serde_json::from_str(raw_message) {
        Ok(value) => value,
        Err(err) => {
            return Some(parse_error_response(err.to_string()));
        }
    };

    if raw_value.is_array() {
        return Some(invalid_request_response(
            "batch requests are not supported".to_string(),
            None,
        ));
    }

    match parse_request_value(raw_value) {
        Ok(req) => process_parsed_request(req, state),
        Err(response) => Some(response),
    }
}

fn process_parsed_request(req: RpcRequest, state: &mut ServerState) -> Option<RpcResponse> {
    if req.id.is_none() {
        let _ = handle_request(req, state);
        None
    } else {
        Some(handle_request(req, state))
    }
}

fn parse_request_value(raw_value: Value) -> std::result::Result<RpcRequest, RpcResponse> {
    let Some(object) = raw_value.as_object() else {
        return Err(invalid_request_response(
            "request must be a JSON object".to_string(),
            None,
        ));
    };

    let id = object.get("id").cloned();
    if let Some(id_value) = id.as_ref() {
        if !is_valid_request_id(id_value) {
            return Err(invalid_request_response(
                "id must be string, number, null, or omitted".to_string(),
                None,
            ));
        }
    }

    match object.get("jsonrpc") {
        Some(Value::String(version)) if version == "2.0" => {}
        Some(_) => {
            return Err(invalid_request_response(
                "jsonrpc must be \"2.0\"".to_string(),
                id,
            ));
        }
        None => {
            return Err(invalid_request_response(
                "jsonrpc must be \"2.0\"".to_string(),
                id,
            ));
        }
    }

    let Some(method) = object.get("method").and_then(Value::as_str) else {
        return Err(invalid_request_response(
            "method must be a string".to_string(),
            id,
        ));
    };

    Ok(RpcRequest {
        jsonrpc: Some("2.0".to_string()),
        id,
        method: method.to_string(),
        params: object.get("params").cloned(),
    })
}

fn is_valid_request_id(id: &Value) -> bool {
    matches!(id, Value::Null | Value::String(_) | Value::Number(_))
}
