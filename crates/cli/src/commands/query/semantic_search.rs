use super::*;

pub(crate) fn run_semantic_search(
    engine: &Engine,
    json: bool,
    args: SemanticSearchArgs,
) -> Result<()> {
    let SemanticSearchArgs {
        query,
        limit,
        auto_index,
        semantic_fail_mode,
        privacy_mode,
        vector_layer_enabled,
        rollout_phase,
    } = args;
    let limit = require_min("limit", limit, 1)?;
    ensure_query_index_ready(engine, auto_index)?;
    let rollout_decision =
        decide_semantic_rollout(true, vector_layer_enabled, rollout_phase, &query);
    let opts = QueryOptions {
        query,
        limit,
        detailed: false,
        semantic: rollout_decision.enabled,
        semantic_fail_mode,
        privacy_mode,
        context_mode: None,
    };
    let hits = engine.search(&opts)?;

    if json {
        let mut payload = serde_json::to_value(&hits)?;
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        print_json(serde_json::to_string_pretty(&payload))?;
    } else {
        for hit in hits {
            print_line(format!(
                "[{:.2}] {} :: {}",
                hit.score,
                sanitize_path_text(privacy_mode, &hit.path),
                hit.preview
            ));
        }
    }

    Ok(())
}
