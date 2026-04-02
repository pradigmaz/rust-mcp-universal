use anyhow::Result;
use serde_json::Value;

use rmu_core::{
    Engine, IndexProfile, IndexingOptions, MigrationMode, PrivacyMode, QueryOptions, RolloutPhase,
    SemanticFailMode, decide_semantic_rollout, sanitize_value_for_privacy,
};

use crate::ServerState;
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_usize_with_min, parse_required_non_empty_string,
    reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{
    ensure_query_index_ready, parse_optional_context_mode, parse_optional_migration_mode,
    parse_optional_privacy_mode, parse_optional_rollout_phase, parse_optional_semantic_fail_mode,
};

pub(super) fn context_pack(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "context_pack",
        &[
            "query",
            "mode",
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
    let query = parse_required_non_empty_string(args, "context_pack", "query")?;
    let mode = parse_optional_context_mode(args, "context_pack", "mode")?.ok_or_else(|| {
        crate::rpc_tools::errors::invalid_params_error("context_pack requires `mode`")
    })?;
    let limit = parse_optional_usize_with_min(args, "context_pack", "limit", 1, 20)?;
    let semantic = parse_optional_bool(args, "context_pack", "semantic")?.unwrap_or(false);
    let auto_index = parse_optional_bool(args, "context_pack", "auto_index")?.unwrap_or(false);
    let semantic_fail_mode =
        parse_optional_semantic_fail_mode(args, "context_pack", "semantic_fail_mode")?
            .unwrap_or(SemanticFailMode::FailOpen);
    let privacy_mode = parse_optional_privacy_mode(args, "context_pack", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let vector_layer_enabled =
        parse_optional_bool(args, "context_pack", "vector_layer_enabled")?.unwrap_or(true);
    let rollout_phase = parse_optional_rollout_phase(args, "context_pack", "rollout_phase")?
        .unwrap_or(RolloutPhase::Full100);
    let migration_mode = parse_optional_migration_mode(args, "context_pack", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);
    let max_chars = parse_optional_usize_with_min(args, "context_pack", "max_chars", 256, 12_000)?;
    let max_tokens = parse_optional_usize_with_min(args, "context_pack", "max_tokens", 64, 3_000)?;

    let semantic_effective =
        decide_semantic_rollout(semantic, vector_layer_enabled, rollout_phase, &query).enabled;
    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )?;
    if auto_index
        && matches!(mode, rmu_core::ContextMode::Design)
        && engine.index_status()?.files == 0
    {
        let _ = engine.index_path_with_options(&IndexingOptions {
            profile: Some(IndexProfile::DocsHeavy),
            reindex: true,
            ..IndexingOptions::default()
        })?;
    } else {
        ensure_query_index_ready(&engine, auto_index)?;
    }
    let result = engine.build_context_pack(
        &QueryOptions {
            query,
            limit,
            detailed: false,
            semantic: semantic_effective,
            semantic_fail_mode,
            privacy_mode,
            context_mode: Some(mode),
            agent_intent_mode: None,
        },
        mode,
        max_chars,
        max_tokens,
    )?;
    let mut payload = serde_json::to_value(result)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}
