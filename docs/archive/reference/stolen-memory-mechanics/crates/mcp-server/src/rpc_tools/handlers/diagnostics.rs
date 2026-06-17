use anyhow::Result;
use obsidian_memory_core::{Engine, sanitize_value_for_privacy};
use serde_json::{Value, json};

use crate::ServerState;
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_non_empty_string, reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::modes::{parse_optional_privacy_mode, parse_optional_storage_mode};

pub(crate) fn memory_status(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "memory_status", &["auto_index", "privacy_mode"])?;
    let auto_index = parse_optional_bool(args, "memory_status", "auto_index")?.unwrap_or(false);
    let privacy_mode =
        parse_optional_privacy_mode(args, "memory_status", "privacy_mode")?.unwrap_or_default();
    let engine = state.bound_engine()?;
    maybe_auto_rebuild(&engine, auto_index)?;
    let mut payload = serde_json::to_value(engine.memory_status()?)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

pub(crate) fn preflight(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "preflight", &["privacy_mode"])?;
    let privacy_mode =
        parse_optional_privacy_mode(args, "preflight", "privacy_mode")?.unwrap_or_default();
    if let Some(binding_failure) = state.binding_failure() {
        let mut payload = json!({
            "status": "warning",
            "project_path": state.project_path.display().to_string(),
            "project_root": state.project_path.display().to_string(),
            "memory_root": "",
            "storage_mode": state.default_storage_mode().as_str(),
            "safe_recovery_hint": binding_failure.details["safe_recovery_hint"]
                .as_str()
                .unwrap_or("call set_project before using project-scoped tools"),
            "legacy_root_layout_detected": false,
            "binding_status": state.binding_status(),
            "binding_source": state.binding_source(),
            "errors": [],
            "warnings": [binding_failure.message]
        });
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        return tool_result(payload);
    }
    let engine = state.bound_engine()?;
    let mut payload = serde_json::to_value(engine.preflight_status()?)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

pub(crate) fn index_status(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "index_status", &["auto_index", "privacy_mode"])?;
    let auto_index = parse_optional_bool(args, "index_status", "auto_index")?.unwrap_or(false);
    let privacy_mode =
        parse_optional_privacy_mode(args, "index_status", "privacy_mode")?.unwrap_or_default();
    let engine = state.bound_engine()?;
    maybe_auto_rebuild(&engine, auto_index)?;
    let mut payload = serde_json::to_value(engine.index_status()?)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

pub(crate) fn rebuild_index(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "rebuild_index", &["privacy_mode"])?;
    let privacy_mode =
        parse_optional_privacy_mode(args, "rebuild_index", "privacy_mode")?.unwrap_or_default();
    let engine = state.bound_engine()?;
    let mut payload = serde_json::to_value(engine.rebuild_index()?)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

pub(crate) fn migrate_memory_root(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "migrate_memory_root",
        &[
            "target_storage_mode",
            "dry_run",
            "privacy_mode",
            "source_root",
        ],
    )?;
    let privacy_mode = parse_optional_privacy_mode(args, "migrate_memory_root", "privacy_mode")?
        .unwrap_or_default();
    let dry_run = parse_optional_bool(args, "migrate_memory_root", "dry_run")?.unwrap_or(true);
    let target_storage_mode =
        parse_optional_storage_mode(args, "migrate_memory_root", "target_storage_mode")?
            .unwrap_or_else(|| state.default_storage_mode());
    let source_root = parse_optional_non_empty_string(args, "migrate_memory_root", "source_root")?
        .map(std::path::PathBuf::from);
    let engine = state.bound_engine()?;
    let result = engine.migrate_memory_root(target_storage_mode, dry_run, source_root)?;
    if result.migrated {
        state.bind_project_path(
            state.project_path.clone(),
            target_storage_mode,
            crate::state::ProjectBindingSource::SetProject,
        );
    }
    let mut payload = serde_json::to_value(result)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

fn maybe_auto_rebuild(engine: &Engine, auto_index: bool) -> Result<()> {
    if !auto_index {
        return Ok(());
    }
    let preflight = engine.preflight_status()?;
    if preflight.running_binary_stale || !preflight.errors.is_empty() {
        return Ok(());
    }
    let _ = engine.ensure_index_ready(true);
    Ok(())
}
