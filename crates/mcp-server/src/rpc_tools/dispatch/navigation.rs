use anyhow::Result;
use serde_json::Value;

use crate::ServerState;
use crate::rpc_tools::handlers::{
    call_path, concept_cluster, constraint_evidence, contract_trace, divergence_report,
    related_files, related_files_v2, route_trace, symbol_body, symbol_lookup, symbol_lookup_v2,
    symbol_references, symbol_references_v2,
};

use super::into_tool_error;

pub(super) fn dispatch(name: &str, args: &Value, state: &mut ServerState) -> Option<Result<Value>> {
    Some(match name {
        "symbol_lookup" => symbol_lookup(args, state).map_err(into_tool_error),
        "symbol_lookup_v2" => symbol_lookup_v2(args, state).map_err(into_tool_error),
        "symbol_references" => symbol_references(args, state).map_err(into_tool_error),
        "symbol_references_v2" => symbol_references_v2(args, state).map_err(into_tool_error),
        "symbol_body" => symbol_body(args, state).map_err(into_tool_error),
        "related_files" => related_files(args, state).map_err(into_tool_error),
        "related_files_v2" => related_files_v2(args, state).map_err(into_tool_error),
        "call_path" => call_path(args, state).map_err(into_tool_error),
        "route_trace" => route_trace(args, state).map_err(into_tool_error),
        "constraint_evidence" => constraint_evidence(args, state).map_err(into_tool_error),
        "concept_cluster" => concept_cluster(args, state).map_err(into_tool_error),
        "contract_trace" => contract_trace(args, state).map_err(into_tool_error),
        "divergence_report" => divergence_report(args, state).map_err(into_tool_error),
        _ => return None,
    })
}
