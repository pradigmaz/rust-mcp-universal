use serde_json::{Value, json};

pub(super) fn string_schema(description: &str, min_length: Option<u64>) -> Value {
    let mut schema = json!({
        "type": "string",
        "description": description,
    });
    if let Some(min_length) = min_length {
        schema["minLength"] = json!(min_length);
    }
    schema
}

pub(super) fn integer_schema(description: &str, minimum: Option<u64>) -> Value {
    let mut schema = json!({
        "type": "integer",
        "description": description,
    });
    if let Some(minimum) = minimum {
        schema["minimum"] = json!(minimum);
    }
    schema
}

pub(super) fn boolean_schema(description: &str) -> Value {
    json!({
        "type": "boolean",
        "description": description,
    })
}

pub(super) fn string_array_schema(description: &str) -> Value {
    json!({
        "type": "array",
        "description": description,
        "items": {"type": "string"}
    })
}

pub(super) fn const_true_schema(description: &str) -> Value {
    json!({
        "type": "boolean",
        "const": true,
        "description": description,
    })
}

pub(super) fn enum_schema(description: &str, options: &[&str]) -> Value {
    json!({
        "type": "string",
        "description": description,
        "oneOf": options.iter().map(|option| json!({"const": option})).collect::<Vec<_>>()
    })
}

pub(crate) fn privacy_mode_schema() -> Value {
    enum_schema(
        "How RMU should handle potentially sensitive path and content fragments in results. Use `off` for unsanitized output.",
        &["off", "mask", "hash"],
    )
}

pub(crate) fn rollout_phase_schema() -> Value {
    enum_schema(
        "Retrieval rollout phase to use for this request.",
        &["shadow", "canary_5", "canary_25", "full_100"],
    )
}

pub(crate) fn migration_mode_schema() -> Value {
    enum_schema(
        "How schema migrations should be handled before serving this request.",
        &["auto", "off"],
    )
}
