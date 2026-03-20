use serde_json::json;

use super::RuntimeConstraintCase;

pub(super) fn cases() -> Vec<RuntimeConstraintCase> {
    vec![
        (
            "semantic_index",
            json!({"profile": true}),
            "`profile` must be string",
        ),
        (
            "semantic_index",
            json!({"profile": "unknown"}),
            "`profile` must be one of: rust-monorepo, mixed, docs-heavy",
        ),
        (
            "semantic_index",
            json!({"changed_since": true}),
            "`changed_since` must be string",
        ),
        (
            "semantic_index",
            json!({"changed_since": "2026-03-15T10:00:00"}),
            "`changed_since` must be RFC3339 timestamp with timezone",
        ),
        (
            "semantic_index",
            json!({"changed_since_commit": true}),
            "`changed_since_commit` must be string",
        ),
        (
            "semantic_index",
            json!({"changed_since_commit": "   "}),
            "`changed_since_commit` must be non-empty",
        ),
        (
            "semantic_index",
            json!({
                "changed_since": "2026-03-15T10:00:00Z",
                "changed_since_commit": "HEAD"
            }),
            "`changed_since` and `changed_since_commit` are mutually exclusive",
        ),
        (
            "semantic_index",
            json!({"include_paths": "src"}),
            "array `include_paths`",
        ),
        (
            "semantic_index",
            json!({"include_paths": [1]}),
            "string items in `include_paths`",
        ),
        (
            "semantic_index",
            json!({"exclude_paths": "target"}),
            "array `exclude_paths`",
        ),
        (
            "semantic_index",
            json!({"reindex": "yes"}),
            "boolean `reindex`",
        ),
        (
            "semantic_index",
            json!({"unknown": true}),
            "does not allow argument `unknown`",
        ),
        (
            "scope_preview",
            json!({"profile": true}),
            "`profile` must be string",
        ),
        (
            "scope_preview",
            json!({"changed_since": "2026-03-15T10:00:00"}),
            "`changed_since` must be RFC3339 timestamp with timezone",
        ),
        (
            "scope_preview",
            json!({"changed_since_commit": "   "}),
            "`changed_since_commit` must be non-empty",
        ),
        (
            "scope_preview",
            json!({
                "changed_since": "2026-03-15T10:00:00Z",
                "changed_since_commit": "HEAD"
            }),
            "`changed_since` and `changed_since_commit` are mutually exclusive",
        ),
        (
            "scope_preview",
            json!({"include_paths": "src"}),
            "array `include_paths`",
        ),
        (
            "scope_preview",
            json!({"privacy_mode": "secret"}),
            "`privacy_mode` must be one of: off, mask, hash",
        ),
        ("delete_index", json!({}), "confirm=true"),
        (
            "delete_index",
            json!({"confirm": "yes"}),
            "boolean `confirm`",
        ),
    ]
}
