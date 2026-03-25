use serde_json::{Value, json};

use super::helpers::json_schema_object;

pub(super) fn index_schema() -> Value {
    json_schema_object(
        &[
            (
                "profile",
                json!({
                    "type": "string",
                    "oneOf": [
                        {"const": "rust-monorepo"},
                        {"const": "mixed"},
                        {"const": "docs-heavy"}
                    ]
                }),
            ),
            (
                "changed_since",
                json!({
                    "type": "string"
                }),
            ),
            (
                "changed_since_commit",
                json!({
                    "type": "string",
                    "minLength": 1
                }),
            ),
            (
                "include_paths",
                json!({
                    "type": "array",
                    "items": {"type": "string"}
                }),
            ),
            (
                "exclude_paths",
                json!({
                    "type": "array",
                    "items": {"type": "string"}
                }),
            ),
            ("reindex", json!({"type": "boolean"})),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(super) fn install_ignore_rules_schema() -> Value {
    json_schema_object(
        &[(
            "target",
            json!({
                "type": "string",
                "oneOf": [
                    {"const": "git-info-exclude"},
                    {"const": "root-gitignore"}
                ]
            }),
        )],
        &[],
    )
}

pub(super) fn scope_preview_schema() -> Value {
    json_schema_object(
        &[
            (
                "profile",
                json!({
                    "type": "string",
                    "oneOf": [
                        {"const": "rust-monorepo"},
                        {"const": "mixed"},
                        {"const": "docs-heavy"}
                    ]
                }),
            ),
            (
                "changed_since",
                json!({
                    "type": "string"
                }),
            ),
            (
                "changed_since_commit",
                json!({
                    "type": "string",
                    "minLength": 1
                }),
            ),
            (
                "include_paths",
                json!({
                    "type": "array",
                    "items": {"type": "string"}
                }),
            ),
            (
                "exclude_paths",
                json!({
                    "type": "array",
                    "items": {"type": "string"}
                }),
            ),
            ("reindex", json!({"type": "boolean"})),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(super) fn rule_violations_schema() -> Value {
    json_schema_object(
        &[
            ("limit", json!({"type": "integer", "minimum": 1})),
            ("path_prefix", json!({"type": "string", "minLength": 1})),
            ("language", json!({"type": "string", "minLength": 1})),
            (
                "rule_ids",
                json!({
                    "type": "array",
                    "items": {"type": "string"}
                }),
            ),
            (
                "metric_ids",
                json!({
                    "type": "array",
                    "items": {"type": "string"}
                }),
            ),
            ("sort_metric_id", json!({"type": "string", "minLength": 1})),
            (
                "sort_by",
                json!({
                    "type": "string",
                    "oneOf": [
                        {"const": "violation_count"},
                        {"const": "size_bytes"},
                        {"const": "non_empty_lines"},
                        {"const": "metric_value"}
                    ]
                }),
            ),
            ("auto_index", json!({"type": "boolean"})),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(super) fn quality_hotspots_schema() -> Value {
    json_schema_object(
        &[
            (
                "aggregation",
                json!({
                    "type": "string",
                    "oneOf": [
                        {"const": "file"},
                        {"const": "directory"},
                        {"const": "module"}
                    ]
                }),
            ),
            ("limit", json!({"type": "integer", "minimum": 1})),
            ("path_prefix", json!({"type": "string", "minLength": 1})),
            ("language", json!({"type": "string", "minLength": 1})),
            (
                "rule_ids",
                json!({
                    "type": "array",
                    "items": {"type": "string"}
                }),
            ),
            (
                "sort_by",
                json!({
                    "type": "string",
                    "oneOf": [
                        {"const": "hotspot_score"},
                        {"const": "risk_score_delta"},
                        {"const": "new_violations"}
                    ]
                }),
            ),
            ("auto_index", json!({"type": "boolean"})),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(super) fn query_schema(include_semantic_flag: bool) -> Value {
    let mut fields = vec![
        ("query", json!({"type": "string", "minLength": 1})),
        ("limit", json!({"type": "integer", "minimum": 1})),
    ];
    if include_semantic_flag {
        fields.push(("semantic", json!({"type": "boolean"})));
    }
    fields.push((
        "semantic_fail_mode",
        json!({
            "type": "string",
            "oneOf": [
                {"const": "fail_open"},
                {"const": "fail_closed"}
            ]
        }),
    ));
    fields.push(("privacy_mode", privacy_mode_schema()));
    fields.push(("vector_layer_enabled", json!({"type": "boolean"})));
    fields.push(("rollout_phase", rollout_phase_schema()));
    fields.push(("migration_mode", migration_mode_schema()));
    fields.push(("auto_index", json!({"type": "boolean"})));
    json_schema_object(&fields, &["query"])
}

pub(super) fn budget_query_schema() -> Value {
    json_schema_object(
        &[
            ("query", json!({"type": "string", "minLength": 1})),
            ("limit", json!({"type": "integer", "minimum": 1})),
            ("semantic", json!({"type": "boolean"})),
            ("auto_index", json!({"type": "boolean"})),
            (
                "semantic_fail_mode",
                json!({
                    "type": "string",
                    "oneOf": [
                        {"const": "fail_open"},
                        {"const": "fail_closed"}
                    ]
                }),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("vector_layer_enabled", json!({"type": "boolean"})),
            ("rollout_phase", rollout_phase_schema()),
            ("migration_mode", migration_mode_schema()),
            ("max_chars", json!({"type": "integer", "minimum": 256})),
            ("max_tokens", json!({"type": "integer", "minimum": 64})),
        ],
        &["query"],
    )
}

pub(super) fn context_pack_schema() -> Value {
    json_schema_object(
        &[
            ("query", json!({"type": "string", "minLength": 1})),
            (
                "mode",
                json!({
                    "type": "string",
                    "oneOf": [
                        {"const": "code"},
                        {"const": "design"},
                        {"const": "bugfix"}
                    ]
                }),
            ),
            ("limit", json!({"type": "integer", "minimum": 1})),
            ("semantic", json!({"type": "boolean"})),
            ("auto_index", json!({"type": "boolean"})),
            (
                "semantic_fail_mode",
                json!({
                    "type": "string",
                    "oneOf": [
                        {"const": "fail_open"},
                        {"const": "fail_closed"}
                    ]
                }),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("vector_layer_enabled", json!({"type": "boolean"})),
            ("rollout_phase", rollout_phase_schema()),
            ("migration_mode", migration_mode_schema()),
            ("max_chars", json!({"type": "integer", "minimum": 256})),
            ("max_tokens", json!({"type": "integer", "minimum": 64})),
        ],
        &["query", "mode"],
    )
}

pub(super) fn investigation_schema() -> Value {
    json_schema_object(
        &[
            ("seed", json!({"type": "string", "minLength": 1})),
            (
                "seed_kind",
                json!({
                    "type": "string",
                    "oneOf": [
                        {"const": "query"},
                        {"const": "symbol"},
                        {"const": "path"},
                        {"const": "path_line"}
                    ]
                }),
            ),
            ("limit", json!({"type": "integer", "minimum": 1})),
            ("auto_index", json!({"type": "boolean"})),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &["seed", "seed_kind"],
    )
}

pub(super) fn preflight_schema() -> Value {
    json_schema_object(
        &[
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(super) fn navigation_schema(arg_name: &str) -> Value {
    json_schema_object(
        &[
            (arg_name, json!({"type": "string", "minLength": 1})),
            ("limit", json!({"type": "integer", "minimum": 1})),
            ("auto_index", json!({"type": "boolean"})),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[arg_name],
    )
}

pub(super) fn call_path_schema() -> Value {
    json_schema_object(
        &[
            ("from", json!({"type": "string", "minLength": 1})),
            ("to", json!({"type": "string", "minLength": 1})),
            ("max_hops", json!({"type": "integer", "minimum": 1})),
            ("auto_index", json!({"type": "boolean"})),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &["from", "to"],
    )
}

pub(super) fn query_benchmark_schema() -> Value {
    json_schema_object(
        &[
            ("dataset_path", json!({"type": "string", "minLength": 1})),
            ("k", json!({"type": "integer", "minimum": 1})),
            ("limit", json!({"type": "integer", "minimum": 1})),
            ("semantic", json!({"type": "boolean"})),
            ("auto_index", json!({"type": "boolean"})),
            (
                "semantic_fail_mode",
                json!({
                    "type": "string",
                    "oneOf": [
                        {"const": "fail_open"},
                        {"const": "fail_closed"}
                    ]
                }),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("vector_layer_enabled", json!({"type": "boolean"})),
            ("rollout_phase", rollout_phase_schema()),
            ("migration_mode", migration_mode_schema()),
            ("max_chars", json!({"type": "integer", "minimum": 256})),
            ("max_tokens", json!({"type": "integer", "minimum": 64})),
            ("baseline", json!({"type": "string", "minLength": 1})),
            ("thresholds", json!({"type": "string", "minLength": 1})),
            ("runs", json!({"type": "integer", "minimum": 1})),
            ("enforce_gates", json!({"type": "boolean"})),
        ],
        &["dataset_path"],
    )
}

pub(super) fn db_maintenance_schema() -> Value {
    json_schema_object(
        &[
            ("integrity_check", json!({"type": "boolean"})),
            ("checkpoint", json!({"type": "boolean"})),
            ("vacuum", json!({"type": "boolean"})),
            ("analyze", json!({"type": "boolean"})),
            ("stats", json!({"type": "boolean"})),
            ("prune", json!({"type": "boolean"})),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(super) fn privacy_mode_schema() -> Value {
    json!({
        "type": "string",
        "oneOf": [
            {"const": "off"},
            {"const": "mask"},
            {"const": "hash"}
        ]
    })
}

pub(super) fn rollout_phase_schema() -> Value {
    json!({
        "type": "string",
        "oneOf": [
            {"const": "shadow"},
            {"const": "canary_5"},
            {"const": "canary_25"},
            {"const": "full_100"}
        ]
    })
}

pub(super) fn migration_mode_schema() -> Value {
    json!({
        "type": "string",
        "oneOf": [
            {"const": "auto"},
            {"const": "off"}
        ]
    })
}
