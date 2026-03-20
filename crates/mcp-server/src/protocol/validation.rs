use rmu_core::PrivacyMode;
use serde_json::Value;

use super::PROTOCOL_VERSION;

pub(super) fn validate_tools_call_params(
    params: Option<&Value>,
) -> std::result::Result<(), String> {
    let params = params.ok_or_else(|| "tools/call params are required".to_string())?;
    let object = params
        .as_object()
        .ok_or_else(|| "tools/call params must be object".to_string())?;

    match object.get("name") {
        Some(Value::String(name)) if !name.trim().is_empty() => {}
        Some(_) => return Err("tools/call requires string field `name`".to_string()),
        None => return Err("tools/call requires string field `name`".to_string()),
    }

    if let Some(arguments) = object.get("arguments") {
        if !arguments.is_object() {
            return Err(format!(
                "tools/call `arguments` must be object, got {}",
                arguments
            ));
        }
    }

    Ok(())
}

pub(super) fn extract_privacy_mode_from_tools_call_params(params: Option<&Value>) -> PrivacyMode {
    let Some(params_obj) = params.and_then(Value::as_object) else {
        return PrivacyMode::Off;
    };
    let Some(arguments) = params_obj.get("arguments").and_then(Value::as_object) else {
        return PrivacyMode::Off;
    };
    let Some(raw_mode) = arguments.get("privacy_mode").and_then(Value::as_str) else {
        return PrivacyMode::Off;
    };
    PrivacyMode::parse(raw_mode).unwrap_or(PrivacyMode::Off)
}

pub(super) fn validate_initialize_params(
    params: Option<&Value>,
) -> std::result::Result<(), String> {
    let params = params.ok_or_else(|| "initialize params are required".to_string())?;
    let object = params
        .as_object()
        .ok_or_else(|| "initialize params must be object".to_string())?;

    match object.get("protocolVersion") {
        Some(Value::String(version)) if version == PROTOCOL_VERSION => {}
        Some(Value::String(_)) => {
            return Err(format!(
                "initialize `protocolVersion` must be `{PROTOCOL_VERSION}`"
            ));
        }
        _ => return Err("initialize requires string field `protocolVersion`".to_string()),
    }

    match object.get("capabilities") {
        Some(Value::Object(_)) => {}
        _ => return Err("initialize requires object field `capabilities`".to_string()),
    }

    let client_info = object
        .get("clientInfo")
        .and_then(Value::as_object)
        .ok_or_else(|| "initialize requires object field `clientInfo`".to_string())?;

    let has_non_empty = |field| {
        client_info
            .get(field)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    };

    if !has_non_empty("name") {
        return Err("initialize requires non-empty string `clientInfo.name`".to_string());
    }
    if !has_non_empty("version") {
        return Err("initialize requires non-empty string `clientInfo.version`".to_string());
    }

    Ok(())
}
