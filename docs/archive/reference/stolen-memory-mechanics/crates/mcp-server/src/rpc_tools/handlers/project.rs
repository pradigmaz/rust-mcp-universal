use anyhow::Result;
use obsidian_memory_core::{ProjectBindingResult, sanitize_value_for_privacy};
use serde_json::Value;

use crate::ServerState;
use crate::path_input::{resolve_existing_directory_input, supported_directory_input_hint};
use crate::rpc_tools::parsing::{parse_required_non_empty_string, reject_unknown_fields};
use crate::rpc_tools::result::tool_result;
use crate::state::ProjectBindingSource;

use super::modes::{parse_optional_privacy_mode, parse_optional_storage_mode};

pub(crate) fn set_project(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "set_project", &["project_path", "storage_mode"])?;
    let raw = parse_required_non_empty_string(args, "set_project", "project_path")?;
    let project_path = resolve_existing_directory_input(&raw).ok_or_else(|| {
        crate::rpc_tools::errors::invalid_params_error(format!(
            "set_project `project_path` must point to an existing directory: {raw}; {}",
            supported_directory_input_hint()
        ))
    })?;
    let storage_mode = parse_optional_storage_mode(args, "set_project", "storage_mode")?
        .unwrap_or_else(|| state.default_storage_mode());
    state.bind_project_path(
        project_path.clone(),
        storage_mode,
        ProjectBindingSource::SetProject,
    );
    let context = state.bound_context()?;

    tool_result(serde_json::to_value(ProjectBindingResult {
        project: context.project_name(),
        project_root: context.project_root.display().to_string(),
        memory_root: context.memory_root.display().to_string(),
        storage_mode: context.storage_mode,
        status: "bound".to_string(),
    })?)
}

pub(crate) fn project_brief(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "project_brief", &["auto_index", "privacy_mode"])?;
    let privacy_mode =
        parse_optional_privacy_mode(args, "project_brief", "privacy_mode")?.unwrap_or_default();
    let auto_index =
        crate::rpc_tools::parsing::parse_optional_bool(args, "project_brief", "auto_index")?
            .unwrap_or(false);
    let engine = state.bound_engine()?;
    engine.ensure_index_ready(auto_index)?;
    let mut payload = serde_json::to_value(engine.project_brief()?)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}
