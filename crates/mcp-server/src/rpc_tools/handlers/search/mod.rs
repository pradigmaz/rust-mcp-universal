use anyhow::Result;
use serde_json::Value;

use crate::ServerState;

pub(crate) fn agent_bootstrap(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::agent_bootstrap::agent_bootstrap(args, state)
}

pub(crate) fn build_context_under_budget(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::build_context_under_budget::build_context_under_budget(args, state)
}

pub(crate) fn context_pack(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::context_pack::context_pack(args, state)
}

pub(crate) fn query_report(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::query_report::query_report(args, state)
}

pub(crate) fn search_candidates(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::search_candidates::search_candidates(args, state)
}

pub(crate) fn semantic_search(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::semantic_search::semantic_search(args, state)
}
