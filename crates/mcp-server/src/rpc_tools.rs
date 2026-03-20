use anyhow::Result;
use serde_json::Value;

use crate::ServerState;

mod dispatch;
mod errors;
mod handlers;
mod parsing;
mod registry;
mod result;

pub fn tools_list() -> Value {
    registry::tools_list()
}

pub fn handle_tool_call(params: Option<Value>, state: &mut ServerState) -> Result<Value> {
    dispatch::handle_tool_call(params, state)
}

pub(crate) fn is_invalid_params_error(err: &anyhow::Error) -> bool {
    errors::is_invalid_params_error(err)
}

pub(crate) fn is_tool_domain_error(err: &anyhow::Error) -> bool {
    errors::is_tool_domain_error(err)
}

pub fn tool_error_result(message: String) -> Value {
    result::tool_error_result(message)
}

#[cfg(test)]
#[path = "rpc_tools_tests.rs"]
mod tests;
