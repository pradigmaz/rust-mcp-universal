use serde_json::Value;

pub(super) fn validate_bounds(
    actual: usize,
    min: Option<u64>,
    max: Option<u64>,
    min_name: &str,
    max_name: &str,
    context: &str,
) -> std::result::Result<(), String> {
    if let Some(min) = min {
        if actual < min as usize {
            return Err(format!(
                "{min_name} violation at {context}: expected >= {min}, got {actual}"
            ));
        }
    }
    if let Some(max) = max {
        if actual > max as usize {
            return Err(format!(
                "{max_name} violation at {context}: expected <= {max}, got {actual}"
            ));
        }
    }

    Ok(())
}

pub(super) fn validate_supported_keywords(
    schema: &serde_json::Map<String, Value>,
    context: &str,
) -> std::result::Result<(), String> {
    const SUPPORTED: &[&str] = &[
        "$schema",
        "$id",
        "$ref",
        "title",
        "description",
        "default",
        "examples",
        "oneOf",
        "type",
        "const",
        "minimum",
        "maximum",
        "minLength",
        "maxLength",
        "minItems",
        "maxItems",
        "minProperties",
        "maxProperties",
        "required",
        "properties",
        "additionalProperties",
        "items",
    ];

    for key in schema.keys() {
        if !SUPPORTED.contains(&key.as_str()) {
            return Err(format!("unsupported schema keyword `{key}` at {context}"));
        }
    }
    Ok(())
}
