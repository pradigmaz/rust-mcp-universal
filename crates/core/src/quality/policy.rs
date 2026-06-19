use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::index_scope::IndexScope;
use crate::model::{IndexingOptions, QualitySuppression};

#[path = "policy_duplication.rs"]
mod duplication_policy;
#[path = "policy_defaults.rs"]
mod policy_defaults;
#[path = "policy_file.rs"]
mod policy_file;
#[path = "policy_suppression.rs"]
mod policy_suppression;

use super::policy_schema::{
    GitRiskPolicyFile, QualityThresholdOverrides, StructuralUnmatchedBehavior, TestRiskPolicyFile,
    parse_quality_policy_file,
};
use super::rule_metadata::RuleMetadata;
pub(crate) use duplication_policy::{DuplicationPolicy, duplication_policy_from_file};
pub(crate) use policy_defaults::default_quality_policy;
use policy_file::{
    apply_rule_metadata_overrides, apply_threshold_overrides, git_risk_policy_from_file,
    quality_policy_from_file, test_risk_policy_from_file,
};
use policy_suppression::{QualitySuppressionMatch, matching_suppressions, suppressions_for_rule};

#[derive(Debug, Clone, Default)]
pub(crate) struct QualityPolicy {
    pub(crate) thresholds: QualityThresholds,
    pub(crate) quality_scope: QualityScopePolicy,
    pub(crate) layering: Option<StructuralPolicy>,
    pub(crate) git_risk: GitRiskPolicy,
    pub(crate) test_risk: TestRiskPolicy,
    pub(crate) duplication: DuplicationPolicy,
    pub(crate) rule_metadata: BTreeMap<String, RuleMetadata>,
    pub(crate) path_scopes: Vec<PathScopePolicy>,
    pub(crate) suppressions: Vec<QualitySuppressionPolicy>,
}

#[derive(Debug, Clone)]
pub(crate) struct EffectiveQualityPolicy {
    pub(crate) thresholds: QualityThresholds,
    pub(crate) git_risk: GitRiskPolicy,
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
        suppressions_for_rule(&self.suppression_matches, rule_id)
    }
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
    pub(crate) max_cyclomatic_complexity: i64,
    pub(crate) max_cognitive_complexity: i64,
    pub(crate) max_duplicate_block_count: i64,
    pub(crate) max_duplicate_density_bps: i64,
    pub(crate) max_public_api_exports_per_file: i64,
    pub(crate) max_public_reexports_per_file: i64,
    pub(crate) max_public_api_hub_score: i64,
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

#[derive(Debug, Clone)]
pub(crate) struct GitRiskPolicy {
    pub(crate) enabled: bool,
    pub(crate) recent_days: u32,
    pub(crate) min_commits_for_ownership: i64,
    pub(crate) max_recent_commits_per_file: i64,
    pub(crate) max_recent_churn_lines_per_file: i64,
    pub(crate) max_primary_author_share_bps: i64,
    pub(crate) max_cochange_neighbors_per_file: i64,
}

impl Default for GitRiskPolicy {
    fn default() -> Self {
        git_risk_policy_from_file(GitRiskPolicyFile::default())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TestRiskPolicy {
    pub(crate) enabled: bool,
    pub(crate) test_paths: Vec<String>,
    pub(crate) nearby_max_directory_distance: usize,
    pub(crate) entrypoint_globs: Vec<String>,
    pub(crate) hotspot_requires_test_evidence_min_score: f64,
}

impl Default for TestRiskPolicy {
    fn default() -> Self {
        test_risk_policy_from_file(TestRiskPolicyFile::default())
    }
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
    pub(crate) duplication: DuplicationPolicy,
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
pub(crate) struct PathMatcher {
    scope: IndexScope,
}

impl PathMatcher {
    pub(crate) fn new(patterns: &[String]) -> Result<Self> {
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

    pub(crate) fn matches(&self, rel_path: &str) -> bool {
        self.scope.allows(rel_path)
    }
}

impl QualityPolicy {
    pub(crate) fn duplication_suppressions_for_class(
        &self,
        class: &crate::quality::duplication::artifact::DuplicationCloneClass,
    ) -> Vec<QualitySuppression> {
        self.duplication.suppressions_for_class(class)
    }

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
            git_risk: self.git_risk.clone(),
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

pub(crate) fn load_quality_policy_digest(project_root: &Path) -> Result<String> {
    const QUALITY_ENGINE_DIGEST_SALT: &str = "quality-engine-v5-wave3-structural-risk";
    let policy_path = project_root.join("rmu-quality-policy.json");
    if !policy_path.exists() {
        return Ok(crate::utils::hash_bytes(
            format!(
                "quality-policy-default-v{}|{}",
                super::policy_schema::CURRENT_QUALITY_POLICY_VERSION,
                QUALITY_ENGINE_DIGEST_SALT
            )
            .as_bytes(),
        ));
    }
    let raw = fs::read(&policy_path)
        .with_context(|| format!("failed to read quality policy `{}`", policy_path.display()))?;
    let mut salted = raw;
    salted.extend_from_slice(b"|");
    salted.extend_from_slice(QUALITY_ENGINE_DIGEST_SALT.as_bytes());
    Ok(crate::utils::hash_bytes(&salted))
}
