use std::collections::BTreeMap;

use crate::model::{QualityCategory, QualitySeverity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuleMetadata {
    pub(crate) severity: QualitySeverity,
    pub(crate) category: QualityCategory,
}

pub(crate) fn default_rule_metadata_map() -> BTreeMap<String, RuleMetadata> {
    known_rule_ids()
        .iter()
        .filter_map(|rule_id| {
            default_rule_metadata(rule_id).map(|metadata| ((*rule_id).to_string(), metadata))
        })
        .collect()
}

pub(crate) fn default_rule_metadata(rule_id: &str) -> Option<RuleMetadata> {
    let metadata = match rule_id {
        "max_size_bytes" => RuleMetadata {
            severity: QualitySeverity::Medium,
            category: QualityCategory::Maintainability,
        },
        "max_non_empty_lines_default"
        | "max_non_empty_lines_test"
        | "max_non_empty_lines_config"
        | "max_import_count"
        | "max_symbol_count_per_file"
        | "max_ref_count_per_file"
        | "max_module_dep_count_per_file"
        | "max_graph_edge_out_count"
        | "max_function_lines"
        | "max_nesting_depth"
        | "max_parameters_per_function"
        | "max_export_count_per_file"
        | "max_class_member_count" => RuleMetadata {
            severity: QualitySeverity::Medium,
            category: QualityCategory::Maintainability,
        },
        "max_cyclomatic_complexity" | "max_cognitive_complexity" => RuleMetadata {
            severity: QualitySeverity::High,
            category: QualityCategory::Maintainability,
        },
        "max_duplicate_block_count" | "max_duplicate_density_bps" => RuleMetadata {
            severity: QualitySeverity::High,
            category: QualityCategory::Maintainability,
        },
        "dead_code_unused_export_candidate" => RuleMetadata {
            severity: QualitySeverity::High,
            category: QualityCategory::Maintainability,
        },
        "max_todo_count_per_file" | "max_line_length" => RuleMetadata {
            severity: QualitySeverity::Low,
            category: QualityCategory::Style,
        },
        "max_fan_in_per_file" | "max_fan_out_per_file" | "hub_module" => RuleMetadata {
            severity: QualitySeverity::High,
            category: QualityCategory::Risk,
        },
        "high_git_churn"
        | "ownership_concentration"
        | "high_change_coupling"
        | "security_smell_shell_exec"
        | "security_smell_path_traversal"
        | "security_smell_raw_sql"
        | "security_smell_unsafe_deserialize"
        | "public_surface_without_tests"
        | "hotspot_without_test_evidence"
        | "integration_entry_without_tests" => RuleMetadata {
            severity: QualitySeverity::High,
            category: QualityCategory::Risk,
        },
        "module_cycle_member"
        | "cross_layer_dependency"
        | "layering_unmatched_zone_dependency"
        | "orphan_module" => RuleMetadata {
            severity: QualitySeverity::High,
            category: QualityCategory::Architecture,
        },
        _ => return None,
    };
    Some(metadata)
}

pub(crate) fn is_known_rule_id(rule_id: &str) -> bool {
    default_rule_metadata(rule_id).is_some()
}

fn known_rule_ids() -> &'static [&'static str] {
    &[
        "max_size_bytes",
        "max_non_empty_lines_default",
        "max_non_empty_lines_test",
        "max_non_empty_lines_config",
        "max_import_count",
        "max_line_length",
        "max_symbol_count_per_file",
        "max_ref_count_per_file",
        "max_module_dep_count_per_file",
        "max_graph_edge_out_count",
        "max_function_lines",
        "max_nesting_depth",
        "max_parameters_per_function",
        "max_export_count_per_file",
        "max_class_member_count",
        "max_todo_count_per_file",
        "max_cyclomatic_complexity",
        "max_cognitive_complexity",
        "max_duplicate_block_count",
        "max_duplicate_density_bps",
        "dead_code_unused_export_candidate",
        "max_fan_in_per_file",
        "max_fan_out_per_file",
        "module_cycle_member",
        "hub_module",
        "cross_layer_dependency",
        "layering_unmatched_zone_dependency",
        "orphan_module",
        "high_git_churn",
        "ownership_concentration",
        "high_change_coupling",
        "security_smell_shell_exec",
        "security_smell_path_traversal",
        "security_smell_raw_sql",
        "security_smell_unsafe_deserialize",
        "public_surface_without_tests",
        "hotspot_without_test_evidence",
        "integration_entry_without_tests",
    ]
}
