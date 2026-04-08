use anyhow::Result;
use rmu_core::{Engine, PrivacyMode, SensitiveDataOptions, sanitize_value_for_privacy};

use crate::output::{print_json, print_line};

pub(crate) struct SensitiveDataArgs {
    pub(crate) path_prefix: Option<String>,
    pub(crate) limit: usize,
    pub(crate) include_low_confidence: bool,
}

pub(crate) fn run(
    engine: &Engine,
    json: bool,
    privacy_mode: PrivacyMode,
    args: SensitiveDataArgs,
) -> Result<()> {
    let result = engine.sensitive_data(&SensitiveDataOptions {
        path_prefix: args.path_prefix.map(|value| value.replace('\\', "/")),
        limit: args.limit,
        include_low_confidence: args.include_low_confidence,
    })?;

    if json {
        let mut value = serde_json::to_value(&result)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        print_line(format!(
            "evaluated_files={}, findings={}, high_confidence_findings={}",
            result.summary.evaluated_files,
            result.summary.findings,
            result.summary.high_confidence_findings
        ));
    }
    Ok(())
}
