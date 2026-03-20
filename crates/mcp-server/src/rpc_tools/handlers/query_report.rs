use anyhow::Result;
use serde_json::Value;

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

pub(super) fn query_report(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "query_report",
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
        ],
    )?;
    let query = parse_required_non_empty_string(args, "query_report", "query")?;
    let limit = parse_optional_usize_with_min(args, "query_report", "limit", 1, 20)?;
    let semantic = parse_optional_bool(args, "query_report", "semantic")?.unwrap_or(false);
    let auto_index = parse_optional_bool(args, "query_report", "auto_index")?.unwrap_or(false);
    let semantic_fail_mode =
        parse_optional_semantic_fail_mode(args, "query_report", "semantic_fail_mode")?
            .unwrap_or(SemanticFailMode::FailOpen);
    let privacy_mode = parse_optional_privacy_mode(args, "query_report", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let vector_layer_enabled =
        parse_optional_bool(args, "query_report", "vector_layer_enabled")?.unwrap_or(true);
    let rollout_phase = parse_optional_rollout_phase(args, "query_report", "rollout_phase")?
        .unwrap_or(RolloutPhase::Full100);
    let migration_mode = parse_optional_migration_mode(args, "query_report", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);
    let max_chars = parse_optional_usize_with_min(args, "query_report", "max_chars", 256, 12_000)?;
    let max_tokens = parse_optional_usize_with_min(args, "query_report", "max_tokens", 64, 3_000)?;

    let semantic_effective =
        decide_semantic_rollout(semantic, vector_layer_enabled, rollout_phase, &query).enabled;
    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )?;
    ensure_query_index_ready(&engine, auto_index)?;
    let report = engine.build_report(
        &QueryOptions {
            query,
            limit,
            detailed: true,
            semantic: semantic_effective,
            semantic_fail_mode,
            privacy_mode,
            context_mode: None,
        },
        max_chars,
        max_tokens,
    )?;
    let mut payload = serde_json::to_value(report)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}
