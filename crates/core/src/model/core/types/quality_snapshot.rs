use serde::{Deserialize, Serialize};

use super::quality::{
    QualityHotspotsResult, QualityStatus, RuleViolationsResult, WorkspaceQualityTopMetric,
    WorkspaceQualityTopRule,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QualityProjectSnapshotKind {
    #[default]
    AdHoc,
    Before,
    After,
    Baseline,
}

impl QualityProjectSnapshotKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ad_hoc" => Some(Self::AdHoc),
            "before" => Some(Self::Before),
            "after" => Some(Self::After),
            "baseline" => Some(Self::Baseline),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QualityProjectSnapshotCompareAgainst {
    #[default]
    None,
    SelfBaseline,
    WaveBefore,
}

impl QualityProjectSnapshotCompareAgainst {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "self_baseline" => Some(Self::SelfBaseline),
            "wave_before" => Some(Self::WaveBefore),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QualityProjectGateStatus {
    #[default]
    Ok,
    Regression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityProjectSnapshotOptions {
    #[serde(default)]
    pub snapshot_kind: QualityProjectSnapshotKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wave_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_root: Option<String>,
    #[serde(default)]
    pub compare_against: QualityProjectSnapshotCompareAgainst,
    #[serde(default = "default_true")]
    pub auto_index: bool,
    #[serde(default)]
    pub promote_self_baseline: bool,
    #[serde(default)]
    pub persist_artifacts: bool,
}

impl Default for QualityProjectSnapshotOptions {
    fn default() -> Self {
        Self {
            snapshot_kind: QualityProjectSnapshotKind::AdHoc,
            wave_id: None,
            output_root: None,
            compare_against: QualityProjectSnapshotCompareAgainst::None,
            auto_index: true,
            promote_self_baseline: false,
            persist_artifacts: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityProjectTopHotFiles {
    #[serde(default)]
    pub violation_count: Vec<String>,
    #[serde(default)]
    pub size_bytes: Vec<String>,
    #[serde(default)]
    pub non_empty_lines: Vec<String>,
    #[serde(default)]
    pub metric_graph_edge_out_count: Vec<String>,
    #[serde(default)]
    pub metric_max_cognitive_complexity: Vec<String>,
    #[serde(default)]
    pub metric_duplicate_density_bps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityProjectTopHotspotBuckets {
    #[serde(default)]
    pub file: Vec<String>,
    #[serde(default)]
    pub directory: Vec<String>,
    #[serde(default)]
    pub module: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityProjectArtifactPaths {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_report: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_report: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityProjectSnapshotReport {
    pub generated_at_utc: String,
    pub snapshot_kind: QualityProjectSnapshotKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wave_id: Option<String>,
    pub quality_status_before_refresh: QualityStatus,
    pub quality_status_after_refresh: QualityStatus,
    pub refresh_performed: bool,
    pub ruleset_id: String,
    pub evaluated_files: usize,
    pub violating_files: usize,
    pub total_violations: usize,
    #[serde(default)]
    pub suppressed_violations: usize,
    pub total_non_empty_lines: i64,
    pub total_size_bytes: i64,
    #[serde(default)]
    pub top_rules: Vec<WorkspaceQualityTopRule>,
    #[serde(default)]
    pub top_metrics: Vec<WorkspaceQualityTopMetric>,
    #[serde(default)]
    pub top_hot_files: QualityProjectTopHotFiles,
    #[serde(default)]
    pub top_hotspot_buckets: QualityProjectTopHotspotBuckets,
    pub rule_violations_by_violation_count: RuleViolationsResult,
    pub rule_violations_by_size_bytes: RuleViolationsResult,
    pub rule_violations_by_non_empty_lines: RuleViolationsResult,
    pub rule_violations_by_metric_graph_edge_out_count: RuleViolationsResult,
    pub rule_violations_by_metric_max_cognitive_complexity: RuleViolationsResult,
    pub rule_violations_by_metric_duplicate_density_bps: RuleViolationsResult,
    pub file_hotspots: QualityHotspotsResult,
    pub directory_hotspots: QualityHotspotsResult,
    pub module_hotspots: QualityHotspotsResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityProjectHotspotDelta {
    pub new_violations: usize,
    pub resolved_violations: usize,
    pub risk_score_delta_total: f64,
    pub hotspot_score_delta_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityProjectDeltaReport {
    pub generated_at_utc: String,
    pub compare_against: QualityProjectSnapshotCompareAgainst,
    pub baseline_generated_at_utc: String,
    pub candidate_generated_at_utc: String,
    pub total_violations_delta: i64,
    pub violating_files_delta: i64,
    pub suppressed_violations_delta: i64,
    pub total_non_empty_lines_delta: i64,
    pub total_size_bytes_delta: i64,
    pub new_violations: usize,
    pub resolved_violations: usize,
    pub file_hotspots: QualityProjectHotspotDelta,
    pub directory_hotspots: QualityProjectHotspotDelta,
    pub module_hotspots: QualityProjectHotspotDelta,
    pub gate_status: QualityProjectGateStatus,
    #[serde(default)]
    pub regression_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityProjectSnapshotCapture {
    pub snapshot: QualityProjectSnapshotReport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<QualityProjectDeltaReport>,
    #[serde(default)]
    pub artifacts: QualityProjectArtifactPaths,
}

const fn default_true() -> bool {
    true
}
