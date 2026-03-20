use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;

#[path = "rebuild_lock/acquire.rs"]
mod acquire;
#[path = "rebuild_lock/cleanup.rs"]
mod cleanup;
#[path = "rebuild_lock/metadata.rs"]
mod metadata;
#[path = "rebuild_lock/process_probe.rs"]
mod process_probe;
#[path = "rebuild_lock/sanitize.rs"]
mod sanitize;
#[path = "rebuild_lock/snapshot.rs"]
mod snapshot;

use metadata::{LockMetadata, LockSnapshot};
use sanitize::sanitize_lock_file_name;

#[cfg(test)]
use metadata::parse_lock_metadata;
#[cfg(test)]
use process_probe::{parse_process_probe_output, process_liveness};

const REBUILD_LOCK_WAIT_TIMEOUT_MS: u64 = 10_000;
const REBUILD_LOCK_RETRY_SLEEP_MS: u64 = 80;
const REBUILD_LOCK_STALE_SECS: u64 = 6 * 60 * 60;
const REBUILD_LOCK_ORPHAN_GRACE_SECS: u64 = 45;
const REBUILD_LOCK_FORCE_RECLAIM_SECS: u64 = 24 * 60 * 60;
static LOCK_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessLiveness {
    Alive,
    Dead,
    Unknown,
}

#[derive(Debug)]
pub(crate) struct RebuildLockGuard {
    pub(super) lock_path: PathBuf,
    pub(super) lock_file: File,
    pub(super) lock_token: String,
    pub(super) wait_ms: u64,
}

pub(crate) fn lock_path_for_db(db_path: &Path) -> PathBuf {
    let file_name = db_path
        .file_name()
        .map(sanitize_lock_file_name)
        .unwrap_or_else(|| "index.db".to_string());
    db_path.with_file_name(format!("{file_name}.rebuild.lock"))
}

#[cfg(test)]
#[path = "rebuild_lock/tests.rs"]
mod tests;
