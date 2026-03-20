use serde_json::Value;

#[path = "schema/common.rs"]
mod common;
#[path = "schema/one_of.rs"]
mod one_of;
#[path = "schema/refs.rs"]
mod refs;
#[path = "schema_keywords.rs"]
mod schema_keywords;
#[path = "schema/shapes.rs"]
mod shapes;

use common::schema_object;
use schema_keywords::validate_supported_keywords;

pub(super) fn validate_schema_keyword_coverage(
    schema: &Value,
    context: &str,
) -> std::result::Result<(), String> {
    if schema.is_boolean() {
        return Ok(());
    }

    let schema_object = schema_object(schema, context)?;
    validate_supported_keywords(schema_object, context)?;

    one_of::validate_keyword_coverage(schema_object, context)?;
    shapes::validate_keyword_coverage(schema_object, context)?;

    Ok(())
}

pub(super) fn validate_required_structure(
    value: &Value,
    schema: &Value,
    context: &str,
) -> std::result::Result<(), String> {
    if let Some(boolean_schema) = schema.as_bool() {
        if boolean_schema {
            return Ok(());
        }
        return Err(format!("boolean schema `false` rejects value at {context}"));
    }

    let schema_object = schema_object(schema, context)?;
    validate_supported_keywords(schema_object, context)?;

    refs::validate(value, schema, context)?;
    one_of::validate_required(value, schema, context)?;
    shapes::validate_required(value, schema, context)?;

    Ok(())
}

pub(super) fn assert_required_structure(value: &Value, schema: &Value, context: &str) {
    if let Err(error) = validate_required_structure(value, schema, context) {
        panic!("expected schema validation success at {context}, got: {error}");
    }
}

pub(super) fn assert_schema_rejects(value: &Value, schema: &Value, context: &str) {
    let result = validate_required_structure(value, schema, context);
    assert!(
        result.is_err(),
        "expected schema validation to fail at {context}"
    );
}
