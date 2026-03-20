use std::fs;

use anyhow::Result;
use time::OffsetDateTime;

use super::super::{DbStoreConfig, cleanup_stale_databases};
use super::{seed_db, temp_dir};
use crate::rebuild_lock::{RebuildLockGuard, lock_path_for_db};

#[test]
fn stale_databases_are_removed() -> Result<()> {
    let root = temp_dir("rmu-db-cleanup");
    fs::create_dir_all(&root)?;

    let stale_db = root.join("stale.db");
    let active_db = root.join("active.db");

    seed_db(
        &stale_db,
        OffsetDateTime::now_utc() - time::Duration::days(30),
    )?;
    seed_db(
        &active_db,
        OffsetDateTime::now_utc() - time::Duration::days(1),
    )?;

    let config = DbStoreConfig {
        root_dir: root.clone(),
        ttl_days: 15,
        shared_store: true,
    };
    let summary = cleanup_stale_databases(&config, &active_db)?;

    assert!(!stale_db.exists());
    assert!(active_db.exists());
    assert_eq!(summary.removed_databases, 1);
    assert_eq!(summary.removed_sidecars, 0);
    assert!(summary.removed_bytes > 0);
    assert_eq!(summary.removed_files, vec![stale_db.display().to_string()]);

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn stale_database_with_active_lock_is_kept() -> Result<()> {
    let root = temp_dir("rmu-db-cleanup-lock");
    fs::create_dir_all(&root)?;

    let stale_db = root.join("stale.db");
    let active_db = root.join("active.db");

    seed_db(
        &stale_db,
        OffsetDateTime::now_utc() - time::Duration::days(30),
    )?;
    seed_db(
        &active_db,
        OffsetDateTime::now_utc() - time::Duration::days(1),
    )?;
    fs::write(lock_path_for_db(&stale_db), b"pid=999999\n")?;

    let config = DbStoreConfig {
        root_dir: root.clone(),
        ttl_days: 15,
        shared_store: true,
    };
    let summary = cleanup_stale_databases(&config, &active_db)?;

    assert!(stale_db.exists());
    assert!(active_db.exists());
    assert_eq!(summary.removed_databases, 0);
    assert_eq!(summary.removed_sidecars, 0);
    assert_eq!(summary.removed_bytes, 0);
    assert!(summary.removed_files.is_empty());

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn stale_database_with_acquired_lock_guard_is_kept() -> Result<()> {
    let root = temp_dir("rmu-db-cleanup-active-guard-lock");
    fs::create_dir_all(&root)?;

    let stale_db = root.join("stale.db");
    let active_db = root.join("active.db");

    seed_db(
        &stale_db,
        OffsetDateTime::now_utc() - time::Duration::days(30),
    )?;
    seed_db(
        &active_db,
        OffsetDateTime::now_utc() - time::Duration::days(1),
    )?;

    let guard =
        RebuildLockGuard::try_acquire(&stale_db)?.expect("lock should be acquired for stale db");

    let config = DbStoreConfig {
        root_dir: root.clone(),
        ttl_days: 15,
        shared_store: true,
    };
    let summary = cleanup_stale_databases(&config, &active_db)?;

    assert!(stale_db.exists());
    assert!(active_db.exists());
    assert_eq!(summary.removed_databases, 0);
    assert_eq!(summary.removed_sidecars, 0);
    assert_eq!(summary.removed_bytes, 0);
    assert!(summary.removed_files.is_empty());

    drop(guard);
    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn stale_database_with_malformed_stale_lock_is_removed() -> Result<()> {
    let root = temp_dir("rmu-db-cleanup-stale-malformed-lock");
    fs::create_dir_all(&root)?;

    let stale_db = root.join("stale.db");
    let active_db = root.join("active.db");

    seed_db(
        &stale_db,
        OffsetDateTime::now_utc() - time::Duration::days(30),
    )?;
    seed_db(
        &active_db,
        OffsetDateTime::now_utc() - time::Duration::days(1),
    )?;

    let lock_path = lock_path_for_db(&stale_db);
    fs::write(&lock_path, b"broken-lock-content")?;

    // Make lock stale so cleanup can reclaim it.
    let stale_ts = std::time::SystemTime::now() - std::time::Duration::from_secs(7 * 60 * 60);
    let lock_file = std::fs::File::options().write(true).open(&lock_path)?;
    lock_file.set_modified(stale_ts)?;

    let config = DbStoreConfig {
        root_dir: root.clone(),
        ttl_days: 15,
        shared_store: true,
    };
    let summary = cleanup_stale_databases(&config, &active_db)?;

    assert!(!stale_db.exists());
    assert!(active_db.exists());
    assert_eq!(summary.removed_databases, 1);
    assert_eq!(summary.removed_sidecars, 0);
    assert!(summary.removed_bytes > 0);
    assert_eq!(summary.removed_files, vec![stale_db.display().to_string()]);

    let _ = fs::remove_dir_all(root);
    Ok(())
}
