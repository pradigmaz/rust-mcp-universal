use std::path::Path;

use anyhow::{Result, bail};
use serde_json::Value;

use super::{ThresholdConfig, parse::ParsedThresholdSections};

pub(super) fn validate_section_shapes(
    sections: &ParsedThresholdSections<'_>,
    source: &str,
) -> Result<()> {
    if sections.min.is_some() && !sections.min.is_some_and(Value::is_object) {
        bail!("`{source}.min` must be an object");
    }
    if sections.max.is_some() && !sections.max.is_some_and(Value::is_object) {
        bail!("`{source}.max` must be an object");
    }
    Ok(())
}

pub(super) fn validate_supported_metrics(config: &ThresholdConfig, path: &Path) -> Result<()> {
    if !config.has_thresholds() {
        bail!(
            "thresholds file `{}` does not define supported min/max metrics",
            path.display()
        );
    }
    Ok(())
}
