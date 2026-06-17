use serde_json::{Value, json};

pub(super) fn tool(
    name: &str,
    description: &str,
    input_schema: Value,
    read_only: bool,
    destructive: bool,
    idempotent: bool,
) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
        "annotations": {
            "readOnlyHint": read_only,
            "destructiveHint": destructive,
            "idempotentHint": idempotent,
            "openWorldHint": false
        }
    })
}

pub(super) fn tool_with_output(
    name: &str,
    description: &str,
    input_schema: Value,
    output_schema: Value,
    read_only: bool,
    destructive: bool,
    idempotent: bool,
) -> Value {
    let mut tool = tool(
        name,
        description,
        input_schema,
        read_only,
        destructive,
        idempotent,
    );
    tool["outputSchema"] = output_schema;
    tool
}

pub(super) fn json_schema_object(fields: &[(&str, Value)], required: &[&str]) -> Value {
    let properties = fields
        .iter()
        .map(|(name, schema)| ((*name).to_string(), schema.clone()))
        .collect::<serde_json::Map<String, Value>>();
    let required_values = required
        .iter()
        .map(|name| Value::String((*name).to_string()))
        .collect::<Vec<_>>();
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties,
        "required": required_values
    })
}
