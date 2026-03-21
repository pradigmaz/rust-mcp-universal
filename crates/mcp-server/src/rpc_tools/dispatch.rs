mod indexing;
mod parsing;
mod project;

use anyhow::Result;
use serde_json::{Value, json};

use crate::ServerState;

use super::errors::{invalid_params_error, is_invalid_params_error, tool_domain_error};
use super::handlers::{
    agent_bootstrap, build_context_under_budget, call_path, context_pack, db_maintenance,
    query_benchmark, query_report, related_files, related_files_v2, rule_violations,
    search_candidates, semantic_search, symbol_lookup, symbol_lookup_v2, symbol_references,
    symbol_references_v2,
};

pub(super) fn handle_tool_call(params: Option<Value>, state: &mut ServerState) -> Result<Value> {
    let params = params.ok_or_else(|| invalid_params_error("tools/call params are required"))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params_error("tools/call requires string field `name`"))?;
    let args = match params.get("arguments") {
        Some(value) if value.is_object() => value.clone(),
        Some(value) => {
            return Err(invalid_params_error(format!(
                "tools/call `arguments` must be object, got {}",
                value
            )));
        }
        None => json!({}),
    };

    match name {
        "set_project_path" => project::set_project_path(&args, state),
        "install_ignore_rules" => project::install_ignore_rules_tool(&args, state),
        "index_status" => project::index_status(&args, state),
        "workspace_brief" => project::workspace_brief(&args, state),
        "agent_bootstrap" => agent_bootstrap(&args, state).map_err(into_tool_error),
        "index" | "semantic_index" => indexing::index(&args, name, state),
        "scope_preview" => indexing::scope_preview(&args, state),
        "delete_index" => indexing::delete_index(&args, state),
        "symbol_lookup" => symbol_lookup(&args, state).map_err(into_tool_error),
        "symbol_lookup_v2" => symbol_lookup_v2(&args, state).map_err(into_tool_error),
        "symbol_references" => symbol_references(&args, state).map_err(into_tool_error),
        "symbol_references_v2" => symbol_references_v2(&args, state).map_err(into_tool_error),
        "related_files" => related_files(&args, state).map_err(into_tool_error),
        "related_files_v2" => related_files_v2(&args, state).map_err(into_tool_error),
        "call_path" => call_path(&args, state).map_err(into_tool_error),
        "search_candidates" => search_candidates(&args, state).map_err(into_tool_error),
        "semantic_search" => semantic_search(&args, state).map_err(into_tool_error),
        "rule_violations" => rule_violations(&args, state).map_err(into_tool_error),
        "build_context_under_budget" => {
            build_context_under_budget(&args, state).map_err(into_tool_error)
        }
        "context_pack" => context_pack(&args, state).map_err(into_tool_error),
        "query_report" => query_report(&args, state).map_err(into_tool_error),
        "query_benchmark" => query_benchmark(&args, state).map_err(into_tool_error),
        "db_maintenance" => db_maintenance(&args, state).map_err(into_tool_error),
        _ => Err(invalid_params_error(format!("unknown tool: {name}"))),
    }
}

fn into_tool_error(err: anyhow::Error) -> anyhow::Error {
    if is_invalid_params_error(&err) {
        err
    } else {
        tool_domain_error(err.to_string())
    }
}
