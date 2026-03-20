use serde_json::json;

use super::*;

#[test]
fn local_schema_checker_rejects_one_of_overlap() {
    let schema = json!({
        "oneOf": [
            {"type": "number"},
            {"type": "integer"}
        ]
    });

    assert_required_structure(&json!(1.5), &schema, "oneOf.single-branch");
    assert_schema_rejects(&json!(1), &schema, "oneOf.overlap");
}

#[test]
fn local_schema_checker_rejects_one_of_zero_match() {
    let schema = json!({
        "oneOf": [
            {"type": "string"},
            {"type": "integer"}
        ]
    });

    assert_schema_rejects(&json!(true), &schema, "oneOf.zero-match");
}

#[test]
fn local_schema_checker_applies_sibling_constraints_after_one_of() {
    let schema = json!({
        "oneOf": [
            {
                "type": "object",
                "properties": {"kind": {"const": "a"}},
                "required": ["kind"]
            },
            {
                "type": "object",
                "properties": {"kind": {"const": "b"}},
                "required": ["kind"]
            }
        ],
        "type": "object",
        "properties": {
            "payload": {"type": "string", "minLength": 1}
        },
        "required": ["payload"]
    });

    assert_required_structure(
        &json!({"kind": "a", "payload": "ok"}),
        &schema,
        "oneOf.sibling.valid",
    );
    assert_schema_rejects(
        &json!({"kind": "a"}),
        &schema,
        "oneOf.sibling.missing-required",
    );
}

#[test]
fn local_schema_checker_rejects_minimum_shape_violations() {
    let min_length_schema = json!({
        "type": "string",
        "minLength": 3
    });
    assert_schema_rejects(&json!("ab"), &min_length_schema, "mini.minLength");

    let min_items_schema = json!({
        "type": "array",
        "minItems": 2,
        "items": {"type": "integer"}
    });
    assert_schema_rejects(&json!([1]), &min_items_schema, "mini.minItems");

    let min_properties_schema = json!({
        "type": "object",
        "minProperties": 2
    });
    assert_schema_rejects(
        &json!({"only": true}),
        &min_properties_schema,
        "mini.minProperties",
    );
}

#[test]
fn local_schema_checker_rejects_maximum_shape_violations() {
    let max_length_schema = json!({
        "type": "string",
        "maxLength": 4
    });
    assert_schema_rejects(&json!("abcde"), &max_length_schema, "maxi.maxLength");

    let max_items_schema = json!({
        "type": "array",
        "maxItems": 2,
        "items": {"type": "integer"}
    });
    assert_schema_rejects(&json!([1, 2, 3]), &max_items_schema, "maxi.maxItems");

    let max_properties_schema = json!({
        "type": "object",
        "maxProperties": 1
    });
    assert_schema_rejects(
        &json!({"a": 1, "b": 2}),
        &max_properties_schema,
        "maxi.maxProperties",
    );
}

#[test]
fn local_schema_checker_accepts_min_max_boundaries() {
    let numeric_schema = json!({
        "type": "number",
        "minimum": 1.0,
        "maximum": 2.0
    });
    assert_required_structure(&json!(1.0), &numeric_schema, "bounds.number.eq-min");
    assert_required_structure(&json!(2.0), &numeric_schema, "bounds.number.eq-max");

    let string_schema = json!({
        "type": "string",
        "minLength": 3,
        "maxLength": 5
    });
    assert_required_structure(&json!("abc"), &string_schema, "bounds.string.eq-min");
    assert_required_structure(&json!("abcde"), &string_schema, "bounds.string.eq-max");

    let array_schema = json!({
        "type": "array",
        "minItems": 2,
        "maxItems": 3,
        "items": {"type": "integer"}
    });
    assert_required_structure(&json!([1, 2]), &array_schema, "bounds.array.eq-min");
    assert_required_structure(&json!([1, 2, 3]), &array_schema, "bounds.array.eq-max");

    let object_schema = json!({
        "type": "object",
        "minProperties": 1,
        "maxProperties": 2
    });
    assert_required_structure(&json!({"a": 1}), &object_schema, "bounds.object.eq-min");
    assert_required_structure(
        &json!({"a": 1, "b": 2}),
        &object_schema,
        "bounds.object.eq-max",
    );
}

#[test]
fn local_schema_checker_rejects_additional_properties_when_disabled() {
    let schema = json!({
        "type": "object",
        "properties": {
            "ok": {"type": "integer"}
        },
        "additionalProperties": false
    });

    assert_required_structure(&json!({"ok": 1}), &schema, "additional.allowed");
    assert_schema_rejects(
        &json!({"ok": 1, "unexpected": true}),
        &schema,
        "additional.rejected",
    );
}

#[test]
fn local_schema_checker_honors_boolean_schemas() {
    assert_required_structure(&json!({"any": "value"}), &json!(true), "bool.true");
    assert_schema_rejects(&json!({"any": "value"}), &json!(false), "bool.false");
}

#[test]
fn local_schema_checker_rejects_unsupported_keywords() {
    let schema = json!({
        "type": "string",
        "enum": ["ok", "fail"]
    });
    assert_schema_rejects(&json!("ok"), &schema, "unsupported.enum");
}
