use anyhow::Result;
use serde_json::{Value, json};

pub(super) fn tool_result(structured_content: Value) -> Result<Value> {
    let text = serde_json::to_string_pretty(&structured_content)?;
    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "structuredContent": structured_content,
        "isError": false
    }))
}

pub(super) fn tool_error_result(message: String) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": message
            }
        ],
        "structuredContent": {
            "error": message
        },
        "isError": true
    })
}
