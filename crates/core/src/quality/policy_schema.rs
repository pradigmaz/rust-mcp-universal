use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::model::{QualityCategory, QualitySeverity};
use crate::quality::is_known_rule_id;

#[path = "policy_schema_duplication.rs"]
mod duplication;

pub(crate) use duplication::{
    DuplicationPathPairFile, DuplicationPolicyFile, DuplicationSuppressionFile,
    validate_duplication_suppressions,
};

pub(crate) const CURRENT_QUALITY_POLICY_VERSION: u32 = 4;

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct QualityPolicyFile {
    #[serde(default = "default_policy_version")]
    pub(crate) version: u32,
    #[serde(default)]
    pub(crate) thresholds: QualityThresholdOverrides,
    #[serde(default)]
    pub(crate) quality_scope: Option<QualityScopePolicyFile>,
    #[serde(default)]
    pub(crate) layering: Option<StructuralPolicyFile>,
    #[serde(default)]
    pub(crate) git_risk: Option<GitRiskPolicyFile>,
    #[serde(default)]
    pub(crate) test_risk: Option<TestRiskPolicyFile>,
    #[serde(default)]
    pub(crate) duplication: Option<DuplicationPolicyFile>,
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
    pub(crate) max_cyclomatic_complexity: Option<i64>,
    pub(crate) max_cognitive_complexity: Option<i64>,
    pub(crate) max_duplicate_block_count: Option<i64>,
    pub(crate) max_duplicate_density_bps: Option<i64>,
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
    #[serde(default)]
    pub(crate) duplication: Option<DuplicationPolicyFile>,
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

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GitRiskPolicyFile {
    #[serde(default = "default_true")]
    pub(crate) enabled: bool,
    #[serde(default = "default_git_recent_days")]
    pub(crate) recent_days: u32,
    #[serde(default = "default_git_min_commits_for_ownership")]
    pub(crate) min_commits_for_ownership: i64,
    #[serde(default = "default_git_max_recent_commits_per_file")]
    pub(crate) max_recent_commits_per_file: i64,
    #[serde(default = "default_git_max_recent_churn_lines_per_file")]
    pub(crate) max_recent_churn_lines_per_file: i64,
    #[serde(default = "default_git_max_primary_author_share_bps")]
    pub(crate) max_primary_author_share_bps: i64,
    #[serde(default = "default_git_max_cochange_neighbors_per_file")]
    pub(crate) max_cochange_neighbors_per_file: i64,
}

impl Default for GitRiskPolicyFile {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            recent_days: default_git_recent_days(),
            min_commits_for_ownership: default_git_min_commits_for_ownership(),
            max_recent_commits_per_file: default_git_max_recent_commits_per_file(),
            max_recent_churn_lines_per_file: default_git_max_recent_churn_lines_per_file(),
            max_primary_author_share_bps: default_git_max_primary_author_share_bps(),
            max_cochange_neighbors_per_file: default_git_max_cochange_neighbors_per_file(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TestRiskPolicyFile {
    #[serde(default = "default_true")]
    pub(crate) enabled: bool,
    #[serde(default = "default_test_paths")]
    pub(crate) test_paths: Vec<String>,
    #[serde(default = "default_nearby_max_directory_distance")]
    pub(crate) nearby_max_directory_distance: usize,
    #[serde(default = "default_entrypoint_globs")]
    pub(crate) entrypoint_globs: Vec<String>,
    #[serde(default = "default_hotspot_requires_test_evidence_min_score")]
    pub(crate) hotspot_requires_test_evidence_min_score: f64,
}

impl Default for TestRiskPolicyFile {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            test_paths: default_test_paths(),
            nearby_max_directory_distance: default_nearby_max_directory_distance(),
            entrypoint_globs: default_entrypoint_globs(),
            hotspot_requires_test_evidence_min_score:
                default_hotspot_requires_test_evidence_min_score(),
        }
    }
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
    Violate,
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
    if let Some(duplication) = parsed.duplication.as_ref() {
        validate_duplication_suppressions(policy_path, None, &duplication.suppressions)?;
    }
    validate_structural_policy(policy_path, parsed.layering.as_ref())?;
    validate_git_risk_policy(policy_path, parsed.git_risk.as_ref())?;
    validate_test_risk_policy(policy_path, parsed.test_risk.as_ref())?;
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
        if let Some(duplication) = scope.duplication.as_ref() {
            validate_duplication_suppressions(
                policy_path,
                Some(scope_id),
                &duplication.suppressions,
            )?;
        }
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
        validate_direction(
            policy_path,
            &zone_ids,
            &direction.from,
            &direction.to,
            "allowed",
        )?;
    }
    for edge in &structural.forbidden_edges {
        validate_direction(policy_path, &zone_ids, &edge.from, &edge.to, "forbidden")?;
    }

    Ok(())
}

fn validate_git_risk_policy(
    policy_path: &Path,
    git_risk: Option<&GitRiskPolicyFile>,
) -> Result<()> {
    let Some(git_risk) = git_risk else {
        return Ok(());
    };
    if git_risk.recent_days == 0 {
        bail!(
            "quality policy `{}` declares git_risk.recent_days as zero",
            policy_path.display()
        );
    }
    if git_risk.min_commits_for_ownership < 1 {
        bail!(
            "quality policy `{}` declares git_risk.min_commits_for_ownership below 1",
            policy_path.display()
        );
    }
    Ok(())
}

fn validate_test_risk_policy(
    policy_path: &Path,
    test_risk: Option<&TestRiskPolicyFile>,
) -> Result<()> {
    let Some(test_risk) = test_risk else {
        return Ok(());
    };
    if test_risk.test_paths.is_empty() {
        bail!(
            "quality policy `{}` declares test_risk without any test_paths",
            policy_path.display()
        );
    }
    if test_risk.entrypoint_globs.is_empty() {
        bail!(
            "quality policy `{}` declares test_risk without any entrypoint_globs",
            policy_path.display()
        );
    }
    if test_risk.hotspot_requires_test_evidence_min_score <= 0.0 {
        bail!(
            "quality policy `{}` declares test_risk.hotspot_requires_test_evidence_min_score <= 0",
            policy_path.display()
        );
    }
    Ok(())
}

const fn default_true() -> bool {
    true
}

const fn default_git_recent_days() -> u32 {
    90
}

const fn default_git_min_commits_for_ownership() -> i64 {
    3
}

const fn default_git_max_recent_commits_per_file() -> i64 {
    12
}

const fn default_git_max_recent_churn_lines_per_file() -> i64 {
    400
}

const fn default_git_max_primary_author_share_bps() -> i64 {
    7_500
}

const fn default_git_max_cochange_neighbors_per_file() -> i64 {
    10
}

fn default_test_paths() -> Vec<String> {
    vec![
        "tests/**".to_string(),
        "**/__tests__/**".to_string(),
        "**/*.test.*".to_string(),
        "**/*_test.*".to_string(),
        "**/*.spec.*".to_string(),
        "**/*_spec.*".to_string(),
        "**/integration/**".to_string(),
        "**/e2e/**".to_string(),
    ]
}

const fn default_nearby_max_directory_distance() -> usize {
    1
}

fn default_entrypoint_globs() -> Vec<String> {
    vec![
        "**/api/**".to_string(),
        "**/routes/**".to_string(),
        "**/router/**".to_string(),
        "**/handlers/**".to_string(),
        "**/controllers/**".to_string(),
        "**/main.*".to_string(),
        "**/app.*".to_string(),
        "**/server.*".to_string(),
    ]
}

const fn default_hotspot_requires_test_evidence_min_score() -> f64 {
    8.0
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
