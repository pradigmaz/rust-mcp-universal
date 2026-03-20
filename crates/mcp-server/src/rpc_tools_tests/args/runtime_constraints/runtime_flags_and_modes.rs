use serde_json::json;

use super::RuntimeConstraintCase;

pub(super) fn cases() -> Vec<RuntimeConstraintCase> {
    vec![
        (
            "search_candidates",
            json!({"query": "q", "semantic": "true"}),
            "boolean `semantic`",
        ),
        (
            "symbol_lookup",
            json!({"name": "q", "auto_index": "true"}),
            "boolean `auto_index`",
        ),
        (
            "related_files",
            json!({"path": "src/main.rs", "auto_index": 1}),
            "boolean `auto_index`",
        ),
        (
            "search_candidates",
            json!({"query": "q", "auto_index": "true"}),
            "boolean `auto_index`",
        ),
        (
            "build_context_under_budget",
            json!({"query": "q", "semantic": 1}),
            "boolean `semantic`",
        ),
        (
            "build_context_under_budget",
            json!({"query": "q", "auto_index": 1}),
            "boolean `auto_index`",
        ),
        (
            "query_report",
            json!({"query": "q", "semantic": null}),
            "boolean `semantic`",
        ),
        (
            "query_report",
            json!({"query": "q", "auto_index": null}),
            "boolean `auto_index`",
        ),
        (
            "search_candidates",
            json!({"query": "q", "semantic_fail_mode": "broken"}),
            "`semantic_fail_mode` must be one of: fail_open, fail_closed",
        ),
        (
            "search_candidates",
            json!({"query": "q", "privacy_mode": "private"}),
            "`privacy_mode` must be one of: off, mask, hash",
        ),
        (
            "search_candidates",
            json!({"query": "q", "rollout_phase": "canary"}),
            "`rollout_phase` must be one of: shadow, canary_5, canary_25, full_100",
        ),
        (
            "query_report",
            json!({"query": "q", "migration_mode": "manual"}),
            "`migration_mode` must be one of: auto, off",
        ),
        (
            "semantic_search",
            json!({"query": "q", "semantic_fail_mode": true}),
            "string `semantic_fail_mode`",
        ),
        (
            "query_report",
            json!({"query": "q", "privacy_mode": true}),
            "string `privacy_mode`",
        ),
        (
            "symbol_references",
            json!({"name": "q", "privacy_mode": "private"}),
            "`privacy_mode` must be one of: off, mask, hash",
        ),
        (
            "related_files",
            json!({"path": "src/main.rs", "migration_mode": "manual"}),
            "`migration_mode` must be one of: auto, off",
        ),
        (
            "db_maintenance",
            json!({"privacy_mode": "private"}),
            "`privacy_mode` must be one of: off, mask, hash",
        ),
        (
            "db_maintenance",
            json!({"migration_mode": "strict"}),
            "`migration_mode` must be one of: auto, off",
        ),
        ("db_maintenance", json!({"stats": "yes"}), "boolean `stats`"),
        ("db_maintenance", json!({"prune": "yes"}), "boolean `prune`"),
        (
            "query_report",
            json!({"query": "q", "semantic_fail_mode": ""}),
            "non-empty `semantic_fail_mode`",
        ),
        (
            "agent_bootstrap",
            json!({"semantic_fail_mode": "fast"}),
            "`semantic_fail_mode` must be one of: fail_open, fail_closed",
        ),
    ]
}
