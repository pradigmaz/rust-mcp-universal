use anyhow::Result;
use serde_json::Value;

use crate::ServerState;
use crate::rpc_tools::handlers::{
    mark_signal_memory, quality_hotspots, quality_snapshot, rule_violations, sensitive_data,
    signal_memory,
};

use super::into_tool_error;

pub(super) fn dispatch(name: &str, args: &Value, state: &mut ServerState) -> Option<Result<Value>> {
    Some(match name {
        "rule_violations" => rule_violations(args, state).map_err(into_tool_error),
        "quality_hotspots" => quality_hotspots(args, state).map_err(into_tool_error),
        "quality_snapshot" => quality_snapshot(args, state).map_err(into_tool_error),
        "sensitive_data" => sensitive_data(args, state).map_err(into_tool_error),
        "signal_memory" => signal_memory(args, state).map_err(into_tool_error),
        "mark_signal_memory" => mark_signal_memory(args, state).map_err(into_tool_error),
        _ => return None,
    })
}
