use anyhow::Result;
use serde_json::Value;

use rmu_core::{
    DbMaintenanceOptions, Engine, MigrationMode, PrivacyMode, sanitize_value_for_privacy,
};

use crate::ServerState;
use crate::rpc_tools::parsing::{parse_optional_bool, reject_unknown_fields};
use crate::rpc_tools::result::tool_result;

use super::{parse_optional_migration_mode, parse_optional_privacy_mode};

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
