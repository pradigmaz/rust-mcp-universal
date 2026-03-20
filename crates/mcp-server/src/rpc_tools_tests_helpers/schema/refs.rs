use serde_json::Value;

use super::super::load_schema;
use super::validate_required_structure;

pub(super) fn validate(
    value: &Value,
    schema: &Value,
    context: &str,
) -> std::result::Result<(), String> {
    if let Some(schema_ref) = schema.get("$ref").and_then(Value::as_str) {
        let schema_file = schema_ref.strip_prefix("./").unwrap_or(schema_ref);
        let referenced_schema = load_schema(schema_file);
        validate_required_structure(value, &referenced_schema, &format!("{context}.$ref"))?;
    }

    Ok(())
}
