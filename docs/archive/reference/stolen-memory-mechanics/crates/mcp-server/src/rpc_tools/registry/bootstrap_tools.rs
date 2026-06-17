use serde_json::{Value, json};

use super::bootstrap_schemas::{
    context_pack_output_schema, decision_entry_schema, hotspot_schema, recent_change_schema,
};
use super::helpers::{json_schema_object, tool_with_output};

pub(super) fn bootstrap_tools(auto_index: &Value, privacy_mode: &Value) -> Vec<Value> {
    let recent_change = recent_change_schema();
    let decision_entry = decision_entry_schema();
    let hotspot = hotspot_schema();

    vec![
        tool_with_output(
            "recent_changes",
            "Get recent indexed note changes for bootstrap context, excluding structural section hubs by default",
            json_schema_object(
                &[
                    (
                        "limit",
                        json!({"type": "integer", "minimum": 1, "maximum": 50}),
                    ),
                    ("auto_index", auto_index.clone()),
                    ("privacy_mode", privacy_mode.clone()),
                ],
                &[],
            ),
            json_schema_object(
                &[(
                    "changes",
                    json!({"type": "array", "items": recent_change.clone()}),
                )],
                &["changes"],
            ),
            true,
            false,
            true,
        ),
        tool_with_output(
            "decision_log",
            "List recent current/open decisions, optionally filtered by lexical topic match",
            json_schema_object(
                &[
                    ("topic", json!({"type": "string", "minLength": 1})),
                    (
                        "limit",
                        json!({"type": "integer", "minimum": 1, "maximum": 50}),
                    ),
                    ("auto_index", auto_index.clone()),
                    ("privacy_mode", privacy_mode.clone()),
                ],
                &[],
            ),
            json_schema_object(
                &[(
                    "decisions",
                    json!({"type": "array", "items": decision_entry}),
                )],
                &["decisions"],
            ),
            true,
            false,
            true,
        ),
        tool_with_output(
            "risk_hotspots",
            "List unresolved risks and constraints ranked by blocker impact",
            json_schema_object(
                &[
                    (
                        "limit",
                        json!({"type": "integer", "minimum": 1, "maximum": 50}),
                    ),
                    ("auto_index", auto_index.clone()),
                    ("privacy_mode", privacy_mode.clone()),
                ],
                &[],
            ),
            json_schema_object(
                &[
                    ("risks", json!({"type": "array", "items": hotspot.clone()})),
                    ("constraints", json!({"type": "array", "items": hotspot})),
                ],
                &["risks", "constraints"],
            ),
            true,
            false,
            true,
        ),
        tool_with_output(
            "context_pack",
            "Build compact current-state bootstrap context around one lexical seed without promoting structural section hubs by default",
            json_schema_object(
                &[
                    ("seed", json!({"type": "string", "minLength": 1})),
                    (
                        "limit",
                        json!({"type": "integer", "minimum": 1, "maximum": 12}),
                    ),
                    (
                        "max_chars",
                        json!({"type": "integer", "minimum": 1, "maximum": 12000}),
                    ),
                    (
                        "max_tokens",
                        json!({"type": "integer", "minimum": 1, "maximum": 3000}),
                    ),
                    ("auto_index", auto_index.clone()),
                    ("privacy_mode", privacy_mode.clone()),
                ],
                &["seed"],
            ),
            context_pack_output_schema(),
            true,
            false,
            true,
        ),
    ]
}
