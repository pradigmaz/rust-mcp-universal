use anyhow::Result;
use obsidian_memory_core::PreflightStatus;
use serde_json::{Value, json};

pub(crate) fn tool_result(structured_content: Value) -> Result<Value> {
    let text = serde_json::to_string_pretty(&structured_content)?;
    Ok(json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": structured_content,
        "isError": false
    }))
}

pub(crate) fn tool_error_result(message: String) -> Value {
    json!({
        "content": [{"type": "text", "text": message}],
        "structuredContent": {"error": message, "code": "E_RUNTIME", "details": {}},
        "isError": true
    })
}

pub(crate) fn tool_state_error_result(code: &str, message: String, details: Value) -> Value {
    json!({
        "content": [{"type": "text", "text": message}],
        "structuredContent": {
            "error": message,
            "code": code,
            "details": details
        },
        "isError": true
    })
}

pub(crate) fn tool_compatibility_error_result(
    message: String,
    status: Option<&PreflightStatus>,
) -> Value {
    let code = if status.is_some_and(|status| {
        status.db_schema_version.unwrap_or_default()
            > status.supported_schema_version.unwrap_or_default()
    }) {
        "E_SCHEMA_MISMATCH"
    } else {
        "E_COMPATIBILITY"
    };
    let details = status.map(|status| {
        json!({
            "kind": "compatibility",
            "safe_recovery_hint": status.safe_recovery_hint,
            "running_binary_version": status.running_binary_version,
            "running_binary_stale": status.running_binary_stale,
            "db_schema_version": status.db_schema_version,
            "supported_schema_version": status.supported_schema_version,
            "stale_process_suspected": status.stale_process_suspected,
            "reason": message
        })
    });

    json!({
        "content": [{"type": "text", "text": message}],
        "structuredContent": {
            "error": message,
            "code": code,
            "details": details
        },
        "isError": true
    })
}
