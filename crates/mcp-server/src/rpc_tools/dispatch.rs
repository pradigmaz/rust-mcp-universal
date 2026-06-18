mod benchmark;
mod indexing;
mod maintenance;
mod navigation;
mod parsing;
mod project;
mod quality;
mod search;
mod usage;

use anyhow::Result;
use rmu_core::{Engine, MigrationMode};
use serde_json::{Value, json};

use crate::ServerState;

use super::errors::{invalid_params_error, is_invalid_params_error, tool_domain_error};
use super::result::{tool_compatibility_error_result, tool_state_error_result};

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

    if !is_known_tool(name) {
        return Err(invalid_params_error(format!("unknown tool: {name}")));
    }

    if tool_requires_bound_project(name) {
        if let Some(binding_failure) = state.binding_failure() {
            return Ok(tool_state_error_result(
                binding_failure.code,
                binding_failure.message,
                binding_failure.details,
            ));
        }
    }

    if name != "preflight" && tool_requires_bound_project(name) {
        if let Some(compatibility_error) = runtime_compatibility_guard(state)? {
            return Ok(compatibility_error);
        }
    }

    let result = match name {
        "set_project_path" => project::set_project_path(&args, state),
        "install_ignore_rules" => project::install_ignore_rules_tool(&args, state),
        "index_status" => project::index_status(&args, state),
        "workspace_brief" => project::workspace_brief(&args, state),
        "usage_stats" => usage::usage_stats(&args, state),
        "index" | "semantic_index" => indexing::index(&args, name, state),
        "scope_preview" => indexing::scope_preview(&args, state),
        "delete_index" => indexing::delete_index(&args, state),
        _ => dispatch_tool_domain(name, &args, state),
    };
    usage::record_tool_usage(state, name, &result);
    result
}

fn dispatch_tool_domain(name: &str, args: &Value, state: &mut ServerState) -> Result<Value> {
    if let Some(result) = search::dispatch(name, args, state) {
        return result;
    }
    if let Some(result) = navigation::dispatch(name, args, state) {
        return result;
    }
    if let Some(result) = quality::dispatch(name, args, state) {
        return result;
    }
    if let Some(result) = benchmark::dispatch(name, args, state) {
        return result;
    }
    if let Some(result) = maintenance::dispatch(name, args, state) {
        return result;
    }
    unreachable!("known tools are handled before dispatch")
}

fn is_known_tool(name: &str) -> bool {
    matches!(
        name,
        "set_project_path"
            | "install_ignore_rules"
            | "index_status"
            | "workspace_brief"
            | "usage_stats"
            | "agent_bootstrap"
            | "index"
            | "semantic_index"
            | "scope_preview"
            | "delete_index"
            | "preflight"
            | "symbol_lookup"
            | "symbol_lookup_v2"
            | "symbol_references"
            | "symbol_references_v2"
            | "symbol_body"
            | "related_files"
            | "related_files_v2"
            | "call_path"
            | "route_trace"
            | "constraint_evidence"
            | "concept_cluster"
            | "contract_trace"
            | "divergence_report"
            | "search_candidates"
            | "semantic_search"
            | "rule_violations"
            | "quality_hotspots"
            | "quality_snapshot"
            | "sensitive_data"
            | "signal_memory"
            | "mark_signal_memory"
            | "build_context_under_budget"
            | "context_pack"
            | "query_report"
            | "query_benchmark"
            | "db_maintenance"
    )
}

fn tool_requires_bound_project(name: &str) -> bool {
    !matches!(name, "set_project_path" | "preflight" | "usage_stats")
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
