use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::index_scope::IndexScope;
use crate::model::{IndexingOptions, QualitySuppression};

use super::policy_schema::{
    PathScopePolicyFile, QualityPolicyFile, QualityRuleMetadataOverrideFile,
    QualityScopePolicyFile, QualitySuppressionFile, QualityThresholdOverrides,
    StructuralPolicyFile, StructuralUnmatchedBehavior, parse_quality_policy_file,
};
use super::rule_metadata::{RuleMetadata, default_rule_metadata_map};

#[derive(Debug, Clone, Default)]
pub(crate) struct QualityPolicy {
    pub(crate) thresholds: QualityThresholds,
    pub(crate) quality_scope: QualityScopePolicy,
    pub(crate) structural: Option<StructuralPolicy>,
    pub(crate) rule_metadata: BTreeMap<String, RuleMetadata>,
    pub(crate) path_scopes: Vec<PathScopePolicy>,
    pub(crate) suppressions: Vec<QualitySuppressionPolicy>,
}

#[derive(Debug, Clone)]
pub(crate) struct EffectiveQualityPolicy {
    pub(crate) thresholds: QualityThresholds,
    rule_metadata: BTreeMap<String, RuleMetadata>,
    suppression_matches: Vec<QualitySuppressionMatch>,
}

impl EffectiveQualityPolicy {
    pub(crate) fn metadata_for_rule(&self, rule_id: &str) -> RuleMetadata {
        self.rule_metadata
            .get(rule_id)
            .copied()
            .unwrap_or_else(|| panic!("missing quality metadata for known rule `{rule_id}`"))
    }

