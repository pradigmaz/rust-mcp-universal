use serde_json::json;

use super::RuntimeConstraintCase;

pub(super) fn cases() -> Vec<RuntimeConstraintCase> {
    vec![
        ("agent_bootstrap", json!({"limit": 0}), "`limit` >= 1"),
        (
            "agent_bootstrap",
            json!({"max_chars": 255}),
            "`max_chars` >= 256",
        ),
        (
            "agent_bootstrap",
            json!({"max_tokens": 63}),
            "`max_tokens` >= 64",
        ),
        (
            "agent_bootstrap",
            json!({"auto_index": "false"}),
            "boolean `auto_index`",
        ),
        ("agent_bootstrap", json!({"query": true}), "string `query`"),
        ("agent_bootstrap", json!({"query": ""}), "non-empty `query`"),
        (
            "set_project_path",
            json!({"project_path": ""}),
            "requires non-empty `project_path`",
        ),
        (
            "set_project_path",
            json!({"project_path": ".", "extra": true}),
            "does not allow argument `extra`",
        ),
    ]
}
