use anyhow::Result;
use serde_json::Value;

use crate::ServerState;
use crate::rpc_tools::handlers::{
    agent_bootstrap, build_context_under_budget, context_pack, query_benchmark, query_report,
    search_candidates, semantic_search,
};
use crate::rpc_tools::registry::ToolHandler;

pub(super) fn dispatch(
    handler: ToolHandler,
    args: &Value,
    state: &mut ServerState,
) -> Option<Result<Value>> {
    let result = match handler {
        ToolHandler::AgentBootstrap => agent_bootstrap(args, state),
        ToolHandler::BuildContextUnderBudget => build_context_under_budget(args, state),
        ToolHandler::ContextPack => context_pack(args, state),
        ToolHandler::QueryBenchmark => query_benchmark(args, state),
        ToolHandler::QueryReport => query_report(args, state),
        ToolHandler::SearchCandidates => search_candidates(args, state),
        ToolHandler::SemanticSearch => semantic_search(args, state),
        _ => return None,
    };
    Some(result.map_err(super::into_tool_error))
}
