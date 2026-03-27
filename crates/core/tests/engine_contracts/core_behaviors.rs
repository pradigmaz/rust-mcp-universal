pub(super) use std::error::Error;
pub(super) use std::fs;
pub(super) use std::path::Path;

pub(super) use rmu_core::model::{
    DbMaintenanceOptions, QueryBenchmarkComparisonOptions, QueryBenchmarkGateThresholds,
};
pub(super) use rmu_core::{
    Engine, PrivacyMode, QueryBenchmarkOptions, QueryOptions, SemanticFailMode,
};
pub(super) use rusqlite::Connection;

pub(super) use crate::common::{cleanup_project, setup_indexed_project, temp_project_dir};

#[path = "core_behaviors/benchmark.rs"]
mod benchmark;
#[path = "core_behaviors/bootstrap.rs"]
mod bootstrap;
#[path = "core_behaviors/bootstrap_broad.rs"]
mod bootstrap_broad;
#[path = "core_behaviors/bootstrap_broad_mod.rs"]
mod bootstrap_broad_mod;
#[path = "core_behaviors/bootstrap_broad_shared.rs"]
mod bootstrap_broad_shared;
#[path = "core_behaviors/context.rs"]
mod context;
#[path = "core_behaviors/maintenance.rs"]
mod maintenance;
#[path = "core_behaviors/navigation.rs"]
mod navigation;
#[path = "core_behaviors/report.rs"]
mod report;
#[path = "core_behaviors/search.rs"]
mod search;
#[path = "core_behaviors/search_mixed.rs"]
mod search_mixed;
#[path = "core_behaviors/search_mod.rs"]
mod search_mod;
#[path = "core_behaviors/status.rs"]
mod status;

pub(super) fn write_single_query_dataset(dataset_path: &Path) -> Result<(), Box<dyn Error>> {
    fs::write(
        dataset_path,
        r#"{
            "queries": [
                {
                    "query": "alpha_beta_gamma",
                    "qrels": [{"path": "src/main.rs", "relevance": 1.0}]
                }
            ]
        }"#,
    )?;
    Ok(())
}
