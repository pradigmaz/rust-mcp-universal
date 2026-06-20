use anyhow::Result;
use serde_json::Value;

use crate::ServerState;
use crate::rpc_tools::handlers::{
    api_surface, complexity_report, dead_code_report, mark_signal_memory, quality_hotspots,
    quality_snapshot, rule_violations, sensitive_data, signal_memory,
};
use crate::rpc_tools::registry::ToolHandler;

pub(super) fn dispatch(
    handler: ToolHandler,
    args: &Value,
    state: &mut ServerState,
) -> Option<Result<Value>> {
    let result = match handler {
        ToolHandler::ApiSurface => api_surface(args, state),
        ToolHandler::ComplexityReport => complexity_report(args, state),
        ToolHandler::DeadCodeReport => dead_code_report(args, state),
        ToolHandler::MarkSignalMemory => mark_signal_memory(args, state),
        ToolHandler::QualityHotspots => quality_hotspots(args, state),
        ToolHandler::QualitySnapshot => quality_snapshot(args, state),
        ToolHandler::RuleViolations => rule_violations(args, state),
        ToolHandler::SensitiveData => sensitive_data(args, state),
        ToolHandler::SignalMemory => signal_memory(args, state),
        _ => return None,
    };
    Some(result.map_err(super::into_tool_error))
}
