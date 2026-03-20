use std::fs;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use fs2::FileExt;

use super::metadata::write_lock_payload;
use super::sanitize::sanitize_lock_file_name;
use super::snapshot::{is_lock_stale, try_reclaim_stale_lock};
use super::{
    REBUILD_LOCK_RETRY_SLEEP_MS, REBUILD_LOCK_WAIT_TIMEOUT_MS, RebuildLockGuard, lock_path_for_db,
};

#[derive(Debug)]
pub(super) struct OperationLockGuard {
    file: File,
}

impl OperationLockGuard {
    pub(super) fn acquire(lock_path: &Path) -> std::io::Result<Self> {
        let op_lock_path = operation_lock_path(lock_path);
        ensure_lock_parent_exists(&op_lock_path)?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(op_lock_path)?;
        file.lock_exclusive()?;
        Ok(Self { file })
    }
}

impl Drop for OperationLockGuard {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

impl RebuildLockGuard {
    pub(crate) fn acquire(db_path: &Path) -> Result<Self> {
        let lock_path = lock_path_for_db(db_path);
        ensure_lock_parent_exists(&lock_path)?;
        let start = Instant::now();

        loop {
            let op_guard = OperationLockGuard::acquire(&lock_path).with_context(|| {
                format!("failed to lock operation guard {}", lock_path.display())
            })?;
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&lock_path)
            {
                Ok(mut lock_file) => {
                    let lock_token = match write_lock_payload(&mut lock_file, &lock_path) {
                        Ok(value) => value,
                        Err(err) => {
                            if let Err(cleanup_err) = fs::remove_file(&lock_path) {
                                return Err(err.context(format!(
                                    "failed to cleanup lock {} after payload write error: {}",
                                    lock_path.display(),
                                    cleanup_err
                                )));
                            }
                            return Err(err);
                        }
                    };
                    let wait_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
                    drop(op_guard);
                    return Ok(Self {
                        lock_path,
                        lock_file,
                        lock_token,
                        wait_ms,
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    if try_reclaim_stale_lock(&lock_path) {
                        drop(op_guard);
                        continue;
                    }
                    if is_lock_stale(&lock_path) {
                        drop(op_guard);
                        sleep(Duration::from_millis(REBUILD_LOCK_RETRY_SLEEP_MS));
                        continue;
                    }
                    if start.elapsed() >= Duration::from_millis(REBUILD_LOCK_WAIT_TIMEOUT_MS) {
                        let stale_hint = if is_lock_stale(&lock_path) {
                            " (lock is stale but owner still appears alive or unknown)"
                        } else {
                            ""
                        };
                        return Err(anyhow!(
                            "timed out waiting for rebuild lock {}{}",
                            lock_path.display(),
                            stale_hint
                        ));
                    }
                    drop(op_guard);
                    sleep(Duration::from_millis(REBUILD_LOCK_RETRY_SLEEP_MS));
                }
                Err(err) => {
                    drop(op_guard);
                    return Err(err).with_context(|| {
                        format!("failed to acquire lock {}", lock_path.display())
                    });
                }
            }
        }
    }

    pub(crate) fn try_acquire(db_path: &Path) -> Result<Option<Self>> {
        let lock_path = lock_path_for_db(db_path);
        ensure_lock_parent_exists(&lock_path)?;
        for _attempt in 0..2 {
            let op_guard = OperationLockGuard::acquire(&lock_path).with_context(|| {
                format!("failed to lock operation guard {}", lock_path.display())
            })?;
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&lock_path)
            {
                Ok(mut lock_file) => {
                    let lock_token = match write_lock_payload(&mut lock_file, &lock_path) {
                        Ok(value) => value,
                        Err(err) => {
                            if let Err(cleanup_err) = fs::remove_file(&lock_path) {
                                return Err(err.context(format!(
                                    "failed to cleanup lock {} after payload write error: {}",
                                    lock_path.display(),
                                    cleanup_err
                                )));
                            }
                            return Err(err);
                        }
                    };
                    drop(op_guard);
                    return Ok(Some(Self {
                        lock_path,
                        lock_file,
                        lock_token,
                        wait_ms: 0,
                    }));
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    if try_reclaim_stale_lock(&lock_path) {
                        drop(op_guard);
                        continue;
                    }
                    if is_lock_stale(&lock_path) {
                        drop(op_guard);
                        sleep(Duration::from_millis(REBUILD_LOCK_RETRY_SLEEP_MS));
                        continue;
                    }
                    drop(op_guard);
                    return Ok(None);
                }
                Err(err) => {
                    drop(op_guard);
                    return Err(err).with_context(|| {
                        format!("failed to acquire lock {}", lock_path.display())
                    });
                }
            }
        }

        Ok(None)
    }

    pub(crate) fn wait_ms(&self) -> u64 {
        self.wait_ms
    }
}

fn operation_lock_path(lock_path: &Path) -> PathBuf {
    let file_name = lock_path
        .file_name()
        .map(sanitize_lock_file_name)
        .unwrap_or_else(|| "index.db.rebuild.lock".to_string());
    lock_path.with_file_name(format!("{file_name}.op.lock"))
}

fn ensure_lock_parent_exists(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
