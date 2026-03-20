use anyhow::Result;
use serde_json::Value;

use super::errors::invalid_params_error;

pub(super) fn parse_optional_usize_with_min(
    args: &Value,
    tool: &str,
    field: &str,
    minimum: usize,
    default: usize,
) -> Result<usize> {
    let Some(value) = args.get(field) else {
        return Ok(default);
    };

    let parsed = match (value.as_u64(), value.as_i64()) {
        (Some(raw), _) => usize::try_from(raw).map_err(|_| {
            invalid_params_error(format!(
                "{tool} requires `{field}` <= {}, got {raw}",
                usize::MAX
            ))
        })?,
        (None, Some(raw)) => {
            if raw < 0 {
                return Err(invalid_params_error(format!(
                    "{tool} requires `{field}` >= {minimum}, got {raw}"
                )));
            }
            raw as usize
        }
        (None, None) => {
            return Err(invalid_params_error(format!(
                "{tool} requires integer `{field}` >= {minimum}, got {}",
                value
            )));
        }
    };

    if parsed < minimum {
        return Err(invalid_params_error(format!(
            "{tool} requires `{field}` >= {minimum}, got {parsed}"
        )));
    }

    let max_supported = usize::try_from(i64::MAX).unwrap_or(usize::MAX);
    if parsed > max_supported {
        return Err(invalid_params_error(format!(
            "{tool} requires `{field}` <= {max_supported}, got {parsed}"
        )));
    }

    Ok(parsed)
}

pub(super) fn parse_optional_bool(args: &Value, tool: &str, field: &str) -> Result<Option<bool>> {
    let Some(value) = args.get(field) else {
        return Ok(None);
    };

    let Some(parsed) = value.as_bool() else {
        return Err(invalid_params_error(format!(
            "{tool} requires boolean `{field}`, got {}",
            value
        )));
    };

    Ok(Some(parsed))
}

pub(super) fn parse_optional_string(
    args: &Value,
    tool: &str,
    field: &str,
) -> Result<Option<String>> {
    let Some(value) = args.get(field) else {
        return Ok(None);
    };

    let Some(parsed) = value.as_str() else {
        return Err(invalid_params_error(format!(
            "{tool} requires string `{field}`, got {}",
            value
        )));
    };

    Ok(Some(parsed.to_string()))
}

pub(super) fn parse_optional_non_empty_string(
    args: &Value,
    tool: &str,
    field: &str,
) -> Result<Option<String>> {
    let Some(raw) = parse_optional_string(args, tool, field)? else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(invalid_params_error(format!(
            "{tool} requires non-empty `{field}`"
        )));
    }
    Ok(Some(trimmed.to_string()))
}

pub(super) fn parse_optional_string_list(
    args: &Value,
    tool: &str,
    field: &str,
) -> Result<Option<Vec<String>>> {
    let Some(value) = args.get(field) else {
        return Ok(None);
    };

    let Some(items) = value.as_array() else {
        return Err(invalid_params_error(format!(
            "{tool} requires array `{field}`, got {}",
            value
        )));
    };

    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(raw) = item.as_str() else {
            return Err(invalid_params_error(format!(
                "{tool} requires string items in `{field}`, got {}",
                item
            )));
        };
        out.push(raw.to_string());
    }

    Ok(Some(out))
}

pub(super) fn parse_required_non_empty_string(
    args: &Value,
    tool: &str,
    field: &str,
) -> Result<String> {
    let value = args
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params_error(format!("{tool} requires `{field}`")))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(invalid_params_error(format!(
            "{tool} requires non-empty `{field}`"
        )));
    }
    Ok(trimmed.to_string())
}

pub(super) fn reject_unknown_fields(args: &Value, tool: &str, allowed: &[&str]) -> Result<()> {
    let object = args
        .as_object()
        .ok_or_else(|| invalid_params_error(format!("{tool} expects object arguments")))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(invalid_params_error(format!(
                "{tool} does not allow argument `{key}`"
            )));
        }
    }
    Ok(())
}
