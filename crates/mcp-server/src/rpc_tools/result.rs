use anyhow::Result;
use rmu_core::PreflightStatus;
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
    let details = compatibility_details(&message, None);
    json!({
        "content": [
            {
                "type": "text",
                "text": message
            }
        ],
        "structuredContent": {
            "error": message,
            "code": if details.is_some() { "E_COMPATIBILITY" } else { "E_RUNTIME" },
            "details": details
        },
        "isError": true
    })
}

pub(super) fn tool_compatibility_error_result(
    message: String,
    status: Option<&PreflightStatus>,
) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": message
            }
        ],
        "structuredContent": {
            "error": message,
            "code": "E_COMPATIBILITY",
            "details": compatibility_details(&message, status)
        },
        "isError": true
    })
}

fn compatibility_details(message: &str, status: Option<&PreflightStatus>) -> Option<Value> {
    if status.is_none() && !is_compatibility_message(message) {
        return None;
    }
    let mut details = json!({
        "kind": "compatibility",
        "safe_recovery_hint": status.map_or_else(
            || default_safe_recovery_hint().to_string(),
            |value| value.safe_recovery_hint.clone()
        ),
        "reason": message
    });
    if let Some(status) = status {
        let details_object = details
            .as_object_mut()
            .expect("compatibility details must be object");
        details_object.insert(
            "running_binary_version".to_string(),
            json!(status.running_binary_version),
        );
        details_object.insert(
            "running_binary_stale".to_string(),
            json!(status.running_binary_stale),
        );
        details_object.insert(
            "stale_process_suspected".to_string(),
            json!(status.stale_process_suspected),
        );
        if let Some(supported_schema_version) = status.supported_schema_version {
            details_object.insert(
                "supported_schema_version".to_string(),
                json!(supported_schema_version),
            );
        }
        if let Some(db_schema_version) = status.db_schema_version {
            details_object.insert("db_schema_version".to_string(), json!(db_schema_version));
        }
    }
    Some(details)
}

fn is_compatibility_message(message: &str) -> bool {
    message.contains("newer than binary supported")
        || message.contains("running binary version") && message.contains("is stale")
}

fn default_safe_recovery_hint() -> &'static str {
    if cfg!(windows) {
        "use scripts/rmu-mcp-server-fresh.cmd or restart the process with a fresh binary, then re-open the index"
    } else {
        "restart the process with a fresh binary and re-open the index"
    }
}
