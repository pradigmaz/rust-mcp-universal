use serde_json::Value;

use super::{RpcError, RpcResponse};

pub(super) fn success_response(id: Option<Value>, value: Value) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(value),
        error: None,
    }
}

pub(super) fn internal_error_response(message: String, id: Option<Value>) -> RpcResponse {
    rpc_error_response(-32603, message, id)
}

pub(crate) fn parse_error_response(message: String) -> RpcResponse {
    rpc_error_response(-32700, message, None)
}

pub(super) fn invalid_request_response(message: String, id: Option<Value>) -> RpcResponse {
    rpc_error_response(-32600, message, id)
}

pub(super) fn invalid_params_response(message: String, id: Option<Value>) -> RpcResponse {
    rpc_error_response(-32602, message, id)
}

pub(super) fn method_not_found_response(id: Option<Value>, method: String) -> RpcResponse {
    rpc_error_response(-32601, format!("method not found: {method}"), id)
}

fn rpc_error_response(code: i64, message: String, id: Option<Value>) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(RpcError { code, message }),
    }
}
