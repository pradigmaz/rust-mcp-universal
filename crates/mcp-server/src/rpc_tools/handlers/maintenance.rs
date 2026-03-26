use anyhow::Result;
use serde_json::{Value, json};

use rmu_core::{
    DbMaintenanceOptions, Engine, MigrationMode, PrivacyMode, sanitize_value_for_privacy,
};

use crate::ServerState;
use crate::rpc_tools::parsing::{parse_optional_bool, reject_unknown_fields};
use crate::rpc_tools::result::tool_result;

use super::{parse_optional_migration_mode, parse_optional_privacy_mode};

const RUNNING_BINARY_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(super) fn preflight(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "preflight", &["privacy_mode", "migration_mode"])?;
    let privacy_mode =
        parse_optional_privacy_mode(args, "preflight", "privacy_mode")?.unwrap_or(PrivacyMode::Off);
    let migration_mode = parse_optional_migration_mode(args, "preflight", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);
    if let Some(binding_failure) = state.binding_failure() {
        let mut payload = json!({
            "status": "warning",
            "project_path": state.project_path.display().to_string(),
            "binary_path": current_binary_path(),
            "running_binary_version": RUNNING_BINARY_VERSION,
            "running_binary_stale": false,
            "same_binary_other_pids": [],
            "stale_process_suspected": false,
            "safe_recovery_hint": binding_failure.details["safe_recovery_hint"]
                .as_str()
                .unwrap_or("provide initialize roots/projectPath or call set_project_path before using project-scoped tools"),
            "binding_status": state.binding_status(),
            "resolved_project_path": Value::Null,
            "resolved_db_path": state.db_path.as_ref().map(|value| value.display().to_string()),
            "db_pinned": state.db_pinned(),
            "binding_errors": [binding_failure.message],
            "errors": []
        });
        if let Some(binding_source) = state.binding_source() {
            payload["binding_source"] = json!(binding_source);
        }
        if cfg!(windows) {
            payload["launcher_recommended"] = json!("scripts/rmu-mcp-server-fresh.cmd");
        }
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        return tool_result(payload);
    }
    let engine = Engine::new_read_only_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )?;
    let mut payload = serde_json::to_value(engine.preflight_status()?)?;
    let payload_object = payload
        .as_object_mut()
        .expect("preflight payload must serialize as object");
    payload_object.insert("binding_status".to_string(), json!(state.binding_status()));
    payload_object.insert("binding_source".to_string(), json!(state.binding_source()));
    payload_object.insert(
        "resolved_project_path".to_string(),
        json!(
            state
                .resolved_project_path()
                .map(|value| value.display().to_string())
        ),
    );
    payload_object.insert(
        "resolved_db_path".to_string(),
        json!(engine.db_path.display().to_string()),
    );
    payload_object.insert("db_pinned".to_string(), json!(state.db_pinned()));
    payload_object.insert("binding_errors".to_string(), json!(Vec::<String>::new()));
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

fn current_binary_path() -> String {
    std::env::current_exe()
        .ok()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub(super) fn db_maintenance(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "db_maintenance",
        &[
            "integrity_check",
            "checkpoint",
            "vacuum",
            "analyze",
            "stats",
            "prune",
            "privacy_mode",
            "migration_mode",
        ],
    )?;

    let integrity_check =
        parse_optional_bool(args, "db_maintenance", "integrity_check")?.unwrap_or(false);
    let checkpoint = parse_optional_bool(args, "db_maintenance", "checkpoint")?.unwrap_or(false);
    let vacuum = parse_optional_bool(args, "db_maintenance", "vacuum")?.unwrap_or(false);
    let analyze = parse_optional_bool(args, "db_maintenance", "analyze")?.unwrap_or(false);
    let stats = parse_optional_bool(args, "db_maintenance", "stats")?.unwrap_or(false);
    let prune = parse_optional_bool(args, "db_maintenance", "prune")?.unwrap_or(false);
    let privacy_mode = parse_optional_privacy_mode(args, "db_maintenance", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let migration_mode = parse_optional_migration_mode(args, "db_maintenance", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);

    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )?;
    let result = engine.db_maintenance(DbMaintenanceOptions {
        integrity_check,
        checkpoint,
        vacuum,
        analyze,
        stats,
        prune,
    })?;
    let mut payload = serde_json::to_value(result)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}
