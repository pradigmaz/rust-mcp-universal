use anyhow::Result;
use serde_json::Value;

use crate::ServerState;
use crate::rpc_tools::handlers::query_benchmark;

use super::into_tool_error;

pub(super) fn dispatch(name: &str, args: &Value, state: &mut ServerState) -> Option<Result<Value>> {
    Some(match name {
        "query_benchmark" => query_benchmark(args, state).map_err(into_tool_error),
        _ => return None,
    })
}
