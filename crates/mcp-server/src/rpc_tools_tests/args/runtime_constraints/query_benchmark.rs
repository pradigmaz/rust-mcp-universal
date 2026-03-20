use serde_json::json;

use super::RuntimeConstraintCase;

pub(super) fn cases() -> Vec<RuntimeConstraintCase> {
    vec![
        (
            "query_benchmark",
            json!({"dataset_path": "dataset.json", "k": 0}),
            "`k` >= 1",
        ),
        (
            "query_benchmark",
            json!({"dataset_path": "dataset.json", "limit": 0}),
            "`limit` >= 1",
        ),
        (
            "query_benchmark",
            json!({"dataset_path": "dataset.json", "max_chars": 255}),
            "`max_chars` >= 256",
        ),
        (
            "query_benchmark",
            json!({"dataset_path": "dataset.json", "max_tokens": 63}),
            "`max_tokens` >= 64",
        ),
        (
            "query_benchmark",
            json!({"dataset_path": ""}),
            "non-empty `dataset_path`",
        ),
        (
            "query_benchmark",
            json!({"dataset_path": "dataset.json", "baseline": true}),
            "string `baseline`",
        ),
        (
            "query_benchmark",
            json!({"dataset_path": "dataset.json", "baseline": ""}),
            "non-empty `baseline`",
        ),
        (
            "query_benchmark",
            json!({"dataset_path": "dataset.json", "thresholds": true}),
            "string `thresholds`",
        ),
        (
            "query_benchmark",
            json!({"dataset_path": "dataset.json", "runs": 0}),
            "`runs` >= 1",
        ),
        (
            "query_benchmark",
            json!({"dataset_path": "dataset.json", "enforce_gates": "true"}),
            "boolean `enforce_gates`",
        ),
        (
            "query_benchmark",
            json!({"dataset_path": "dataset.json", "runs": 2}),
            "compare mode requires non-empty `baseline`",
        ),
        (
            "query_benchmark",
            json!({"dataset_path": "dataset.json", "thresholds": "thresholds.json"}),
            "compare mode requires non-empty `baseline`",
        ),
        (
            "query_benchmark",
            json!({"dataset_path": "dataset.json", "baseline": "baseline.json", "enforce_gates": true}),
            "`enforce_gates` requires non-empty `thresholds`",
        ),
    ]
}
