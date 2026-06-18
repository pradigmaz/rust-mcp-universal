use serde_json::{Value, json};

use super::super::helpers::{json_schema_object, tool};
use super::super::schemas::{
    agent_intent_mode_schema, bootstrap_profile_schema, budget_query_schema, context_pack_schema,
    migration_mode_schema, privacy_mode_schema, query_schema, report_query_schema,
    rollout_phase_schema,
};

pub(super) fn tools() -> Vec<Value> {
    vec![
        tool(
            "agent_bootstrap",
            "Primary explore path for agents: one-shot bootstrap payload before narrower follow-up tools",
            agent_bootstrap_schema(),
        ),
        tool(
            "search_candidates",
            "Search indexed candidates by query with canonical privacy_mode values `off`, `mask`, or `hash`",
            query_schema(true),
        ),
        tool(
            "semantic_search",
            "Search indexed candidates with semantic rerank enabled",
            query_schema(false),
        ),
        tool(
            "build_context_under_budget",
            "Build context constrained by char/token budgets",
            budget_query_schema(),
        ),
        tool(
            "context_pack",
            "Build mode-aware context pack for code, design, or bugfix work",
            context_pack_schema(),
        ),
        tool(
            "query_report",
            "Generate retrieval report for a query",
            report_query_schema(),
        ),
    ]
}

fn agent_bootstrap_schema() -> Value {
    json_schema_object(
        &[
            (
                "query",
                json!({
                    "type": "string",
                    "minLength": 1,
                    "description": "Task or question the bootstrap payload should support."
                }),
            ),
            (
                "limit",
                json!({
                    "type": "integer",
                    "minimum": 1,
                    "description": "Maximum number of candidates to consider. Defaults to 3 to keep MCP output compact."
                }),
            ),
            (
                "semantic",
                json!({
                    "type": "boolean",
                    "description": "Enable semantic reranking for candidate selection."
                }),
            ),
            (
                "auto_index",
                json!({
                    "type": "boolean",
                    "description": "Automatically build or refresh the index if needed."
                }),
            ),
            (
                "semantic_fail_mode",
                json!({
                    "type": "string",
                    "description": "How to behave if semantic search is unavailable.",
                    "oneOf": [
                        {"const": "fail_open"},
                        {"const": "fail_closed"}
                    ]
                }),
            ),
            ("privacy_mode", privacy_mode_schema()),
            (
                "vector_layer_enabled",
                json!({
                    "type": "boolean",
                    "description": "Allow vector-layer retrieval when available."
                }),
            ),
            ("rollout_phase", rollout_phase_schema()),
            ("migration_mode", migration_mode_schema()),
            (
                "max_chars",
                json!({
                    "type": "integer",
                    "minimum": 256,
                    "maximum": 120000,
                    "description": "Maximum number of characters allowed in the assembled payload. Defaults to 4000."
                }),
            ),
            (
                "max_tokens",
                json!({
                    "type": "integer",
                    "minimum": 64,
                    "maximum": 30000,
                    "description": "Maximum number of tokens allowed in the assembled payload. Defaults to 1000."
                }),
            ),
            (
                "mode",
                agent_intent_mode_schema(
                    "Optional agent-facing intent mode. When omitted, RMU resolves one heuristically.",
                ),
            ),
            ("profile", bootstrap_profile_schema()),
            (
                "include_report",
                json!({
                    "type": "boolean",
                    "description": "Include the expensive query report payload in the bootstrap response."
                }),
            ),
            (
                "include_investigation_summary",
                json!({
                    "type": "boolean",
                    "description": "Include the expensive investigation summary payload in the bootstrap response."
                }),
            ),
        ],
        &[],
    )
}
