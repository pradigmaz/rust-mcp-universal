use serde_json::json;

use super::RuntimeConstraintCase;

pub(super) fn cases() -> Vec<RuntimeConstraintCase> {
    vec![
        (
            "index_status",
            json!({"unexpected": 1}),
            "does not allow argument `unexpected`",
        ),
        (
            "search_candidates",
            json!({"query": "ok", "extra": 1}),
            "does not allow argument `extra`",
        ),
        (
            "symbol_lookup",
            json!({"name": "ok", "extra": 1}),
            "does not allow argument `extra`",
        ),
        (
            "related_files",
            json!({"path": "src/main.rs", "extra": 1}),
            "does not allow argument `extra`",
        ),
        (
            "rule_violations",
            json!({"extra": 1}),
            "does not allow argument `extra`",
        ),
        (
            "search_candidates",
            json!({"query": ""}),
            "non-empty `query`",
        ),
        ("symbol_lookup", json!({"name": ""}), "non-empty `name`"),
        ("related_files", json!({"path": ""}), "non-empty `path`"),
        (
            "rule_violations",
            json!({"sort_by": "wrong"}),
            "`sort_by` must be one of: violation_count, size_bytes, non_empty_lines, metric_value",
        ),
        (
            "rule_violations",
            json!({"sort_by": "path"}),
            "use `path_prefix` to filter paths",
        ),
    ]
}
