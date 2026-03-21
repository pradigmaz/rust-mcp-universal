use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteIndexResult {
    pub db_path: String,
    pub removed_count: usize,
    pub removed_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct DbMaintenanceOptions {
    #[serde(default)]
    pub integrity_check: bool,
    #[serde(default)]
    pub checkpoint: bool,
    #[serde(default)]
    pub vacuum: bool,
    #[serde(default)]
    pub analyze: bool,
    #[serde(default)]
    pub stats: bool,
    #[serde(default)]
    pub prune: bool,
}

impl DbMaintenanceOptions {
    pub fn normalized(self) -> Self {
        if self.integrity_check
            || self.checkpoint
            || self.vacuum
            || self.analyze
            || self.stats
            || self.prune
        {
            self
        } else {
            Self {
                integrity_check: true,
                checkpoint: true,
                vacuum: true,
                analyze: true,
                stats: true,
                prune: true,
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbCheckpointResult {
    pub busy: i64,
    pub wal_pages: i64,
    pub checkpointed_pages: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMaintenanceStats {
    pub page_size: i64,
    pub page_count: i64,
    pub freelist_count: i64,
    pub approx_free_bytes: u64,
    pub db_size_bytes: u64,
    pub wal_size_bytes: u64,
    pub shm_size_bytes: u64,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DbPruneResult {
    pub removed_databases: usize,
    pub removed_sidecars: usize,
    pub removed_bytes: u64,
    pub removed_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMaintenanceResult {
    pub db_path: String,
    pub options: DbMaintenanceOptions,
    pub integrity_ok: Option<bool>,
    pub integrity_message: Option<String>,
    pub checkpoint: Option<DbCheckpointResult>,
    pub vacuum_ran: bool,
    pub analyze_ran: bool,
    pub stats: Option<DbMaintenanceStats>,
    pub prune: Option<DbPruneResult>,
}
