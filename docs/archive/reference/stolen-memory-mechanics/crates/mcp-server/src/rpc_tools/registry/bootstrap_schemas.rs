use serde_json::{Value, json};

use super::helpers::json_schema_object;

pub(super) fn node_summary_schema() -> Value {
    json_schema_object(
        &[
            ("id", json!({"type": "string"})),
            ("slug", json!({"type": "string"})),
            ("title", json!({"type": "string"})),
            ("node_type", json!({"type": "string"})),
            ("status", json!({"type": "string"})),
            ("file_path", json!({"type": "string"})),
            ("summary", json!({"type": "string"})),
            ("updated_at", json!({"type": "string"})),
            ("normalized_status", json!({"type": "string"})),
        ],
        &[
            "id",
            "slug",
            "title",
            "node_type",
            "status",
            "file_path",
            "summary",
        ],
    )
}

pub(super) fn recent_change_schema() -> Value {
    json_schema_object(
        &[
            ("id", json!({"type": "string"})),
            ("slug", json!({"type": "string"})),
            ("title", json!({"type": "string"})),
            ("node_type", json!({"type": "string"})),
            ("status", json!({"type": "string"})),
            ("file_path", json!({"type": "string"})),
            ("change_hint", json!({"type": "string"})),
            ("updated_at", json!({"type": "string"})),
        ],
        &[
            "id",
            "slug",
            "title",
            "node_type",
            "status",
            "file_path",
            "change_hint",
        ],
    )
}

pub(super) fn decision_entry_schema() -> Value {
    json_schema_object(
        &[
            ("id", json!({"type": "string"})),
            ("slug", json!({"type": "string"})),
            ("title", json!({"type": "string"})),
            ("status", json!({"type": "string"})),
            ("summary", json!({"type": "string"})),
            ("updated_at", json!({"type": "string"})),
        ],
        &["id", "slug", "title", "status", "summary"],
    )
}

pub(super) fn hotspot_schema() -> Value {
    json_schema_object(
        &[
            ("id", json!({"type": "string"})),
            ("slug", json!({"type": "string"})),
            ("title", json!({"type": "string"})),
            ("status", json!({"type": "string"})),
            ("normalized_status", json!({"type": "string"})),
            ("summary", json!({"type": "string"})),
            ("updated_at", json!({"type": "string"})),
            (
                "blocks",
                json!({"type": "array", "items": {"type": "string"}}),
            ),
            (
                "affects",
                json!({"type": "array", "items": {"type": "string"}}),
            ),
        ],
        &[
            "id",
            "slug",
            "title",
            "status",
            "normalized_status",
            "summary",
            "blocks",
            "affects",
        ],
    )
}

pub(super) fn context_node_schema() -> Value {
    json_schema_object(
        &[
            ("id", json!({"type": "string"})),
            ("slug", json!({"type": "string"})),
            ("title", json!({"type": "string"})),
            ("node_type", json!({"type": "string"})),
            ("summary", json!({"type": "string"})),
            ("why_included", json!({"type": "string"})),
            ("updated_at", json!({"type": "string"})),
        ],
        &[
            "id",
            "slug",
            "title",
            "node_type",
            "summary",
            "why_included",
        ],
    )
}

pub(super) fn budget_schema() -> Value {
    json_schema_object(
        &[
            ("max_chars", json!({"type": "integer"})),
            ("max_tokens", json!({"type": "integer"})),
            ("used_chars", json!({"type": "integer"})),
            ("truncated", json!({"type": "boolean"})),
        ],
        &["max_chars", "max_tokens", "used_chars", "truncated"],
    )
}

pub(super) fn context_pack_output_schema() -> Value {
    let node_summary = node_summary_schema();
    let recent_change = recent_change_schema();
    let hotspot = hotspot_schema();
    json_schema_object(
        &[
            ("seed", json!({"type": "string"})),
            (
                "brief",
                json_schema_object(
                    &[
                        ("project", json!({"type": "string"})),
                        ("summary", json!({"type": "string"})),
                        (
                            "top_decisions",
                            json!({"type": "array", "items": node_summary.clone()}),
                        ),
                        (
                            "top_risks",
                            json!({"type": "array", "items": node_summary.clone()}),
                        ),
                        (
                            "recent_changes",
                            json!({"type": "array", "items": node_summary}),
                        ),
                    ],
                    &[
                        "project",
                        "summary",
                        "top_decisions",
                        "top_risks",
                        "recent_changes",
                    ],
                ),
            ),
            (
                "included_nodes",
                json!({"type": "array", "items": context_node_schema()}),
            ),
            (
                "recent_changes",
                json!({"type": "array", "items": recent_change}),
            ),
            ("risks", json!({"type": "array", "items": hotspot})),
            ("budget", budget_schema()),
        ],
        &[
            "seed",
            "brief",
            "included_nodes",
            "recent_changes",
            "risks",
            "budget",
        ],
    )
}
