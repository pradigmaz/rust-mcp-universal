use std::path::PathBuf;

use anyhow::Result;
use rmu_core::{Engine, IgnoreInstallTarget, install_ignore_rules};
use serde_json::{Value, json};

use crate::ServerState;
use crate::rpc_tools::errors::{invalid_params_error, tool_domain_error};
use crate::rpc_tools::parsing::{parse_required_non_empty_string, reject_unknown_fields};
use crate::rpc_tools::result::tool_result;

use super::parsing::{parse_optional_ignore_install_target, parse_optional_migration_mode};

pub(super) fn set_project_path(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "set_project_path", &["project_path"])?;
    let project_path = parse_required_non_empty_string(args, "set_project_path", "project_path")?;
    let path = PathBuf::from(&project_path);
    let metadata = std::fs::metadata(&path).map_err(|_| {
        invalid_params_error(format!(
            "set_project_path `project_path` does not exist: {project_path}"
        ))
    })?;
    if !metadata.is_dir() {
        return Err(invalid_params_error(format!(
            "set_project_path `project_path` must be an existing directory: {project_path}"
        )));
    }
    state.project_path = path;
    tool_result(json!({
        "ok": true,
        "project_path": project_path,
        "gitignore_created": false,
        "gitignore_updated": false
    }))
}

pub(super) fn install_ignore_rules_tool(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "install_ignore_rules", &["target"])?;
    let target = parse_optional_ignore_install_target(args, "install_ignore_rules")?
        .unwrap_or(IgnoreInstallTarget::GitInfoExclude);
    let report = install_ignore_rules(&state.project_path, target)
        .map_err(|err| tool_domain_error(err.to_string()))?;
    tool_result(serde_json::to_value(report)?)
}

pub(super) fn index_status(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "index_status", &["migration_mode"])?;
    let migration_mode = parse_optional_migration_mode(args, "index_status")?
        .unwrap_or(rmu_core::MigrationMode::Auto);
    let engine = Engine::new_read_only_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )
    .map_err(|err| tool_domain_error(err.to_string()))?;
    let status = engine
        .index_status()
        .map_err(|err| tool_domain_error(err.to_string()))?;
    tool_result(serde_json::to_value(status)?)
}

pub(super) fn workspace_brief(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "workspace_brief", &["migration_mode"])?;
    let migration_mode = parse_optional_migration_mode(args, "workspace_brief")?
        .unwrap_or(rmu_core::MigrationMode::Auto);
    let engine = Engine::new_read_only_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )
    .map_err(|err| tool_domain_error(err.to_string()))?;
    let brief = engine
        .workspace_brief_with_policy(false)
        .map_err(|err| tool_domain_error(err.to_string()))?;
    tool_result(serde_json::to_value(brief)?)
}
