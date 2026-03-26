use anyhow::Result;
use serde_json::Value;

use rmu_core::{
    AgentBootstrapIncludeOptions, Engine, MigrationMode, PrivacyMode, RolloutPhase,
    SemanticFailMode, decide_semantic_rollout, sanitize_value_for_privacy,
};

use crate::ServerState;
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_non_empty_string, parse_optional_usize_with_min,
    reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{
    ensure_query_index_ready, parse_optional_migration_mode, parse_optional_privacy_mode,
    parse_optional_rollout_phase, parse_optional_semantic_fail_mode,
};

pub(super) fn agent_bootstrap(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "agent_bootstrap",
        &[
            "query",
            "limit",
            "semantic",
            "auto_index",
            "semantic_fail_mode",
            "privacy_mode",
            "vector_layer_enabled",
            "rollout_phase",
            "migration_mode",
            "max_chars",
            "max_tokens",
            "include_report",
            "include_investigation_summary",
        ],
    )?;
    let query = parse_optional_non_empty_string(args, "agent_bootstrap", "query")?;
    let limit = parse_optional_usize_with_min(args, "agent_bootstrap", "limit", 1, 20)?;
    let semantic = parse_optional_bool(args, "agent_bootstrap", "semantic")?.unwrap_or(false);
    let auto_index = parse_optional_bool(args, "agent_bootstrap", "auto_index")?.unwrap_or(false);
    let semantic_fail_mode =
        parse_optional_semantic_fail_mode(args, "agent_bootstrap", "semantic_fail_mode")?
            .unwrap_or(SemanticFailMode::FailOpen);
    let privacy_mode = parse_optional_privacy_mode(args, "agent_bootstrap", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let vector_layer_enabled =
        parse_optional_bool(args, "agent_bootstrap", "vector_layer_enabled")?.unwrap_or(true);
    let rollout_phase = parse_optional_rollout_phase(args, "agent_bootstrap", "rollout_phase")?
        .unwrap_or(RolloutPhase::Full100);
    let migration_mode = parse_optional_migration_mode(args, "agent_bootstrap", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);
    let max_chars =
        parse_optional_usize_with_min(args, "agent_bootstrap", "max_chars", 256, 12_000)?;
    let max_tokens =
        parse_optional_usize_with_min(args, "agent_bootstrap", "max_tokens", 64, 3_000)?;
    let include_report =
        parse_optional_bool(args, "agent_bootstrap", "include_report")?.unwrap_or(false);
    let include_investigation_summary = parse_optional_bool(
        args,
        "agent_bootstrap",
        "include_investigation_summary",
    )?
    .unwrap_or(false);

    let semantic_effective = query
        .as_deref()
        .map(|value| decide_semantic_rollout(semantic, vector_layer_enabled, rollout_phase, value))
        .map(|decision| decision.enabled)
        .unwrap_or(false);
    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )?;
    if auto_index {
        ensure_query_index_ready(&engine, true)?;
    }
    let payload = engine.agent_bootstrap_with_auto_index_and_options(
        query.as_deref(),
        limit,
        semantic_effective,
        semantic_fail_mode,
        privacy_mode,
        max_chars,
        max_tokens,
        false,
        AgentBootstrapIncludeOptions {
            include_report,
            include_investigation_summary,
        },
    )?;
    let mut payload = serde_json::to_value(payload)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}
