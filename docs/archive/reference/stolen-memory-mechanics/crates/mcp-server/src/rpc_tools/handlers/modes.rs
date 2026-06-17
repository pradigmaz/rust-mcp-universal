use anyhow::Result;
use obsidian_memory_core::{PrivacyMode, StorageMode};
use serde_json::Value;

use crate::rpc_tools::errors::invalid_params_error;
use crate::rpc_tools::parsing::parse_optional_non_empty_string;

pub(super) fn parse_optional_privacy_mode(
    args: &Value,
    tool_name: &str,
    field_name: &str,
) -> Result<Option<PrivacyMode>> {
    let value = parse_optional_non_empty_string(args, tool_name, field_name)?;
    let Some(raw) = value else {
        return Ok(None);
    };
    let parsed = PrivacyMode::parse(&raw).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `{field_name}` must be one of: off, mask, hash"
        ))
    })?;
    Ok(Some(parsed))
}

pub(super) fn parse_optional_storage_mode(
    args: &Value,
    tool_name: &str,
    field_name: &str,
) -> Result<Option<StorageMode>> {
    let value = parse_optional_non_empty_string(args, tool_name, field_name)?;
    let Some(raw) = value else {
        return Ok(None);
    };
    let parsed = StorageMode::parse(&raw).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `{field_name}` must be one of: codex, project"
        ))
    })?;
    Ok(Some(parsed))
}
