use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use time::OffsetDateTime;

use crate::model::DbPruneResult;
use crate::rebuild_lock::RebuildLockGuard;

use super::DbStoreConfig;
use super::metadata::{load_last_access, same_path, sqlite_sidecar_paths_impl};

pub(super) fn cleanup_stale_databases_impl(
    config: &DbStoreConfig,
    active_db_path: &Path,
) -> Result<DbPruneResult> {
    // TTL cleanup is only meaningful for shared multi-project stores.
    if !config.shared_store || config.ttl_days <= 0 {
        return Ok(DbPruneResult::default());
    }
    fs::create_dir_all(&config.root_dir)
        .with_context(|| format!("failed to create {}", config.root_dir.display()))?;

    let threshold = OffsetDateTime::now_utc() - time::Duration::days(config.ttl_days);
    let mut result = DbPruneResult::default();
    for entry in fs::read_dir(&config.root_dir)? {
        let entry = match entry {
            Ok(v) => v,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("db") {
            continue;
        }
        if same_path(&path, active_db_path) {
            continue;
        }

        let lock_guard = match RebuildLockGuard::try_acquire(&path) {
            Ok(Some(lock)) => lock,
            Ok(None) => continue,
            Err(_) => continue,
        };
        let Some(last_access) = load_last_access(&path) else {
            drop(lock_guard);
            continue;
        };
        if last_access >= threshold {
            drop(lock_guard);
            continue;
        }

        let mut can_remove_db = true;
        let mut removed_sidecars = 0_usize;
        let mut removed_bytes = 0_u64;
        let mut removed_files = Vec::new();
        for sidecar in sqlite_sidecar_paths_impl(&path) {
            let sidecar_bytes = fs::metadata(&sidecar).map(|meta| meta.len()).unwrap_or(0);
            if let Err(err) = fs::remove_file(&sidecar) {
                if err.kind() != std::io::ErrorKind::NotFound {
                    can_remove_db = false;
                    break;
                }
            } else {
                removed_sidecars += 1;
                removed_bytes = removed_bytes.saturating_add(sidecar_bytes);
                removed_files.push(sidecar.display().to_string());
            }
        }
        if can_remove_db {
            let db_bytes = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
            if fs::remove_file(&path).is_ok() {
                result.removed_databases += 1;
                result.removed_sidecars += removed_sidecars;
                result.removed_bytes = result
                    .removed_bytes
                    .saturating_add(removed_bytes.saturating_add(db_bytes));
                result.removed_files.push(path.display().to_string());
                result.removed_files.extend(removed_files);
            }
        }
        drop(lock_guard);
    }

    Ok(result)
}
