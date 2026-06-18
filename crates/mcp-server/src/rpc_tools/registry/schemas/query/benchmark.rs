use serde_json::Value;

use super::super::common::{
    boolean_schema, enum_schema, integer_range_schema, integer_schema, migration_mode_schema,
    privacy_mode_schema, rollout_phase_schema, string_schema,
};
use crate::rpc_tools::registry::helpers::json_schema_object;

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
                integer_range_schema(
                    "Maximum number of characters allowed in assembled context payloads.",
                    Some(256),
                    Some(120_000),
                ),
            ),
            (
                "max_tokens",
                integer_range_schema(
                    "Maximum number of tokens allowed in assembled context payloads.",
                    Some(64),
                    Some(30_000),
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
