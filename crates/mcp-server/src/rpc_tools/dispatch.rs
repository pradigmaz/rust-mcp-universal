mod indexing;
mod parsing;
mod project;

use anyhow::Result;
use rmu_core::{Engine, MigrationMode};
use serde_json::{Value, json};

use crate::ServerState;

use super::errors::{invalid_params_error, is_invalid_params_error, tool_domain_error};
use super::handlers::{
    agent_bootstrap, build_context_under_budget, call_path, concept_cluster, constraint_evidence,
    context_pack, db_maintenance, divergence_report, preflight, quality_hotspots, query_benchmark,
    query_report, related_files, related_files_v2, route_trace, rule_violations, search_candidates,
    semantic_search, symbol_body, symbol_lookup, symbol_lookup_v2, symbol_references,
    symbol_references_v2,
};
use super::result::tool_compatibility_error_result;

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

    if name != "preflight" {
        if let Some(compatibility_error) = runtime_compatibility_guard(state)? {
            return Ok(compatibility_error);
        }
    }

    match name {
        "set_project_path" => project::set_project_path(&args, state),
        "install_ignore_rules" => project::install_ignore_rules_tool(&args, state),
        "index_status" => project::index_status(&args, state),
        "workspace_brief" => project::workspace_brief(&args, state),
        "agent_bootstrap" => agent_bootstrap(&args, state).map_err(into_tool_error),
        "index" | "semantic_index" => indexing::index(&args, name, state),
        "scope_preview" => indexing::scope_preview(&args, state),
        "delete_index" => indexing::delete_index(&args, state),
        "preflight" => preflight(&args, state).map_err(into_tool_error),
        "symbol_lookup" => symbol_lookup(&args, state).map_err(into_tool_error),
        "symbol_lookup_v2" => symbol_lookup_v2(&args, state).map_err(into_tool_error),
        "symbol_references" => symbol_references(&args, state).map_err(into_tool_error),
        "symbol_references_v2" => symbol_references_v2(&args, state).map_err(into_tool_error),
        "symbol_body" => symbol_body(&args, state).map_err(into_tool_error),
        "related_files" => related_files(&args, state).map_err(into_tool_error),
        "related_files_v2" => related_files_v2(&args, state).map_err(into_tool_error),
        "call_path" => call_path(&args, state).map_err(into_tool_error),
        "route_trace" => route_trace(&args, state).map_err(into_tool_error),
        "constraint_evidence" => constraint_evidence(&args, state).map_err(into_tool_error),
        "concept_cluster" => concept_cluster(&args, state).map_err(into_tool_error),
        "divergence_report" => divergence_report(&args, state).map_err(into_tool_error),
        "search_candidates" => search_candidates(&args, state).map_err(into_tool_error),
        "semantic_search" => semantic_search(&args, state).map_err(into_tool_error),
        "rule_violations" => rule_violations(&args, state).map_err(into_tool_error),
        "quality_hotspots" => quality_hotspots(&args, state).map_err(into_tool_error),
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

fn runtime_compatibility_guard(state: &ServerState) -> Result<Option<Value>> {
    let engine = Engine::new_read_only_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        MigrationMode::Auto,
    )?;
    let status = engine.preflight_status()?;
    if status.running_binary_stale || !status.errors.is_empty() {
        let message = status
            .errors
            .first()
            .cloned()
            .unwrap_or_else(|| "compatibility check failed before tool execution".to_string());
        return Ok(Some(tool_compatibility_error_result(
            message,
            Some(&status),
        )));
    }
    Ok(None)
}

fn into_tool_error(err: anyhow::Error) -> anyhow::Error {
    if is_invalid_params_error(&err) {
        err
    } else {
        tool_domain_error(err.to_string())
    }
}
