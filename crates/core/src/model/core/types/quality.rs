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
pub struct WorkspaceQualitySummary {
    pub ruleset_id: String,
    pub status: QualityStatus,
    pub evaluated_files: usize,
    pub violating_files: usize,
    pub total_violations: usize,
    pub top_rules: Vec<WorkspaceQualityTopRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityViolationEntry {
    pub rule_id: String,
    pub actual_value: i64,
    pub threshold_value: i64,
    pub message: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolationsSummary {
    pub ruleset_id: String,
    pub status: QualityStatus,
    pub evaluated_files: usize,
    pub violating_files: usize,
    pub total_violations: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuleViolationsSortBy {
    #[default]
    ViolationCount,
    SizeBytes,
    NonEmptyLines,
}

impl RuleViolationsSortBy {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "violation_count" => Some(Self::ViolationCount),
            "size_bytes" => Some(Self::SizeBytes),
            "non_empty_lines" => Some(Self::NonEmptyLines),
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
    pub sort_by: RuleViolationsSortBy,
}

impl Default for RuleViolationsOptions {
    fn default() -> Self {
        Self {
            limit: 20,
            path_prefix: None,
            language: None,
            rule_ids: Vec::new(),
            sort_by: RuleViolationsSortBy::ViolationCount,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolationsResult {
    pub summary: RuleViolationsSummary,
    pub hits: Vec<RuleViolationFileHit>,
}
