use anyhow::Result;

use crate::engine::Engine;
use crate::model::{RuleViolationsOptions, RuleViolationsResult, WorkspaceQualitySummary};

#[path = "engine_quality/query.rs"]
mod query;
#[path = "engine_quality/refresh.rs"]
mod refresh;
#[path = "engine_quality/status.rs"]
mod status;

impl Engine {
    pub fn rule_violations(&self, options: &RuleViolationsOptions) -> Result<RuleViolationsResult> {
        query::load_rule_violations(self, options)
    }
}

pub(crate) fn load_quality_summary(engine: &Engine) -> Result<WorkspaceQualitySummary> {
    query::load_quality_summary(engine)
}

pub(crate) fn quality_index_needs_refresh(engine: &Engine) -> Result<bool> {
    status::quality_index_needs_refresh(engine)
}

pub(crate) fn refresh_quality_only(engine: &Engine) -> Result<()> {
    refresh::refresh_quality_only(engine)
}

pub(crate) fn refresh_quality_after_index(
    engine: &Engine,
    refresh_paths: &std::collections::HashSet<String>,
    deleted_paths: &std::collections::HashSet<String>,
) -> Result<()> {
    refresh::refresh_quality_after_index(engine, refresh_paths, deleted_paths)
}
