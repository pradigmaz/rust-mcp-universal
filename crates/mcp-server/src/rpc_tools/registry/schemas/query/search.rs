use serde_json::Value;

use super::super::common::{
    boolean_schema, enum_schema, integer_range_schema, integer_schema, migration_mode_schema,
    privacy_mode_schema, rollout_phase_schema, string_schema,
};
use super::bootstrap::agent_intent_mode_schema;
use crate::rpc_tools::registry::helpers::json_schema_object;

pub(crate) fn query_schema(include_semantic_flag: bool) -> Value {
    let mut fields = vec![
        (
            "query",
            string_schema("Natural-language query to search for.", Some(1)),
        ),
        (
            "limit",
            integer_schema(
                "Maximum number of candidates to return. Defaults to 3 to keep MCP output compact.",
                Some(1),
            ),
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
            common_query_fields(
                "Natural-language query to search for.",
                "Enable semantic reranking for this request.",
            )
            .as_slice(),
            &budget_fields(),
        ]
        .concat(),
        &["query"],
    )
}

pub(crate) fn report_query_schema() -> Value {
    json_schema_object(
        &[
            &[
                (
                    "query",
                    string_schema("Natural-language query to search for.", Some(1)),
                ),
                (
                    "mode",
                    agent_intent_mode_schema(
                        "Optional agent-facing intent mode. When omitted, RMU resolves one heuristically.",
                    ),
                ),
                (
                    "limit",
                    integer_schema(
                        "Maximum number of candidates to return. Defaults to 3 to keep MCP output compact.",
                        Some(1),
                    ),
                ),
                (
                    "details",
                    boolean_schema(
                        "Include investigation summary and timing diagnostics. Defaults to false.",
                    ),
                ),
            ][..],
            common_options("Enable semantic reranking for this request.").as_slice(),
            &budget_fields(),
        ]
        .concat(),
        &["query"],
    )
}

pub(crate) fn context_pack_schema() -> Value {
    json_schema_object(
        &[
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
                    integer_schema(
                        "Maximum number of candidates to consider. Defaults to 3 to keep MCP output compact.",
                        Some(1),
                    ),
                ),
            ][..],
            common_options("Enable semantic reranking for candidate selection.").as_slice(),
            &budget_fields(),
        ]
        .concat(),
        &["query", "mode"],
    )
}

fn common_query_fields(
    query_description: &'static str,
    semantic_description: &'static str,
) -> Vec<(&'static str, Value)> {
    let mut fields = vec![
        ("query", string_schema(query_description, Some(1))),
        (
            "limit",
            integer_schema(
                "Maximum number of candidates to return. Defaults to 3 to keep MCP output compact.",
                Some(1),
            ),
        ),
    ];
    fields.extend(common_options(semantic_description));
    fields
}

fn common_options(semantic_description: &'static str) -> Vec<(&'static str, Value)> {
    vec![
        ("semantic", boolean_schema(semantic_description)),
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
    ]
}

fn budget_fields() -> Vec<(&'static str, Value)> {
    vec![
        (
            "max_chars",
            integer_range_schema(
                "Maximum number of characters allowed in the assembled context. Defaults to 4000.",
                Some(256),
                Some(120_000),
            ),
        ),
        (
            "max_tokens",
            integer_range_schema(
                "Maximum number of tokens allowed in the assembled context. Defaults to 1000.",
                Some(64),
                Some(30_000),
            ),
        ),
    ]
}
