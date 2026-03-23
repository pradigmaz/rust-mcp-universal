use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Default)]
pub(crate) struct QualityPolicy {
    pub(crate) thresholds: QualityThresholds,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct QualityThresholds {
    pub(crate) max_non_empty_lines_default: i64,
    pub(crate) max_non_empty_lines_test: i64,
    pub(crate) max_non_empty_lines_config: i64,
    pub(crate) max_size_bytes: i64,
    pub(crate) max_import_count: i64,
    pub(crate) max_line_length: i64,
    pub(crate) max_symbol_count_per_file: i64,
    pub(crate) max_ref_count_per_file: i64,
    pub(crate) max_module_dep_count_per_file: i64,
    pub(crate) max_graph_edge_out_count: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct QualityPolicyFile {
    #[serde(default)]
    thresholds: QualityThresholdOverrides,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct QualityThresholdOverrides {
    max_non_empty_lines_default: Option<i64>,
    max_non_empty_lines_test: Option<i64>,
    max_non_empty_lines_config: Option<i64>,
    max_size_bytes: Option<i64>,
    max_import_count: Option<i64>,
    max_line_length: Option<i64>,
    max_symbol_count_per_file: Option<i64>,
    max_ref_count_per_file: Option<i64>,
    max_module_dep_count_per_file: Option<i64>,
    max_graph_edge_out_count: Option<i64>,
}

impl QualityThresholds {
    pub(crate) fn threshold_for_rule(&self, rule_id: &str) -> Option<i64> {
        Some(match rule_id {
            "max_non_empty_lines_default" => self.max_non_empty_lines_default,
            "max_non_empty_lines_test" => self.max_non_empty_lines_test,
            "max_non_empty_lines_config" => self.max_non_empty_lines_config,
            "max_size_bytes" => self.max_size_bytes,
            "max_import_count" => self.max_import_count,
            "max_line_length" => self.max_line_length,
            "max_symbol_count_per_file" => self.max_symbol_count_per_file,
            "max_ref_count_per_file" => self.max_ref_count_per_file,
            "max_module_dep_count_per_file" => self.max_module_dep_count_per_file,
            "max_graph_edge_out_count" => self.max_graph_edge_out_count,
            _ => return None,
        })
    }
}

pub(crate) fn default_quality_policy() -> QualityPolicy {
    QualityPolicy {
        thresholds: QualityThresholds {
            max_non_empty_lines_default: super::metrics::MAX_NON_EMPTY_LINES_DEFAULT,
            max_non_empty_lines_test: super::metrics::MAX_NON_EMPTY_LINES_TEST,
            max_non_empty_lines_config: super::metrics::MAX_NON_EMPTY_LINES_CONFIG,
            max_size_bytes: super::metrics::MAX_SIZE_BYTES,
            max_import_count: super::metrics::MAX_IMPORT_COUNT,
            max_line_length: super::metrics::MAX_LINE_LENGTH,
            max_symbol_count_per_file: super::metrics::MAX_SYMBOL_COUNT_PER_FILE,
            max_ref_count_per_file: super::metrics::MAX_REF_COUNT_PER_FILE,
            max_module_dep_count_per_file: super::metrics::MAX_MODULE_DEP_COUNT_PER_FILE,
            max_graph_edge_out_count: super::metrics::MAX_GRAPH_EDGE_OUT_COUNT,
        },
    }
}

pub(crate) fn load_quality_policy(project_root: &Path) -> Result<QualityPolicy> {
    let policy_path = project_root.join("rmu-quality-policy.json");
    if !policy_path.exists() {
        return Ok(default_quality_policy());
    }

    let raw = fs::read(&policy_path)
        .with_context(|| format!("failed to read quality policy `{}`", policy_path.display()))?;
    let content = std::str::from_utf8(&raw).with_context(|| {
        format!(
            "quality policy `{}` is not valid UTF-8",
            policy_path.display()
        )
    })?;
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let parsed: QualityPolicyFile = serde_json::from_str(content)
        .with_context(|| format!("failed to parse quality policy `{}`", policy_path.display()))?;

    let mut policy = default_quality_policy();
    apply_threshold_overrides(&mut policy.thresholds, parsed.thresholds);
    Ok(policy)
}

fn apply_threshold_overrides(
    thresholds: &mut QualityThresholds,
    overrides: QualityThresholdOverrides,
) {
    if let Some(value) = overrides.max_non_empty_lines_default {
        thresholds.max_non_empty_lines_default = value;
    }
    if let Some(value) = overrides.max_non_empty_lines_test {
        thresholds.max_non_empty_lines_test = value;
    }
    if let Some(value) = overrides.max_non_empty_lines_config {
        thresholds.max_non_empty_lines_config = value;
    }
    if let Some(value) = overrides.max_size_bytes {
        thresholds.max_size_bytes = value;
    }
    if let Some(value) = overrides.max_import_count {
        thresholds.max_import_count = value;
    }
    if let Some(value) = overrides.max_line_length {
        thresholds.max_line_length = value;
    }
    if let Some(value) = overrides.max_symbol_count_per_file {
        thresholds.max_symbol_count_per_file = value;
    }
    if let Some(value) = overrides.max_ref_count_per_file {
        thresholds.max_ref_count_per_file = value;
    }
    if let Some(value) = overrides.max_module_dep_count_per_file {
        thresholds.max_module_dep_count_per_file = value;
    }
    if let Some(value) = overrides.max_graph_edge_out_count {
        thresholds.max_graph_edge_out_count = value;
    }
}
