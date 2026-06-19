use anyhow::Result;

use crate::engine::Engine;
use crate::model::{
    QualityHotspotsOptions, QualityHotspotsResult, QualityProjectSnapshotCapture,
    QualityProjectSnapshotOptions, QualityStatus, RuleViolationsOptions, RuleViolationsResult,
    WorkspaceQualitySummary,
};

#[path = "engine_quality/breakdown.rs"]
mod breakdown;
#[path = "engine_quality/hotspots.rs"]
mod hotspots;
#[path = "engine_quality/metrics.rs"]
mod metrics;
#[path = "engine_quality/query.rs"]
mod query;
#[path = "engine_quality/refresh.rs"]
mod refresh;
#[path = "engine_quality/refresh_input.rs"]
mod refresh_input;
#[path = "engine_quality/refresh_persist.rs"]
mod refresh_persist;
#[path = "engine_quality/rule_violation_rows.rs"]
mod rule_violation_rows;
#[path = "engine_quality/rule_violations.rs"]
mod rule_violations;
#[path = "engine_quality/scope.rs"]
mod scope;
#[path = "engine_quality/snapshot.rs"]
mod snapshot;
#[path = "engine_quality/snapshot_artifacts.rs"]
mod snapshot_artifacts;
#[path = "engine_quality/snapshot_delta.rs"]
mod snapshot_delta;
#[path = "engine_quality/status.rs"]
mod status;
#[path = "engine_quality/structural.rs"]
mod structural;

impl Engine {
    pub fn quality_hotspots(
        &self,
        options: &QualityHotspotsOptions,
    ) -> Result<QualityHotspotsResult> {
        hotspots::load_quality_hotspots(self, options)
    }

    pub fn rule_violations(&self, options: &RuleViolationsOptions) -> Result<RuleViolationsResult> {
        rule_violations::load_rule_violations(self, options)
    }

    pub fn quality_degradation_reason(&self) -> Result<Option<String>> {
        status::read_quality_degradation_reason(self)
    }

    pub fn refresh_quality_if_needed(&self) -> Result<()> {
        if status::compute_quality_status(self)? != QualityStatus::Ready {
            refresh::refresh_quality_only(self)?;
        }
        Ok(())
    }

    pub fn quality_project_snapshot(
        &self,
        options: &QualityProjectSnapshotOptions,
    ) -> Result<QualityProjectSnapshotCapture> {
        snapshot::capture_quality_project_snapshot(self, options)
    }
}

pub(crate) fn load_quality_summary(engine: &Engine) -> Result<WorkspaceQualitySummary> {
    query::load_quality_summary(engine)
}

pub(crate) fn compute_quality_status(engine: &Engine) -> Result<QualityStatus> {
    status::compute_quality_status(engine)
}

pub(crate) fn refresh_quality_after_index(
    engine: &Engine,
    refresh_paths: &std::collections::HashSet<String>,
    deleted_paths: &std::collections::HashSet<String>,
) -> Result<()> {
    refresh::refresh_quality_after_index(engine, refresh_paths, deleted_paths)
}
