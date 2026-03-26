use serde_json::Value;

use super::common::{
    boolean_schema, enum_schema, integer_schema, migration_mode_schema, privacy_mode_schema,
    rollout_phase_schema, string_schema,
};
use crate::rpc_tools::registry::helpers::json_schema_object;

pub(crate) fn query_schema(include_semantic_flag: bool) -> Value {
    let mut fields = vec![
        (
            "query",
            string_schema("Natural-language query to search for.", Some(1)),
        ),
        (
            "limit",
            integer_schema("Maximum number of candidates to return.", Some(1)),
        ),
    ];
    if include_semantic_flag {
        fields.push((
            "semantic",
            boolean_schema("Enable semantic reranking for this search."),
        ));
    }
    fields.push((
        "semantic_fail_mode",
        enum_schema(
            "How to behave if semantic search is unavailable.",
            &["fail_open", "fail_closed"],
        ),
    ));
    fields.push(("privacy_mode", privacy_mode_schema()));
    fields.push((
        "vector_layer_enabled",
        boolean_schema("Allow vector-layer retrieval when available."),
    ));
    fields.push(("rollout_phase", rollout_phase_schema()));
    fields.push(("migration_mode", migration_mode_schema()));
    fields.push((
        "auto_index",
        boolean_schema("Automatically build or refresh the index if needed."),
    ));
    json_schema_object(&fields, &["query"])
}

pub(crate) fn budget_query_schema() -> Value {
    json_schema_object(
        &[
            (
                "query",
                string_schema("Natural-language query to search for.", Some(1)),
            ),
            (
                "limit",
                integer_schema("Maximum number of candidates to return.", Some(1)),
            ),
            (
                "semantic",
                boolean_schema("Enable semantic reranking for this request."),
            ),
            (
                "auto_index",
                boolean_schema("Automatically build or refresh the index if needed."),
            ),
            (
                "semantic_fail_mode",
                enum_schema(
                    "How to behave if semantic search is unavailable.",
                    &["fail_open", "fail_closed"],
                ),
            ),
            ("privacy_mode", privacy_mode_schema()),
            (
                "vector_layer_enabled",
                boolean_schema("Allow vector-layer retrieval when available."),
            ),
            ("rollout_phase", rollout_phase_schema()),
            ("migration_mode", migration_mode_schema()),
            (
                "max_chars",
                integer_schema(
                    "Maximum number of characters allowed in the assembled context.",
                    Some(256),
                ),
            ),
            (
                "max_tokens",
                integer_schema(
                    "Maximum number of tokens allowed in the assembled context.",
                    Some(64),
                ),
            ),
        ],
        &["query"],
    )
}

pub(crate) fn context_pack_schema() -> Value {
    json_schema_object(
        &[
            (
                "query",
                string_schema("Task or question the context pack should support.", Some(1)),
            ),
            (
                "mode",
                enum_schema(
                    "Context-pack mode tuned to the current task.",
                    &["code", "design", "bugfix"],
                ),
            ),
            (
                "limit",
                integer_schema("Maximum number of candidates to consider.", Some(1)),
            ),
            (
                "semantic",
                boolean_schema("Enable semantic reranking for candidate selection."),
            ),
            (
                "auto_index",
                boolean_schema("Automatically build or refresh the index if needed."),
            ),
            (
                "semantic_fail_mode",
                enum_schema(
                    "How to behave if semantic search is unavailable.",
                    &["fail_open", "fail_closed"],
                ),
            ),
            ("privacy_mode", privacy_mode_schema()),
            (
                "vector_layer_enabled",
                boolean_schema("Allow vector-layer retrieval when available."),
            ),
            ("rollout_phase", rollout_phase_schema()),
            ("migration_mode", migration_mode_schema()),
            (
                "max_chars",
                integer_schema(
                    "Maximum number of characters allowed in the assembled context.",
                    Some(256),
                ),
            ),
            (
                "max_tokens",
                integer_schema(
                    "Maximum number of tokens allowed in the assembled context.",
                    Some(64),
                ),
            ),
        ],
        &["query", "mode"],
    )
}

pub(crate) fn investigation_schema() -> Value {
    json_schema_object(
        &[
            (
                "seed",
                string_schema("Concept seed, symbol, path, or path:line probe.", Some(1)),
            ),
            (
                "seed_kind",
                enum_schema(
                    "How RMU should interpret the seed value.",
                    &["query", "symbol", "path", "path_line"],
                ),
            ),
            (
                "limit",
                integer_schema("Maximum number of variants or snippets to return.", Some(1)),
            ),
            (
                "auto_index",
                boolean_schema("Automatically build or refresh the index if needed."),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &["seed", "seed_kind"],
    )
}

pub(crate) fn query_benchmark_schema() -> Value {
    json_schema_object(
        &[
            (
                "dataset_path",
                string_schema("Path to the benchmark dataset file.", Some(1)),
            ),
            (
                "k",
                integer_schema("Top-k cutoff used for retrieval metrics.", Some(1)),
            ),
            (
                "limit",
                integer_schema("Maximum number of candidates to keep per query.", Some(1)),
            ),
            (
                "semantic",
                boolean_schema("Enable semantic reranking during the benchmark."),
            ),
            (
                "auto_index",
                boolean_schema("Automatically build or refresh the index if needed."),
            ),
            (
                "semantic_fail_mode",
                enum_schema(
                    "How to behave if semantic search is unavailable.",
                    &["fail_open", "fail_closed"],
                ),
            ),
            ("privacy_mode", privacy_mode_schema()),
            (
                "vector_layer_enabled",
                boolean_schema("Allow vector-layer retrieval when available."),
            ),
            ("rollout_phase", rollout_phase_schema()),
            ("migration_mode", migration_mode_schema()),
            (
                "max_chars",
                integer_schema(
                    "Maximum number of characters allowed in assembled context payloads.",
                    Some(256),
                ),
            ),
            (
                "max_tokens",
                integer_schema(
                    "Maximum number of tokens allowed in assembled context payloads.",
                    Some(64),
                ),
            ),
            (
                "baseline",
                string_schema(
                    "Path to a baseline benchmark report for compare mode.",
                    Some(1),
                ),
            ),
            (
                "thresholds",
                string_schema(
                    "Path to a thresholds file used for gate enforcement.",
                    Some(1),
                ),
            ),
            (
                "runs",
                integer_schema("Number of benchmark repetitions to execute.", Some(1)),
            ),
            (
                "enforce_gates",
                boolean_schema("Fail the run when benchmark gates are violated."),
            ),
        ],
        &["dataset_path"],
    )
}
