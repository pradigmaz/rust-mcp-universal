use super::*;

pub(crate) fn run_context(engine: &Engine, json: bool, args: ContextArgs) -> Result<()> {
    let ContextArgs {
        query,
        limit,
        semantic,
        auto_index,
        semantic_fail_mode,
        privacy_mode,
        vector_layer_enabled,
        rollout_phase,
        max_chars,
        max_tokens,
    } = args;
    let limit = require_min("limit", limit, 1)?;
    let max_chars = require_min("max_chars", max_chars, 256)?;
    let max_tokens = require_min("max_tokens", max_tokens, 64)?;
    ensure_query_index_ready(engine, auto_index)?;
    let rollout_decision =
        decide_semantic_rollout(semantic, vector_layer_enabled, rollout_phase, &query);
    let opts = QueryOptions {
        query,
        limit,
        detailed: false,
        semantic: rollout_decision.enabled,
        semantic_fail_mode,
        privacy_mode,
        context_mode: None,
        agent_intent_mode: None,
    };
    let ctx = engine.build_context_under_budget(&opts, max_chars, max_tokens)?;

    if json {
        let mut payload = serde_json::to_value(&ctx)?;
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        print_json(serde_json::to_string_pretty(&payload))?;
    } else {
        print_line(format!(
            "files={}, chars={}, est_tokens={}, truncated={}",
            ctx.files.len(),
            ctx.total_chars,
            ctx.estimated_tokens,
            ctx.truncated
        ));
    }

    Ok(())
}

pub(crate) fn run_context_pack(engine: &Engine, json: bool, args: ContextPackArgs) -> Result<()> {
    let ContextPackArgs {
        query,
        mode,
        limit,
        semantic,
        auto_index,
        semantic_fail_mode,
        privacy_mode,
        vector_layer_enabled,
        rollout_phase,
        max_chars,
        max_tokens,
    } = args;
    let limit = require_min("limit", limit, 1)?;
    let max_chars = require_min("max_chars", max_chars, 256)?;
    let max_tokens = require_min("max_tokens", max_tokens, 64)?;
    ensure_context_pack_index_ready(engine, mode, auto_index)?;
    let rollout_decision =
        decide_semantic_rollout(semantic, vector_layer_enabled, rollout_phase, &query);
    let pack = engine.build_context_pack(
        &QueryOptions {
            query,
            limit,
            detailed: false,
            semantic: rollout_decision.enabled,
            semantic_fail_mode,
            privacy_mode,
            context_mode: Some(mode),
            agent_intent_mode: None,
        },
        mode,
        max_chars,
        max_tokens,
    )?;

    if json {
        let mut payload = serde_json::to_value(&pack)?;
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        print_json(serde_json::to_string_pretty(&payload))?;
    } else {
        print_line(format!(
            "mode={}, files={}, chars={}, est_tokens={}, truncated={}",
            pack.mode.as_str(),
            pack.context.files.len(),
            pack.context.total_chars,
            pack.context.estimated_tokens,
            pack.context.truncated
        ));
    }

    Ok(())
}
