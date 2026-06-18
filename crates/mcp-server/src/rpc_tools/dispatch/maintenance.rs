use anyhow::Result;
use serde_json::Value;

use crate::ServerState;
use crate::rpc_tools::handlers::{db_maintenance, preflight};

use super::into_tool_error;

pub(super) fn dispatch(name: &str, args: &Value, state: &mut ServerState) -> Option<Result<Value>> {
    Some(match name {
        "preflight" => preflight(args, state).map_err(into_tool_error),
        "db_maintenance" => db_maintenance(args, state).map_err(into_tool_error),
        _ => return None,
    })
}
