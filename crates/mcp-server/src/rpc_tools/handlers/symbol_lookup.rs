use anyhow::Result;
use serde_json::{Value, json};

use rmu_core::{Engine, MigrationMode, PrivacyMode, sanitize_value_for_privacy};

use crate::ServerState;
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_usize_with_min, parse_required_non_empty_string,
    reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{ensure_query_index_ready, parse_optional_migration_mode, parse_optional_privacy_mode};

pub(super) fn symbol_lookup(args: &Value, state: &mut ServerState) -> Result<Value> {
    run_symbol_lookup(args, state, false)
}

pub(super) fn symbol_lookup_v2(args: &Value, state: &mut ServerState) -> Result<Value> {
    run_symbol_lookup(args, state, true)
}

fn run_symbol_lookup(args: &Value, state: &mut ServerState, wrap_hits: bool) -> Result<Value> {
    reject_unknown_fields(
        args,
        "symbol_lookup",
        &[
            "name",
            "limit",
            "auto_index",
            "privacy_mode",
            "migration_mode",
        ],
    )?;
    let name = parse_required_non_empty_string(args, "symbol_lookup", "name")?;
    let limit = parse_optional_usize_with_min(args, "symbol_lookup", "limit", 1, 20)?;
    let auto_index = parse_optional_bool(args, "symbol_lookup", "auto_index")?.unwrap_or(false);
    let privacy_mode = parse_optional_privacy_mode(args, "symbol_lookup", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let migration_mode = parse_optional_migration_mode(args, "symbol_lookup", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);

    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )?;
    ensure_query_index_ready(&engine, auto_index)?;
    let matches = engine.symbol_lookup(&name, limit)?;
    let mut payload = serde_json::to_value(matches)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    if wrap_hits {
        payload = json!({ "hits": payload });
    }
    tool_result(payload)
}
