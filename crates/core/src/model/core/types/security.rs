use serde::{Deserialize, Serialize};

use super::{FindingConfidence, FindingFamily, QualityLocation, SignalMemoryStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitiveDataSnippetType {
    InlineToken,
    Assignment,
    PrivateKeyHeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitiveDataValidationStatus {
    PatternMatch,
    Heuristic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitiveDataPlaceholderStatus {
    Realistic,
    Placeholder,
    Masked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitiveDataExposureScope {
    CommittedText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitiveDataRotationUrgency {
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveDataFinding {
    pub signal_key: String,
    pub finding_family: FindingFamily,
    pub secret_kind: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<QualityLocation>,
    pub snippet_type: SensitiveDataSnippetType,
    pub confidence: FindingConfidence,
    pub validation_status: SensitiveDataValidationStatus,
    pub placeholder_status: SensitiveDataPlaceholderStatus,
    pub exposure_scope: SensitiveDataExposureScope,
    pub rotation_urgency: SensitiveDataRotationUrgency,
    pub manual_review_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_excerpt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_status: Option<SignalMemoryStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveDataSummary {
    pub evaluated_files: usize,
    pub findings: usize,
    #[serde(default)]
    pub high_confidence_findings: usize,
    #[serde(default)]
    pub remembered_noisy_findings: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveDataResult {
    pub summary: SensitiveDataSummary,
    pub hits: Vec<SensitiveDataFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveDataOptions {
    #[serde(default)]
    pub path_prefix: Option<String>,
    pub limit: usize,
    #[serde(default)]
    pub include_low_confidence: bool,
}

impl Default for SensitiveDataOptions {
    fn default() -> Self {
        Self {
            path_prefix: None,
            limit: 20,
            include_low_confidence: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalMemoryDecision {
    Useful,
    Noisy,
}

impl SignalMemoryDecision {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "useful" => Some(Self::Useful),
            "noisy" => Some(Self::Noisy),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMemoryEntry {
    pub signal_key: String,
    pub finding_family: FindingFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub decision: SignalMemoryDecision,
    pub reason: String,
    pub source: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMemoryOptions {
    pub limit: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finding_family: Option<FindingFamily>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<SignalMemoryDecision>,
}

impl Default for SignalMemoryOptions {
    fn default() -> Self {
        Self {
            limit: 20,
            finding_family: None,
            decision: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMemoryResult {
    pub entries: Vec<SignalMemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMemoryMarkRequest {
    pub signal_key: String,
    pub finding_family: FindingFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub decision: SignalMemoryDecision,
    pub reason: String,
    pub source: String,
}
