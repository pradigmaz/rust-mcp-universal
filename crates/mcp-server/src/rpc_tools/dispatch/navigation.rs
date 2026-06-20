use anyhow::Result;
use serde_json::Value;

use crate::ServerState;
use crate::rpc_tools::handlers::{
    call_path, concept_cluster, constraint_evidence, contract_trace, divergence_report,
    related_files, related_files_v2, route_trace, symbol_body, symbol_lookup, symbol_lookup_v2,
    symbol_references, symbol_references_v2,
};
use crate::rpc_tools::registry::ToolHandler;

pub(super) fn dispatch(
    handler: ToolHandler,
    args: &Value,
    state: &mut ServerState,
) -> Option<Result<Value>> {
    let result = match handler {
        ToolHandler::CallPath => call_path(args, state),
        ToolHandler::ConceptCluster => concept_cluster(args, state),
        ToolHandler::ConstraintEvidence => constraint_evidence(args, state),
        ToolHandler::ContractTrace => contract_trace(args, state),
        ToolHandler::DivergenceReport => divergence_report(args, state),
        ToolHandler::RelatedFiles => related_files(args, state),
        ToolHandler::RelatedFilesV2 => related_files_v2(args, state),
        ToolHandler::RouteTrace => route_trace(args, state),
        ToolHandler::SymbolBody => symbol_body(args, state),
        ToolHandler::SymbolLookup => symbol_lookup(args, state),
        ToolHandler::SymbolLookupV2 => symbol_lookup_v2(args, state),
        ToolHandler::SymbolReferences => symbol_references(args, state),
        ToolHandler::SymbolReferencesV2 => symbol_references_v2(args, state),
        _ => return None,
    };
    Some(result.map_err(super::into_tool_error))
}
