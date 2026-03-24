use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::model::{QualityCategory, QualitySeverity};
use crate::quality::is_known_rule_id;

pub(crate) const CURRENT_QUALITY_POLICY_VERSION: u32 = 2;

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct QualityPolicyFile {
    #[serde(default = "default_policy_version")]
    pub(crate) version: u32,
    #[serde(default)]
    pub(crate) thresholds: QualityThresholdOverrides,
    #[serde(default)]
    pub(crate) quality_scope: Option<QualityScopePolicyFile>,
    #[serde(default)]
    pub(crate) structural: Option<StructuralPolicyFile>,
    #[serde(default)]
    pub(crate) rule_overrides: BTreeMap<String, QualityRuleMetadataOverrideFile>,
    #[serde(default)]
    pub(crate) path_scopes: Vec<PathScopePolicyFile>,
    #[serde(default)]
    pub(crate) suppressions: Vec<QualitySuppressionFile>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct QualityThresholdOverrides {
    pub(crate) max_non_empty_lines_default: Option<i64>,
    pub(crate) max_non_empty_lines_test: Option<i64>,
    pub(crate) max_non_empty_lines_config: Option<i64>,
    pub(crate) max_size_bytes: Option<i64>,
    pub(crate) max_import_count: Option<i64>,
    pub(crate) max_line_length: Option<i64>,
    pub(crate) max_symbol_count_per_file: Option<i64>,
    pub(crate) max_ref_count_per_file: Option<i64>,
    pub(crate) max_module_dep_count_per_file: Option<i64>,
    pub(crate) max_graph_edge_out_count: Option<i64>,
    pub(crate) max_function_lines: Option<i64>,
    pub(crate) max_nesting_depth: Option<i64>,
    pub(crate) max_parameters_per_function: Option<i64>,
    pub(crate) max_export_count_per_file: Option<i64>,
    pub(crate) max_class_member_count: Option<i64>,
    pub(crate) max_todo_count_per_file: Option<i64>,
    pub(crate) max_fan_in_per_file: Option<i64>,
    pub(crate) max_fan_out_per_file: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct QualityScopePolicyFile {
    #[serde(default)]
    pub(crate) exclude_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct QualityRuleMetadataOverrideFile {
    pub(crate) severity: Option<QualitySeverity>,
    pub(crate) category: Option<QualityCategory>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct QualitySuppressionFile {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) rule_ids: Vec<String>,
    #[serde(default)]
    pub(crate) paths: Vec<String>,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct PathScopePolicyFile {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) paths: Vec<String>,
    #[serde(default)]
    pub(crate) thresholds: QualityThresholdOverrides,
    #[serde(default)]
    pub(crate) rule_overrides: BTreeMap<String, QualityRuleMetadataOverrideFile>,
    #[serde(default)]
    pub(crate) suppressions: Vec<QualitySuppressionFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StructuralPolicyFile {
    #[serde(default)]
    pub(crate) zones: Vec<StructuralZoneFile>,
    #[serde(default)]
    pub(crate) allowed_directions: Vec<StructuralDirectionFile>,
    #[serde(default)]
    pub(crate) forbidden_edges: Vec<StructuralForbiddenEdgeFile>,
    #[serde(default)]
    pub(crate) unmatched_behavior: StructuralUnmatchedBehavior,
}

impl Default for StructuralPolicyFile {
    fn default() -> Self {
        Self {
            zones: Vec::new(),
            allowed_directions: Vec::new(),
            forbidden_edges: Vec::new(),
            unmatched_behavior: StructuralUnmatchedBehavior::Allow,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StructuralZoneFile {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StructuralDirectionFile {
    pub(crate) from: String,
    pub(crate) to: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StructuralForbiddenEdgeFile {
    pub(crate) from: String,
    pub(crate) to: String,
    #[serde(default)]
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StructuralUnmatchedBehavior {
    #[default]
    Allow,
    Ignore,
}

pub(crate) fn parse_quality_policy_file(
    content: &str,
    policy_path: &Path,
) -> Result<QualityPolicyFile> {
    let parsed: QualityPolicyFile = serde_json::from_str(content)
        .with_context(|| format!("failed to parse quality policy `{}`", policy_path.display()))?;
    if parsed.version != CURRENT_QUALITY_POLICY_VERSION {
        bail!(
            "quality policy `{}` declares unsupported version `{}`",
            policy_path.display(),
            parsed.version
        );
    }
    validate_rule_overrides(policy_path, &parsed.rule_overrides)?;
    validate_path_scopes(policy_path, &parsed.path_scopes)?;
    validate_suppressions(policy_path, None, &parsed.suppressions)?;
    validate_structural_policy(policy_path, parsed.structural.as_ref())?;
    Ok(parsed)
}

fn validate_rule_overrides(
    policy_path: &Path,
    overrides: &BTreeMap<String, QualityRuleMetadataOverrideFile>,
) -> Result<()> {
    for rule_id in overrides.keys() {
        if !is_known_rule_id(rule_id) {
            bail!(
                "quality policy `{}` references unknown rule `{rule_id}` in rule_overrides",
                policy_path.display()
            );
        }
    }
    Ok(())
}

fn validate_path_scopes(policy_path: &Path, scopes: &[PathScopePolicyFile]) -> Result<()> {
    let mut ids = BTreeSet::new();
    for scope in scopes {
        let scope_id = scope.id.trim();
        if scope_id.is_empty() {
            bail!(
                "quality policy `{}` contains a path scope with an empty `id`",
                policy_path.display()
            );
        }
        if !ids.insert(scope_id.to_string()) {
            bail!(
                "quality policy `{}` declares duplicate path scope `{scope_id}`",
                policy_path.display()
            );
        }
        if scope.paths.is_empty() {
            bail!(
                "quality policy `{}` declares path scope `{scope_id}` without any paths",
                policy_path.display()
            );
        }
        validate_rule_overrides(policy_path, &scope.rule_overrides)?;
        validate_suppressions(policy_path, Some(scope_id), &scope.suppressions)?;
    }
    Ok(())
}

fn validate_suppressions(
    policy_path: &Path,
    scope_id: Option<&str>,
    suppressions: &[QualitySuppressionFile],
) -> Result<()> {
    let mut ids = BTreeSet::new();
    for suppression in suppressions {
        let id = suppression.id.trim();
        if id.is_empty() {
            bail!(
                "quality policy `{}` contains a suppression with an empty `id`",
                policy_path.display()
            );
        }
        if !ids.insert(id.to_string()) {
            match scope_id {
                Some(scope_id) => bail!(
                    "quality policy `{}` declares duplicate suppression `{id}` inside path scope `{scope_id}`",
                    policy_path.display()
                ),
                None => bail!(
                    "quality policy `{}` declares duplicate suppression `{id}`",
                    policy_path.display()
                ),
            }
        }
        if suppression.rule_ids.is_empty() {
            bail!(
                "quality policy `{}` declares suppression `{id}` without any rule_ids",
                policy_path.display()
            );
        }
        if suppression.paths.is_empty() {
            bail!(
                "quality policy `{}` declares suppression `{id}` without any paths",
                policy_path.display()
            );
        }
        if suppression.reason.trim().is_empty() {
            bail!(
                "quality policy `{}` declares suppression `{id}` without a reason",
                policy_path.display()
            );
        }
        for rule_id in &suppression.rule_ids {
            if !is_known_rule_id(rule_id) {
                bail!(
                    "quality policy `{}` suppression `{id}` references unknown rule `{rule_id}`",
                    policy_path.display()
                );
            }
        }
    }
    Ok(())
}

fn validate_structural_policy(
    policy_path: &Path,
    structural: Option<&StructuralPolicyFile>,
) -> Result<()> {
    let Some(structural) = structural else {
        return Ok(());
    };

    let mut zone_ids = BTreeSet::new();
    let mut pattern_owners = BTreeMap::<String, String>::new();
    for zone in &structural.zones {
        let zone_id = zone.id.trim();
        if zone_id.is_empty() {
            bail!(
                "quality policy `{}` contains a structural zone with an empty `id`",
                policy_path.display()
            );
        }
        if !zone_ids.insert(zone_id.to_string()) {
            bail!(
                "quality policy `{}` declares duplicate structural zone `{zone_id}`",
                policy_path.display()
            );
        }
        if zone.paths.is_empty() {
            bail!(
                "quality policy `{}` declares structural zone `{zone_id}` without any paths",
                policy_path.display()
            );
        }
        for path in &zone.paths {
            let normalized = path.trim().replace('\\', "/");
            if normalized.is_empty() {
                bail!(
                    "quality policy `{}` declares an empty path inside structural zone `{zone_id}`",
                    policy_path.display()
                );
            }
            if let Some(existing_zone) = pattern_owners.get(&normalized) {
                bail!(
                    "quality policy `{}` declares overlapping structural zone pattern `{normalized}` in `{existing_zone}` and `{zone_id}`",
                    policy_path.display()
                );
            }
            pattern_owners.insert(normalized, zone_id.to_string());
        }
    }

    for direction in &structural.allowed_directions {
        validate_direction(policy_path, &zone_ids, &direction.from, &direction.to, "allowed")?;
    }
    for edge in &structural.forbidden_edges {
        validate_direction(policy_path, &zone_ids, &edge.from, &edge.to, "forbidden")?;
    }

    Ok(())
}

fn validate_direction(
    policy_path: &Path,
    zone_ids: &BTreeSet<String>,
    from: &str,
    to: &str,
    label: &str,
) -> Result<()> {
    if !zone_ids.contains(from) {
        bail!(
            "quality policy `{}` references unknown structural zone `{from}` in {label} direction",
            policy_path.display()
        );
    }
    if !zone_ids.contains(to) {
        bail!(
            "quality policy `{}` references unknown structural zone `{to}` in {label} direction",
            policy_path.display()
        );
    }
    Ok(())
}

const fn default_policy_version() -> u32 {
    CURRENT_QUALITY_POLICY_VERSION
}
