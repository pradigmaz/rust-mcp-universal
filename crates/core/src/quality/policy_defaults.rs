use super::super::rule_metadata::default_rule_metadata_map;
use super::{
    DuplicationPolicy, GitRiskPolicy, QualityPolicy, QualityScopePolicy, QualityThresholds,
    TestRiskPolicy,
};

pub(crate) fn default_quality_policy() -> QualityPolicy {
    QualityPolicy {
        thresholds: QualityThresholds {
            max_non_empty_lines_default: super::super::metrics::MAX_NON_EMPTY_LINES_DEFAULT,
            max_non_empty_lines_test: super::super::metrics::MAX_NON_EMPTY_LINES_TEST,
            max_non_empty_lines_config: super::super::metrics::MAX_NON_EMPTY_LINES_CONFIG,
            max_size_bytes: super::super::metrics::MAX_SIZE_BYTES,
            max_import_count: super::super::metrics::MAX_IMPORT_COUNT,
            max_line_length: super::super::metrics::MAX_LINE_LENGTH,
            max_symbol_count_per_file: super::super::metrics::MAX_SYMBOL_COUNT_PER_FILE,
            max_ref_count_per_file: super::super::metrics::MAX_REF_COUNT_PER_FILE,
            max_module_dep_count_per_file: super::super::metrics::MAX_MODULE_DEP_COUNT_PER_FILE,
            max_graph_edge_out_count: super::super::metrics::MAX_GRAPH_EDGE_OUT_COUNT,
            max_function_lines: super::super::metrics::MAX_FUNCTION_LINES,
            max_nesting_depth: super::super::metrics::MAX_NESTING_DEPTH,
            max_parameters_per_function: super::super::metrics::MAX_PARAMETERS_PER_FUNCTION,
            max_export_count_per_file: super::super::metrics::MAX_EXPORT_COUNT_PER_FILE,
            max_class_member_count: super::super::metrics::MAX_CLASS_MEMBER_COUNT,
            max_todo_count_per_file: super::super::metrics::MAX_TODO_COUNT_PER_FILE,
            max_fan_in_per_file: super::super::metrics::MAX_FAN_IN_PER_FILE,
            max_fan_out_per_file: super::super::metrics::MAX_FAN_OUT_PER_FILE,
            max_cyclomatic_complexity: super::super::metrics::MAX_CYCLOMATIC_COMPLEXITY,
            max_cognitive_complexity: super::super::metrics::MAX_COGNITIVE_COMPLEXITY,
            max_duplicate_block_count: super::super::metrics::MAX_DUPLICATE_BLOCK_COUNT,
            max_duplicate_density_bps: super::super::metrics::MAX_DUPLICATE_DENSITY_BPS,
            max_public_api_exports_per_file: super::super::metrics::MAX_PUBLIC_API_EXPORTS_PER_FILE,
            max_public_reexports_per_file: super::super::metrics::MAX_PUBLIC_REEXPORTS_PER_FILE,
            max_public_api_hub_score: super::super::metrics::MAX_PUBLIC_API_HUB_SCORE,
        },
        quality_scope: QualityScopePolicy::default(),
        layering: None,
        git_risk: GitRiskPolicy::default(),
        test_risk: TestRiskPolicy::default(),
        duplication: DuplicationPolicy::default(),
        rule_metadata: default_rule_metadata_map(),
        path_scopes: Vec::new(),
        suppressions: Vec::new(),
    }
}
