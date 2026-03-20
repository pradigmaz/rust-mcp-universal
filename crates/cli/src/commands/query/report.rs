use super::*;

pub(crate) fn run_report(engine: &Engine, json: bool, args: ReportArgs) -> Result<()> {
    let ReportArgs {
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
        detailed: true,
        semantic: rollout_decision.enabled,
        semantic_fail_mode,
        privacy_mode,
        context_mode: None,
    };
    let report = engine.build_report(&opts, max_chars, max_tokens)?;

    if json {
        let mut payload = serde_json::to_value(&report)?;
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        print_json(serde_json::to_string_pretty(&payload))?;
    } else {
        print_line(format!(
            "query_id={}, confidence={:.2}, selected={}, budget_tokens={}/{}, truncated={}",
            report.query_id,
            report.confidence.overall,
            report.selected_context.len(),
            report.budget.used_estimate,
            report.budget.max_tokens,
            report.budget.hard_truncated
        ));
        for item in &report.selected_context {
            print_line(format!(
                "[{:.2}] {} (chars={})",
                item.score, item.path, item.chars
            ));
        }
        if !report.gaps.is_empty() {
            print_line(format!("gaps={}", report.gaps.join(" | ")));
        }
    }

    Ok(())
}
