use std::collections::BTreeMap;

use anyhow::Result;

use super::super::policy_schema::{
    GitRiskPolicyFile, PathScopePolicyFile, QualityPolicyFile, QualityRuleMetadataOverrideFile,
    QualityScopePolicyFile, QualitySuppressionFile, QualityThresholdOverrides,
    StructuralPolicyFile, TestRiskPolicyFile,
};
use super::super::rule_metadata::RuleMetadata;
use super::{
    GitRiskPolicy, PathMatcher, PathScopePolicy, QualityPolicy, QualityRuleMetadataOverride,
    QualityScopePolicy, QualitySuppressionPolicy, QualityThresholds, StructuralDirection,
    StructuralForbiddenEdge, StructuralPolicy, StructuralZone, TestRiskPolicy,
    default_quality_policy, duplication_policy_from_file,
};

pub(super) fn quality_policy_from_file(parsed: QualityPolicyFile) -> Result<QualityPolicy> {
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
    policy.layering = parsed.layering.map(structural_policy_from_file);
    policy.git_risk = parsed
        .git_risk
        .map(git_risk_policy_from_file)
        .unwrap_or_default();
    policy.test_risk = parsed
        .test_risk
        .map(test_risk_policy_from_file)
        .unwrap_or_default();
    policy.duplication = duplication_policy_from_file(None, None, parsed.duplication)?;
    policy.path_scopes = parsed
        .path_scopes
        .into_iter()
        .map(path_scope_from_file)
        .collect::<Result<Vec<_>>>()?;
    for scope in &policy.path_scopes {
        policy.duplication.extend_from(scope.duplication.clone());
    }
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
        duplication: duplication_policy_from_file(
            Some(parsed.id.as_str()),
            Some(&parsed.paths),
            parsed.duplication,
        )?,
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

pub(super) fn apply_threshold_overrides(
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
    if let Some(value) = overrides.max_cyclomatic_complexity {
        thresholds.max_cyclomatic_complexity = value;
    }
    if let Some(value) = overrides.max_cognitive_complexity {
        thresholds.max_cognitive_complexity = value;
    }
    if let Some(value) = overrides.max_duplicate_block_count {
        thresholds.max_duplicate_block_count = value;
    }
    if let Some(value) = overrides.max_duplicate_density_bps {
        thresholds.max_duplicate_density_bps = value;
    }
    if let Some(value) = overrides.max_public_api_exports_per_file {
        thresholds.max_public_api_exports_per_file = value;
    }
    if let Some(value) = overrides.max_public_reexports_per_file {
        thresholds.max_public_reexports_per_file = value;
    }
    if let Some(value) = overrides.max_public_api_hub_score {
        thresholds.max_public_api_hub_score = value;
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

pub(super) fn apply_rule_metadata_overrides(
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

pub(super) fn git_risk_policy_from_file(parsed: GitRiskPolicyFile) -> GitRiskPolicy {
    GitRiskPolicy {
        enabled: parsed.enabled,
        recent_days: parsed.recent_days,
        min_commits_for_ownership: parsed.min_commits_for_ownership,
        max_recent_commits_per_file: parsed.max_recent_commits_per_file,
        max_recent_churn_lines_per_file: parsed.max_recent_churn_lines_per_file,
        max_primary_author_share_bps: parsed.max_primary_author_share_bps,
        max_cochange_neighbors_per_file: parsed.max_cochange_neighbors_per_file,
    }
}

pub(super) fn test_risk_policy_from_file(parsed: TestRiskPolicyFile) -> TestRiskPolicy {
    TestRiskPolicy {
        enabled: parsed.enabled,
        test_paths: parsed.test_paths,
        nearby_max_directory_distance: parsed.nearby_max_directory_distance,
        entrypoint_globs: parsed.entrypoint_globs,
        hotspot_requires_test_evidence_min_score: parsed.hotspot_requires_test_evidence_min_score,
    }
}
