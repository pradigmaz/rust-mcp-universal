use anyhow::Result;
use serde_json::{Value, json};

use rmu_core::{Engine, MigrationMode, PrivacyMode, sanitize_value_for_privacy};

use crate::ServerState;
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_usize_with_min, parse_required_non_empty_string,
    reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{parse_optional_migration_mode, parse_optional_privacy_mode};

pub(super) fn related_files(args: &Value, state: &mut ServerState) -> Result<Value> {
    run_related_files(args, state, false)
}

pub(super) fn related_files_v2(args: &Value, state: &mut ServerState) -> Result<Value> {
    run_related_files(args, state, true)
}

fn run_related_files(args: &Value, state: &mut ServerState, wrap_hits: bool) -> Result<Value> {
    reject_unknown_fields(
        args,
        "related_files",
        &[
            "path",
            "limit",
            "auto_index",
            "privacy_mode",
            "migration_mode",
        ],
    )?;
    let path = parse_required_non_empty_string(args, "related_files", "path")?;
    let limit = parse_optional_usize_with_min(args, "related_files", "limit", 1, 20)?;
    let auto_index = parse_optional_bool(args, "related_files", "auto_index")?.unwrap_or(false);
    let privacy_mode = parse_optional_privacy_mode(args, "related_files", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let migration_mode = parse_optional_migration_mode(args, "related_files", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);

    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )?;
    let _ = engine.ensure_mixed_index_ready_for_paths(auto_index, std::slice::from_ref(&path))?;
    let hits = engine.related_files(&path, limit)?;
    let mut payload = serde_json::to_value(hits)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    if wrap_hits {
        payload = json!({ "hits": payload });
    }
    tool_result(payload)
}
