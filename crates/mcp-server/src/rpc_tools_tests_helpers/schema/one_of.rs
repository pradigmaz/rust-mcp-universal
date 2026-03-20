use serde_json::{Map, Value};

use super::validate_required_structure as validate_schema_required_structure;
use super::validate_schema_keyword_coverage;

pub(super) fn validate_keyword_coverage(
    schema_object: &Map<String, Value>,
    context: &str,
) -> std::result::Result<(), String> {
    if let Some(one_of) = schema_object.get("oneOf").and_then(Value::as_array) {
        for (idx, branch) in one_of.iter().enumerate() {
            validate_schema_keyword_coverage(branch, &format!("{context}.oneOf[{idx}]"))?;
        }
    }

    Ok(())
}

pub(super) fn validate_required(
    value: &Value,
    schema: &Value,
    context: &str,
) -> std::result::Result<(), String> {
    if let Some(branches) = schema.get("oneOf").and_then(Value::as_array) {
        let matched = branches
            .iter()
            .filter(|branch| validate_schema_required_structure(value, branch, context).is_ok())
            .count();
        if matched != 1 {
            return Err(format!(
                "oneOf violation at {context}: expected exactly one branch, got {matched}"
            ));
        }
    }

    Ok(())
}
