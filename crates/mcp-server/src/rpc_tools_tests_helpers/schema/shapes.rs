use serde_json::{Map, Value};

use super::schema_keywords::validate_bounds;
use super::validate_required_structure as validate_schema_required_structure;
use super::validate_schema_keyword_coverage;

pub(super) fn validate_keyword_coverage(
    schema_object: &Map<String, Value>,
    context: &str,
) -> std::result::Result<(), String> {
    if let Some(properties) = schema_object.get("properties").and_then(Value::as_object) {
        for (name, property_schema) in properties {
            validate_schema_keyword_coverage(
                property_schema,
                &format!("{context}.properties.{name}"),
            )?;
        }
    }
    if let Some(additional_properties) = schema_object.get("additionalProperties") {
        if additional_properties.is_boolean() || additional_properties.is_object() {
            validate_schema_keyword_coverage(
                additional_properties,
                &format!("{context}.additionalProperties"),
            )?;
        }
    }
    if let Some(items_schema) = schema_object.get("items") {
        validate_schema_keyword_coverage(items_schema, &format!("{context}.items"))?;
    }

    Ok(())
}

pub(super) fn validate_required(
    value: &Value,
    schema: &Value,
    context: &str,
) -> std::result::Result<(), String> {
    if let Some(expected_type) = schema.get("type").and_then(Value::as_str) {
        let type_matches = match expected_type {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
            "boolean" => value.is_boolean(),
            _ => true,
        };
        if !type_matches {
            return Err(format!(
                "schema type mismatch at {context}: expected `{expected_type}`, got `{value}`"
            ));
        }
    }
    if let Some(const_value) = schema.get("const") {
        if value != const_value {
            return Err(format!(
                "const mismatch at {context}: expected `{const_value}`, got `{value}`"
            ));
        }
    }
    if let Some(minimum) = schema.get("minimum").and_then(Value::as_f64) {
        let Some(actual) = value.as_f64() else {
            return Err(format!("expected number for minimum at {context}"));
        };
        if actual < minimum {
            return Err(format!(
                "minimum violation at {context}: expected >= {minimum}, got {actual}"
            ));
        }
    }
    if let Some(maximum) = schema.get("maximum").and_then(Value::as_f64) {
        let Some(actual) = value.as_f64() else {
            return Err(format!("expected number for maximum at {context}"));
        };
        if actual > maximum {
            return Err(format!(
                "maximum violation at {context}: expected <= {maximum}, got {actual}"
            ));
        }
    }

    let min_length = schema.get("minLength").and_then(Value::as_u64);
    let max_length = schema.get("maxLength").and_then(Value::as_u64);
    if min_length.is_some() || max_length.is_some() {
        let Some(actual) = value.as_str().map(|raw| raw.chars().count()) else {
            return Err(format!(
                "expected string for minLength/maxLength at {context}"
            ));
        };
        validate_bounds(
            actual,
            min_length,
            max_length,
            "minLength",
            "maxLength",
            context,
        )?;
    }

    let min_items = schema.get("minItems").and_then(Value::as_u64);
    let max_items = schema.get("maxItems").and_then(Value::as_u64);
    if min_items.is_some() || max_items.is_some() {
        let Some(actual) = value.as_array().map(Vec::len) else {
            return Err(format!("expected array for minItems/maxItems at {context}"));
        };
        validate_bounds(
            actual, min_items, max_items, "minItems", "maxItems", context,
        )?;
    }

    let min_properties = schema.get("minProperties").and_then(Value::as_u64);
    let max_properties = schema.get("maxProperties").and_then(Value::as_u64);
    if min_properties.is_some() || max_properties.is_some() {
        let Some(actual) = value.as_object().map(serde_json::Map::len) else {
            return Err(format!(
                "expected object for minProperties/maxProperties at {context}"
            ));
        };
        validate_bounds(
            actual,
            min_properties,
            max_properties,
            "minProperties",
            "maxProperties",
            context,
        )?;
    }

    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        let Some(object) = value.as_object() else {
            return Err(format!("expected object at {context}"));
        };
        for field in required {
            let Some(key) = field.as_str() else {
                return Err(format!("non-string required field at {context}: {field}"));
            };
            if !object.contains_key(key) {
                return Err(format!("missing required field `{key}` at {context}"));
            }
        }
    }

    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        let Some(object) = value.as_object() else {
            return Err(format!("expected object for properties at {context}"));
        };
        for (name, property_schema) in properties {
            let Some(property_value) = object.get(name) else {
                continue;
            };
            validate_schema_required_structure(
                property_value,
                property_schema,
                &format!("{context}.{name}"),
            )?;
        }
    }

    if let Some(additional_properties) = schema.get("additionalProperties") {
        let Some(object) = value.as_object() else {
            return Err(format!(
                "expected object for additionalProperties at {context}"
            ));
        };
        let declared_properties = schema.get("properties").and_then(Value::as_object);
        for (name, property_value) in object {
            let is_declared = declared_properties.is_some_and(|props| props.contains_key(name));
            if is_declared {
                continue;
            }
            match additional_properties {
                Value::Bool(false) => {
                    return Err(format!(
                        "additionalProperties violation at {context}: unexpected `{name}`"
                    ));
                }
                Value::Object(additional_schema) => {
                    let schema_value = Value::Object(additional_schema.clone());
                    validate_schema_required_structure(
                        property_value,
                        &schema_value,
                        &format!("{context}.{name}"),
                    )?;
                }
                _ => {}
            }
        }
    }

    if let Some(items_schema) = schema.get("items") {
        let Some(array) = value.as_array() else {
            return Err(format!("expected array at {context}"));
        };
        for (idx, item) in array.iter().enumerate() {
            validate_schema_required_structure(item, items_schema, &format!("{context}[{idx}]"))?;
        }
    }

    Ok(())
}
