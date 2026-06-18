use anyhow::Result;
use serde_json::Value;

use crate::ServerState;
use crate::rpc_tools::handlers::{
    agent_bootstrap, build_context_under_budget, context_pack, query_report, search_candidates,
    semantic_search,
};

use super::into_tool_error;

pub(super) fn dispatch(name: &str, args: &Value, state: &mut ServerState) -> Option<Result<Value>> {
    Some(match name {
        "agent_bootstrap" => agent_bootstrap(args, state).map_err(into_tool_error),
        "search_candidates" => search_candidates(args, state).map_err(into_tool_error),
        "semantic_search" => semantic_search(args, state).map_err(into_tool_error),
        "build_context_under_budget" => {
            build_context_under_budget(args, state).map_err(into_tool_error)
        }
        "context_pack" => context_pack(args, state).map_err(into_tool_error),
        "query_report" => query_report(args, state).map_err(into_tool_error),
        _ => return None,
    })
}
