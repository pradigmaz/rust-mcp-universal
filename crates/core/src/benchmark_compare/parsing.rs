use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;

pub(super) fn parse_required_metric(value: &Value, key: &str, source: &str) -> Result<f32> {
    let field_path = format!("{source}.{key}");
    let raw = value
        .get(key)
        .ok_or_else(|| anyhow!("`{source}` is missing `{key}`"))?;
    parse_metric_number(raw, &field_path)
}

pub(super) fn parse_optional_metric(
    section: Option<&Value>,
    key: &str,
    source: &str,
) -> Result<Option<f32>> {
    let Some(section) = section else {
        return Ok(None);
    };
    let section_obj = section
        .as_object()
        .ok_or_else(|| anyhow!("`{source}` must be an object"))?;
    let Some(raw) = section_obj.get(key) else {
        return Ok(None);
    };
    let field_path = format!("{source}.{key}");
    parse_metric_number(raw, &field_path).map(Some)
}

fn parse_metric_number(value: &Value, field_path: &str) -> Result<f32> {
    let number = value
        .as_f64()
        .ok_or_else(|| anyhow!("`{field_path}` must be a number"))?;
    if !number.is_finite() {
        bail!("`{field_path}` must be finite");
    }
    let narrowed = number as f32;
    if !narrowed.is_finite() {
        bail!("`{field_path}` is out of range for f32");
    }
    Ok(narrowed)
}

pub(super) fn read_json_file(path: &Path, label: &str) -> Result<Value> {
    let raw = fs::read(path)
        .with_context(|| format!("failed to read {label} file `{}`", path.display()))?;
    let content = std::str::from_utf8(&raw)
        .with_context(|| format!("{label} file `{}` is not valid UTF-8", path.display()))?;
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    serde_json::from_str(content)
        .with_context(|| format!("failed to parse {label} JSON `{}`", path.display()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_metric_number;

    #[test]
    fn parse_metric_number_accepts_finite_numbers() {
        let cases = [
            (json!(0.0), 0.0_f32),
            (json!(1.5), 1.5_f32),
            (json!(-42.0), -42.0_f32),
        ];

        for (value, expected) in cases {
            let parsed = parse_metric_number(&value, "metrics.value").expect("must parse");
            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn parse_metric_number_rejects_non_number() {
        let error = parse_metric_number(&json!("not-a-number"), "metrics.value")
            .expect_err("non-number must fail");

        assert_eq!(error.to_string(), "`metrics.value` must be a number");
    }

    #[test]
    fn parse_metric_number_rejects_out_of_range_after_narrowing() {
        let error = parse_metric_number(&json!(1e100), "metrics.value")
            .expect_err("out-of-range f64 must fail");

        assert_eq!(error.to_string(), "`metrics.value` is out of range for f32");
    }
}
