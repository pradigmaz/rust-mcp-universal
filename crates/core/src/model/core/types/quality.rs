use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QualityStatus {
    #[default]
    Ready,
    Stale,
    Degraded,
    Unavailable,
}

impl QualityStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Stale => "stale",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ready" => Some(Self::Ready),
            "stale" => Some(Self::Stale),
            "degraded" => Some(Self::Degraded),
            "unavailable" => Some(Self::Unavailable),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityLocation {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualitySource {
    Ast,
    ParserLight,
    Heuristic,
    Graph,
    Git,
    Test,
    Duplication,
}

impl QualitySource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ast => "ast",
            Self::ParserLight => "parser_light",
            Self::Heuristic => "heuristic",
            Self::Graph => "graph",
            Self::Git => "git",
            Self::Test => "test",
            Self::Duplication => "duplication",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ast" => Some(Self::Ast),
            "parser_light" => Some(Self::ParserLight),
            "heuristic" => Some(Self::Heuristic),
            "graph" => Some(Self::Graph),
            "git" => Some(Self::Git),
            "test" => Some(Self::Test),
            "duplication" => Some(Self::Duplication),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualitySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl QualitySeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityCategory {
    Style,
    Maintainability,
    Risk,
    Performance,
    Architecture,
}

impl QualityCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Style => "style",
            Self::Maintainability => "maintainability",
            Self::Risk => "risk",
            Self::Performance => "performance",
            Self::Architecture => "architecture",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "style" => Some(Self::Style),
            "maintainability" => Some(Self::Maintainability),
            "risk" => Some(Self::Risk),
            "performance" => Some(Self::Performance),
            "architecture" => Some(Self::Architecture),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualitySuppression {
    pub suppression_id: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QualityMode {
    Indexed,
    QualityOnlyOversize,
}

impl QualityMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Indexed => "indexed",
            Self::QualityOnlyOversize => "quality-only-oversize",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "indexed" => Some(Self::Indexed),
            "quality-only-oversize" => Some(Self::QualityOnlyOversize),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceQualityTopRule {
    pub rule_id: String,
    pub files: usize,
    pub violations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceQualityTopMetric {
    pub metric_id: String,
    pub files: usize,
    pub max_value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceQualitySummary {
    pub ruleset_id: String,
    pub status: QualityStatus,
    pub evaluated_files: usize,
    pub violating_files: usize,
    pub total_violations: usize,
    #[serde(default)]
    pub suppressed_violations: usize,
    pub top_rules: Vec<WorkspaceQualityTopRule>,
    pub top_metrics: Vec<WorkspaceQualityTopMetric>,
    #[serde(default)]
    pub severity_breakdown: Vec<WorkspaceQualitySeverityCount>,
    #[serde(default)]
    pub category_breakdown: Vec<WorkspaceQualityCategoryCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityViolationEntry {
    pub rule_id: String,
    pub actual_value: i64,
    pub threshold_value: i64,
    pub message: String,
    pub severity: QualitySeverity,
    pub category: QualityCategory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<QualityLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<QualitySource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppressedQualityViolationEntry {
    pub violation: QualityViolationEntry,
    #[serde(default)]
    pub suppressions: Vec<QualitySuppression>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetricValue {
    pub metric_id: String,
    pub metric_value: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<QualityLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<QualitySource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QualityRiskScoreWeights {
    pub violation_count: f64,
    pub severity: f64,
    pub fan_in: f64,
    pub fan_out: f64,
    pub size: f64,
    pub nesting: f64,
    pub function_length: f64,
    pub complexity: f64,
    #[serde(default)]
    pub layering: f64,
    #[serde(default)]
    pub git_risk: f64,
    #[serde(default)]
    pub test_risk: f64,
    #[serde(default)]
    pub duplication: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct QualityRiskScoreComponents {
    pub violation_count: f64,
    pub severity: f64,
    pub fan_in: f64,
    pub fan_out: f64,
    pub size: f64,
    pub nesting: f64,
    pub function_length: f64,
    pub complexity: f64,
    #[serde(default)]
    pub layering: f64,
    #[serde(default)]
    pub git_risk: f64,
    #[serde(default)]
    pub test_risk: f64,
    #[serde(default)]
    pub duplication: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QualityRiskScoreBreakdown {
    pub score: f64,
    pub components: QualityRiskScoreComponents,
    pub weights: QualityRiskScoreWeights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolationFileHit {
    pub path: String,
    pub language: String,
    pub size_bytes: i64,
    pub total_lines: Option<i64>,
    pub non_empty_lines: Option<i64>,
    pub import_count: Option<i64>,
    pub quality_mode: QualityMode,
    pub violations: Vec<QualityViolationEntry>,
    #[serde(default)]
    pub metrics: Vec<QualityMetricValue>,
    #[serde(default)]
    pub suppressed_violations: Vec<SuppressedQualityViolationEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_score: Option<QualityRiskScoreBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceQualitySeverityCount {
    pub severity: QualitySeverity,
    pub files: usize,
    pub violations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceQualityCategoryCount {
    pub category: QualityCategory,
    pub files: usize,
    pub violations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolationsSummary {
    pub ruleset_id: String,
    pub status: QualityStatus,
    pub evaluated_files: usize,
    pub violating_files: usize,
    pub total_violations: usize,
    #[serde(default)]
    pub suppressed_violations: usize,
    #[serde(default)]
    pub severity_breakdown: Vec<WorkspaceQualitySeverityCount>,
    #[serde(default)]
    pub category_breakdown: Vec<WorkspaceQualityCategoryCount>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuleViolationsSortBy {
    #[default]
    ViolationCount,
    SizeBytes,
    NonEmptyLines,
    MetricValue,
}

impl RuleViolationsSortBy {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "violation_count" => Some(Self::ViolationCount),
            "size_bytes" => Some(Self::SizeBytes),
            "non_empty_lines" => Some(Self::NonEmptyLines),
            "metric_value" => Some(Self::MetricValue),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolationsOptions {
    pub limit: usize,
    #[serde(default)]
    pub path_prefix: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub rule_ids: Vec<String>,
    #[serde(default)]
    pub metric_ids: Vec<String>,
    #[serde(default)]
    pub sort_metric_id: Option<String>,
    #[serde(default)]
    pub sort_by: RuleViolationsSortBy,
}

impl Default for RuleViolationsOptions {
    fn default() -> Self {
        Self {
            limit: 20,
            path_prefix: None,
            language: None,
            rule_ids: Vec::new(),
            metric_ids: Vec::new(),
            sort_metric_id: None,
            sort_by: RuleViolationsSortBy::ViolationCount,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolationsResult {
    pub summary: RuleViolationsSummary,
    pub hits: Vec<RuleViolationFileHit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QualityHotspotAggregation {
    #[default]
    File,
    Directory,
    Module,
}

impl QualityHotspotAggregation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
            Self::Module => "module",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "file" => Some(Self::File),
            "directory" => Some(Self::Directory),
            "module" => Some(Self::Module),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QualityHotspotsSortBy {
    #[default]
    HotspotScore,
    RiskScoreDelta,
    NewViolations,
}

impl QualityHotspotsSortBy {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "hotspot_score" => Some(Self::HotspotScore),
            "risk_score_delta" => Some(Self::RiskScoreDelta),
            "new_violations" => Some(Self::NewViolations),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityHotspotRuleCount {
    pub rule_id: String,
    pub violations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityHotspotStructuralSignals {
    pub module_cycle_member: usize,
    pub hub_module: usize,
    pub cross_layer_dependency: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityDeltaSummary {
    pub new_violations: usize,
    pub resolved_violations: usize,
    pub risk_score_delta: f64,
    pub hotspot_score_delta: f64,
    pub new_hotspot: bool,
    pub regressed_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityHotspotBucket {
    pub bucket_id: String,
    pub hotspot_score: f64,
    pub active_violation_count: usize,
    #[serde(default)]
    pub suppressed_violation_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_score: Option<QualityRiskScoreBreakdown>,
    #[serde(default)]
    pub rule_counts: Vec<QualityHotspotRuleCount>,
    #[serde(default)]
    pub structural_signals: QualityHotspotStructuralSignals,
    #[serde(default)]
    pub top_files: Vec<String>,
    #[serde(default)]
    pub delta: QualityDeltaSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityHotspotsSummary {
    pub status: QualityStatus,
    pub aggregation: QualityHotspotAggregation,
    pub evaluated_buckets: usize,
    pub hot_buckets: usize,
    pub total_active_violations: usize,
    #[serde(default)]
    pub total_suppressed_violations: usize,
    #[serde(default)]
    pub new_violations: usize,
    #[serde(default)]
    pub resolved_violations: usize,
    #[serde(default)]
    pub hotspot_score_delta_total: f64,
    #[serde(default)]
    pub risk_score_delta_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityHotspotsOptions {
    pub limit: usize,
    #[serde(default)]
    pub path_prefix: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub rule_ids: Vec<String>,
    #[serde(default)]
    pub aggregation: QualityHotspotAggregation,
    #[serde(default)]
    pub sort_by: QualityHotspotsSortBy,
}

impl Default for QualityHotspotsOptions {
    fn default() -> Self {
        Self {
            limit: 20,
            path_prefix: None,
            language: None,
            rule_ids: Vec::new(),
            aggregation: QualityHotspotAggregation::File,
            sort_by: QualityHotspotsSortBy::HotspotScore,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityHotspotsResult {
    pub summary: QualityHotspotsSummary,
    pub buckets: Vec<QualityHotspotBucket>,
}
