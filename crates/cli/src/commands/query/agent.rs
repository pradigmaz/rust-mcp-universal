use super::*;

pub(crate) fn run_agent(engine: &Engine, json: bool, args: AgentArgs) -> Result<()> {
    let AgentArgs {
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
    let normalized_query = query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let semantic_effective = normalized_query
        .map(|value| {
            decide_semantic_rollout(semantic, vector_layer_enabled, rollout_phase, value).enabled
        })
        .unwrap_or(false);
    let payload = engine.agent_bootstrap_with_auto_index_and_mode(
        normalized_query,
        limit,
        semantic_effective,
        semantic_fail_mode,
        privacy_mode,
        max_chars,
        max_tokens,
        auto_index,
    )?;

    if json {
        let mut value = serde_json::to_value(payload)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        let masked_db = sanitize_path_text(privacy_mode, &payload.brief.index_status.db_path);
        print_line(format!(
            "auto_indexed={}, files={}, symbols={}, semantic_vectors={}, semantic_model={}, db={}",
            payload.brief.auto_indexed,
            payload.brief.index_status.files,
            payload.brief.index_status.symbols,
            payload.brief.index_status.semantic_vectors,
            payload.brief.index_status.semantic_model,
            masked_db
        ));
        if let Some(bundle) = payload.query_bundle {
            print_line(format!(
                "query=\"{}\", hits={}, context_files={}, est_tokens={}",
                sanitize_query_text(privacy_mode, &bundle.query),
                bundle.hits.len(),
                bundle.context.files.len(),
                bundle.context.estimated_tokens
            ));
            for hit in bundle.hits {
                print_line(format!(
                    "[{:.2}] {} :: {}",
                    hit.score, hit.path, hit.preview
                ));
            }
        }
    }

    Ok(())
}
