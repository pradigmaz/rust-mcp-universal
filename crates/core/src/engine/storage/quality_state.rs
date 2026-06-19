use crate::quality::{quality_metrics_hash, suppressed_violations_hash, violations_hash};

#[derive(Debug, Clone)]
pub(in crate::engine) struct ActualQualityState {
    pub(in crate::engine) metric_count: i64,
    pub(in crate::engine) metric_hash: String,
    pub(in crate::engine) violation_count: i64,
    pub(in crate::engine) violation_hash: String,
    pub(in crate::engine) suppressed_violation_count: i64,
    pub(in crate::engine) suppressed_violation_hash: String,
}

impl Default for ActualQualityState {
    fn default() -> Self {
        Self {
            metric_count: 0,
            metric_hash: quality_metrics_hash(&[]),
            violation_count: 0,
            violation_hash: violations_hash(&[]),
            suppressed_violation_count: 0,
            suppressed_violation_hash: suppressed_violations_hash(&[]),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExistingQualityState {
    pub(in crate::engine) source_mtime_unix_ms: Option<i64>,
    pub(in crate::engine) quality_ruleset_version: i64,
    pub(in crate::engine) quality_metric_count: i64,
    pub(in crate::engine) quality_metric_hash: String,
    pub(in crate::engine) quality_violation_count: i64,
    pub(in crate::engine) quality_violation_hash: String,
    pub(in crate::engine) quality_suppressed_violation_count: i64,
    pub(in crate::engine) quality_suppressed_violation_hash: String,
    pub(in crate::engine) actual_quality_metric_count: i64,
    pub(in crate::engine) actual_quality_metric_hash: String,
    pub(in crate::engine) actual_quality_violation_count: i64,
    pub(in crate::engine) actual_quality_violation_hash: String,
    pub(in crate::engine) actual_quality_suppressed_violation_count: i64,
    pub(in crate::engine) actual_quality_suppressed_violation_hash: String,
}

impl ExistingQualityState {
    pub(crate) fn is_complete(&self, expected_ruleset_version: i64) -> bool {
        self.quality_ruleset_version == expected_ruleset_version
            && self.quality_metric_count == self.actual_quality_metric_count
            && self.quality_metric_hash == self.actual_quality_metric_hash
            && self.quality_violation_count == self.actual_quality_violation_count
            && self.quality_violation_hash == self.actual_quality_violation_hash
            && self.quality_suppressed_violation_count
                == self.actual_quality_suppressed_violation_count
            && self.quality_suppressed_violation_hash
                == self.actual_quality_suppressed_violation_hash
    }
}
