use serde_json::json;

use super::RuntimeConstraintCase;

pub(super) fn cases() -> Vec<RuntimeConstraintCase> {
    vec![
        (
            "search_candidates",
            json!({"query": "q", "limit": 0}),
            "`limit` >= 1",
        ),
        (
            "semantic_search",
            json!({"query": "q", "limit": 0}),
            "`limit` >= 1",
        ),
        (
            "build_context_under_budget",
            json!({"query": "q", "limit": 0}),
            "`limit` >= 1",
        ),
        (
            "build_context_under_budget",
            json!({"query": "q", "max_chars": 255}),
            "`max_chars` >= 256",
        ),
        (
            "build_context_under_budget",
            json!({"query": "q", "max_tokens": 63}),
            "`max_tokens` >= 64",
        ),
        (
            "query_report",
            json!({"query": "q", "limit": 0}),
            "`limit` >= 1",
        ),
        (
            "symbol_lookup",
            json!({"name": "q", "limit": 0}),
            "`limit` >= 1",
        ),
        (
            "symbol_references",
            json!({"name": "q", "limit": 0}),
            "`limit` >= 1",
        ),
        (
            "related_files",
            json!({"path": "src/main.rs", "limit": 0}),
            "`limit` >= 1",
        ),
        (
            "query_report",
            json!({"query": "q", "max_chars": 255}),
            "`max_chars` >= 256",
        ),
        (
            "query_report",
            json!({"query": "q", "max_tokens": 63}),
            "`max_tokens` >= 64",
        ),
    ]
}