    pub(crate) fn suppressions_for_rule(&self, rule_id: &str) -> Vec<QualitySuppression> {
        self.suppression_matches
            .iter()
            .filter(|suppression| suppression.rule_ids.contains(rule_id))
            .map(|suppression| QualitySuppression {
                suppression_id: suppression.suppression_id.clone(),
                reason: suppression.reason.clone(),
                scope_id: suppression.scope_id.clone(),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct QualitySuppressionMatch {
    pub(crate) suppression_id: String,
    pub(crate) reason: String,
    pub(crate) scope_id: Option<String>,
    rule_ids: BTreeSet<String>,
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
    pub(crate) max_function_lines: i64,
    pub(crate) max_nesting_depth: i64,
    pub(crate) max_parameters_per_function: i64,
    pub(crate) max_export_count_per_file: i64,
    pub(crate) max_class_member_count: i64,
    pub(crate) max_todo_count_per_file: i64,
    pub(crate) max_fan_in_per_file: i64,
    pub(crate) max_fan_out_per_file: i64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct QualityScopePolicy {
    pub(crate) exclude_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct StructuralPolicy {
    pub(crate) zones: Vec<StructuralZone>,
    pub(crate) allowed_directions: Vec<StructuralDirection>,
    pub(crate) forbidden_edges: Vec<StructuralForbiddenEdge>,
    pub(crate) unmatched_behavior: StructuralUnmatchedBehavior,
}

impl StructuralPolicy {
    pub(crate) fn has_zones(&self) -> bool {
        !self.zones.is_empty()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StructuralZone {
    pub(crate) id: String,
    pub(crate) paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct StructuralDirection {
    pub(crate) from: String,
    pub(crate) to: String,
}

#[derive(Debug, Clone)]
pub(crate) struct StructuralForbiddenEdge {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PathScopePolicy {
    matcher: PathMatcher,
    pub(crate) thresholds: QualityThresholdOverrides,
    pub(crate) rule_overrides: BTreeMap<String, QualityRuleMetadataOverride>,
    pub(crate) suppressions: Vec<QualitySuppressionPolicy>,
}

#[derive(Debug, Clone)]
pub(crate) struct QualitySuppressionPolicy {
    pub(crate) suppression_id: String,
    pub(crate) reason: String,
    pub(crate) scope_id: Option<String>,
    matcher: PathMatcher,
    rule_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct QualityRuleMetadataOverride {
    pub(crate) severity: Option<crate::model::QualitySeverity>,
    pub(crate) category: Option<crate::model::QualityCategory>,
}

#[derive(Debug, Clone)]
struct PathMatcher {
    scope: IndexScope,
}

impl PathMatcher {
    fn new(patterns: &[String]) -> Result<Self> {
        Ok(Self {
            scope: IndexScope::new(&IndexingOptions {
                profile: None,
                changed_since: None,
                changed_since_commit: None,
                include_paths: patterns.to_vec(),
                exclude_paths: Vec::new(),
                reindex: false,
            })?,
        })
    }

    fn matches(&self, rel_path: &str) -> bool {
        self.scope.allows(rel_path)
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
            max_function_lines: super::metrics::MAX_FUNCTION_LINES,
            max_nesting_depth: super::metrics::MAX_NESTING_DEPTH,
            max_parameters_per_function: super::metrics::MAX_PARAMETERS_PER_FUNCTION,
            max_export_count_per_file: super::metrics::MAX_EXPORT_COUNT_PER_FILE,
            max_class_member_count: super::metrics::MAX_CLASS_MEMBER_COUNT,
            max_todo_count_per_file: super::metrics::MAX_TODO_COUNT_PER_FILE,
            max_fan_in_per_file: super::metrics::MAX_FAN_IN_PER_FILE,
            max_fan_out_per_file: super::metrics::MAX_FAN_OUT_PER_FILE,
        },
        quality_scope: QualityScopePolicy::default(),
        structural: None,
        rule_metadata: default_rule_metadata_map(),
        path_scopes: Vec::new(),
        suppressions: Vec::new(),
    }
}

impl QualityPolicy {
    pub(crate) fn effective_for_path(&self, rel_path: &str) -> EffectiveQualityPolicy {
        let mut thresholds = self.thresholds.clone();
        let mut rule_metadata = self.rule_metadata.clone();
        let mut suppression_matches = matching_suppressions(&self.suppressions, rel_path);

        for scope in &self.path_scopes {
            if !scope.matcher.matches(rel_path) {
                continue;
            }
            apply_threshold_overrides(&mut thresholds, scope.thresholds.clone());
            apply_rule_metadata_overrides(&mut rule_metadata, &scope.rule_overrides);
            suppression_matches.extend(matching_suppressions(&scope.suppressions, rel_path));
        }

        EffectiveQualityPolicy {
            thresholds,
            rule_metadata,
            suppression_matches,
        }
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
    let parsed = parse_quality_policy_file(content, &policy_path)?;

    quality_policy_from_file(parsed)
}

fn quality_policy_from_file(parsed: QualityPolicyFile) -> Result<QualityPolicy> {
    let mut policy = default_quality_policy();
    apply_threshold_overrides(&mut policy.thresholds, parsed.thresholds);
    apply_quality_scope(&mut policy.quality_scope, parsed.quality_scope);
    apply_rule_metadata_overrides(
        &mut policy.rule_metadata,
        &parsed
            .rule_overrides
            .into_iter()
            .map(|(rule_id, override_file)| {
                (rule_id, rule_metadata_override_from_file(override_file))
            })
            .collect(),
    );
    policy.structural = parsed.structural.map(structural_policy_from_file);
    policy.path_scopes = parsed
        .path_scopes
        .into_iter()
        .map(path_scope_from_file)
        .collect::<Result<Vec<_>>>()?;
    policy.suppressions = parsed
        .suppressions
        .into_iter()
        .map(|suppression| suppression_from_file(None, suppression))
        .collect::<Result<Vec<_>>>()?;
    Ok(policy)
}

fn path_scope_from_file(parsed: PathScopePolicyFile) -> Result<PathScopePolicy> {
    Ok(PathScopePolicy {
        matcher: PathMatcher::new(&parsed.paths)?,
        thresholds: parsed.thresholds,
        rule_overrides: parsed
            .rule_overrides
            .into_iter()
            .map(|(rule_id, override_file)| {
                (rule_id, rule_metadata_override_from_file(override_file))
            })
            .collect(),
        suppressions: parsed
            .suppressions
            .into_iter()
            .map(|suppression| suppression_from_file(Some(parsed.id.as_str()), suppression))
            .collect::<Result<Vec<_>>>()?,
    })
}

fn suppression_from_file(
    scope_id: Option<&str>,
    parsed: QualitySuppressionFile,
) -> Result<QualitySuppressionPolicy> {
    Ok(QualitySuppressionPolicy {
        suppression_id: parsed.id,
        reason: parsed.reason,
        scope_id: scope_id.map(str::to_string),
        matcher: PathMatcher::new(&parsed.paths)?,
        rule_ids: parsed.rule_ids.into_iter().collect(),
    })
}

fn matching_suppressions(
    suppressions: &[QualitySuppressionPolicy],
    rel_path: &str,
) -> Vec<QualitySuppressionMatch> {
    suppressions
        .iter()
        .filter(|suppression| suppression.matcher.matches(rel_path))
        .map(|suppression| QualitySuppressionMatch {
            suppression_id: suppression.suppression_id.clone(),
            reason: suppression.reason.clone(),
            scope_id: suppression.scope_id.clone(),
            rule_ids: suppression.rule_ids.clone(),
        })
        .collect()
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
    if let Some(value) = overrides.max_function_lines {
        thresholds.max_function_lines = value;
    }
    if let Some(value) = overrides.max_nesting_depth {
        thresholds.max_nesting_depth = value;
    }
    if let Some(value) = overrides.max_parameters_per_function {
        thresholds.max_parameters_per_function = value;
    }
    if let Some(value) = overrides.max_export_count_per_file {
        thresholds.max_export_count_per_file = value;
    }
    if let Some(value) = overrides.max_class_member_count {
        thresholds.max_class_member_count = value;
    }
    if let Some(value) = overrides.max_todo_count_per_file {
        thresholds.max_todo_count_per_file = value;
    }
    if let Some(value) = overrides.max_fan_in_per_file {
        thresholds.max_fan_in_per_file = value;
    }
    if let Some(value) = overrides.max_fan_out_per_file {
        thresholds.max_fan_out_per_file = value;
    }
}

fn apply_quality_scope(scope: &mut QualityScopePolicy, overrides: Option<QualityScopePolicyFile>) {
    let Some(overrides) = overrides else {
        return;
    };
    scope.exclude_paths = overrides.exclude_paths;
}

fn rule_metadata_override_from_file(
    parsed: QualityRuleMetadataOverrideFile,
) -> QualityRuleMetadataOverride {
    QualityRuleMetadataOverride {
        severity: parsed.severity,
        category: parsed.category,
    }
}

fn apply_rule_metadata_overrides(
    rule_metadata: &mut BTreeMap<String, RuleMetadata>,
    overrides: &BTreeMap<String, QualityRuleMetadataOverride>,
) {
    for (rule_id, override_value) in overrides {
        if let Some(metadata) = rule_metadata.get_mut(rule_id) {
            if let Some(severity) = override_value.severity {
                metadata.severity = severity;
            }
            if let Some(category) = override_value.category {
                metadata.category = category;
            }
        }
    }
}

fn structural_policy_from_file(parsed: StructuralPolicyFile) -> StructuralPolicy {
    StructuralPolicy {
        zones: parsed
            .zones
            .into_iter()
            .map(|zone| StructuralZone {
                id: zone.id,
                paths: zone.paths,
            })
            .collect(),
        allowed_directions: parsed
            .allowed_directions
            .into_iter()
            .map(|direction| StructuralDirection {
                from: direction.from,
                to: direction.to,
            })
            .collect(),
        forbidden_edges: parsed
            .forbidden_edges
            .into_iter()
            .map(|edge| StructuralForbiddenEdge {
                from: edge.from,
                to: edge.to,
                reason: edge.reason,
            })
            .collect(),
        unmatched_behavior: parsed.unmatched_behavior,
    }
}
