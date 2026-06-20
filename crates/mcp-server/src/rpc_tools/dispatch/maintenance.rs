use anyhow::Result;
use serde_json::Value;

use crate::ServerState;
use crate::rpc_tools::handlers::{db_maintenance, preflight};
use crate::rpc_tools::registry::ToolHandler;

pub(super) fn dispatch(
    handler: ToolHandler,
    args: &Value,
    state: &mut ServerState,
) -> Option<Result<Value>> {
    let result = match handler {
        ToolHandler::DbMaintenance => db_maintenance(args, state),
        ToolHandler::Preflight => preflight(args, state),
        _ => return None,
    };
    Some(result.map_err(super::into_tool_error))
}
