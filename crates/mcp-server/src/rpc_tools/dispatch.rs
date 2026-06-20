mod indexing;
mod maintenance;
mod navigation;
mod parsing;
mod project;
mod quality;
mod query;
mod usage;

use anyhow::Result;
use rmu_core::{Engine, MigrationMode};
use serde_json::{Value, json};

use crate::ServerState;

use super::errors::{invalid_params_error, is_invalid_params_error, tool_domain_error};
use super::registry::{ToolHandler, metadata, names};
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
    let metadata =
        metadata(name).ok_or_else(|| invalid_params_error(format!("unknown tool: {name}")))?;

    if metadata.requires_bound_project {
        if let Some(binding_failure) = state.binding_failure() {
            return Ok(tool_state_error_result(
                binding_failure.code,
                binding_failure.message,
                binding_failure.details,
            ));
        }
    }

    if name != names::PREFLIGHT && metadata.requires_bound_project {
        if let Some(compatibility_error) = runtime_compatibility_guard(state)? {
            return Ok(compatibility_error);
        }
    }

    let result = dispatch_registered_tool(metadata.handler, name, &args, state);
    usage::record_tool_usage(state, name, &result);
    result
}

fn dispatch_registered_tool(
    handler: ToolHandler,
    name: &str,
    args: &Value,
    state: &mut ServerState,
) -> Result<Value> {
    if let Some(result) = query::dispatch(handler, args, state) {
        return result;
    }
    if let Some(result) = navigation::dispatch(handler, args, state) {
        return result;
    }
    if let Some(result) = quality::dispatch(handler, args, state) {
        return result;
    }
    if let Some(result) = maintenance::dispatch(handler, args, state) {
        return result;
    }
    match handler {
        ToolHandler::DeleteIndex => indexing::delete_index(args, state),
        ToolHandler::Index => indexing::index(args, name, state),
        ToolHandler::IndexStatus => project::index_status(args, state),
        ToolHandler::InstallIgnoreRules => project::install_ignore_rules_tool(args, state),
        ToolHandler::ScopePreview => indexing::scope_preview(args, state),
        ToolHandler::SetProjectPath => project::set_project_path(args, state),
        ToolHandler::UsageStats => usage::usage_stats(args, state),
        ToolHandler::WorkspaceBrief => project::workspace_brief(args, state),
        _ => unreachable!("tool handler must be covered by a dispatch domain"),
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
