use serde_json::{Value, json};

use super::super::common::{
    enum_schema, migration_mode_schema, privacy_mode_schema, rollout_phase_schema,
};
use crate::rpc_tools::registry::helpers::json_schema_object;

pub(crate) fn agent_bootstrap_schema() -> Value {
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

pub(crate) fn agent_intent_mode_schema(description: &str) -> Value {
    enum_schema(
        description,
        &[
            "entrypoint_map",
            "test_map",
            "review_prep",
            "api_contract_map",
            "runtime_surface",
            "refactor_surface",
        ],
    )
}

pub(crate) fn bootstrap_profile_schema() -> Value {
    enum_schema(
        "Preferred bootstrap surface depth. Overrides legacy include flags when provided.",
        &["fast", "investigation_summary", "report", "full"],
    )
}
