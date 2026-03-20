use anyhow::Result;
use serde_json::{Value, json};

use rmu_core::{
    Engine, MigrationMode, PrivacyMode, QueryOptions, RolloutPhase, SemanticFailMode,
    decide_semantic_rollout, sanitize_value_for_privacy,
};

use crate::ServerState;
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_usize_with_min, parse_required_non_empty_string,
    reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{
    ensure_query_index_ready, parse_optional_migration_mode, parse_optional_privacy_mode,
    parse_optional_rollout_phase, parse_optional_semantic_fail_mode,
};

pub(super) fn semantic_search(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "semantic_search",
        &[
            "query",
            "limit",
            "auto_index",
            "semantic_fail_mode",
            "privacy_mode",
            "vector_layer_enabled",
            "rollout_phase",
            "migration_mode",
        ],
    )?;
    let query = parse_required_non_empty_string(args, "semantic_search", "query")?;
    let limit = parse_optional_usize_with_min(args, "semantic_search", "limit", 1, 20)?;
    let auto_index = parse_optional_bool(args, "semantic_search", "auto_index")?.unwrap_or(false);
    let semantic_fail_mode =
        parse_optional_semantic_fail_mode(args, "semantic_search", "semantic_fail_mode")?
            .unwrap_or(SemanticFailMode::FailOpen);
    let privacy_mode = parse_optional_privacy_mode(args, "semantic_search", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let vector_layer_enabled =
        parse_optional_bool(args, "semantic_search", "vector_layer_enabled")?.unwrap_or(true);
    let rollout_phase = parse_optional_rollout_phase(args, "semantic_search", "rollout_phase")?
        .unwrap_or(RolloutPhase::Full100);
    let migration_mode = parse_optional_migration_mode(args, "semantic_search", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);
    let semantic_effective =
        decide_semantic_rollout(true, vector_layer_enabled, rollout_phase, &query).enabled;

    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )?;
    ensure_query_index_ready(&engine, auto_index)?;
    let hits = engine.search(&QueryOptions {
        query,
        limit,
        detailed: false,
        semantic: semantic_effective,
        semantic_fail_mode,
        privacy_mode,
        context_mode: None,
    })?;
    let mut payload = json!({"hits": hits});
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}
