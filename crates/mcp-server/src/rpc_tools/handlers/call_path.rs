use anyhow::Result;
use serde_json::Value;

use rmu_core::{Engine, MigrationMode, PrivacyMode, sanitize_value_for_privacy};

use crate::ServerState;
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_usize_with_min, parse_required_non_empty_string,
    reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{ensure_query_index_ready, parse_optional_migration_mode, parse_optional_privacy_mode};

pub(super) fn call_path(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "call_path",
        &[
            "from",
            "to",
            "max_hops",
            "auto_index",
            "privacy_mode",
            "migration_mode",
        ],
    )?;
    let from = parse_required_non_empty_string(args, "call_path", "from")?;
    let to = parse_required_non_empty_string(args, "call_path", "to")?;
    let max_hops = parse_optional_usize_with_min(args, "call_path", "max_hops", 1, 6)?;
    let auto_index = parse_optional_bool(args, "call_path", "auto_index")?.unwrap_or(false);
    let privacy_mode =
        parse_optional_privacy_mode(args, "call_path", "privacy_mode")?.unwrap_or(PrivacyMode::Off);
    let migration_mode = parse_optional_migration_mode(args, "call_path", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);

    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )?;
    ensure_query_index_ready(&engine, auto_index)?;
    let result = engine.call_path(&from, &to, max_hops)?;
    let mut payload = serde_json::to_value(result)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}
