use serde_json::{Map, Value};

pub(super) fn schema_object<'a>(
    schema: &'a Value,
    context: &str,
) -> std::result::Result<&'a Map<String, Value>, String> {
    schema
        .as_object()
        .ok_or_else(|| format!("schema at {context} must be object or boolean, got `{schema}`"))
}
