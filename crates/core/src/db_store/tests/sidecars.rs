use std::fs;

use anyhow::Result;
use time::OffsetDateTime;

use super::super::{DbStoreConfig, cleanup_stale_databases};
use super::{seed_db, temp_dir};

#[test]
fn stale_database_with_unremovable_sidecar_is_kept() -> Result<()> {
    let root = temp_dir("rmu-db-cleanup-unremovable-sidecar");
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

    let stale_wal = std::path::PathBuf::from(format!("{}-wal", stale_db.display()));
    fs::create_dir_all(&stale_wal)?;

    let config = DbStoreConfig {
        root_dir: root.clone(),
        ttl_days: 15,
        shared_store: true,
    };
    let summary = cleanup_stale_databases(&config, &active_db)?;

    assert!(stale_db.exists());
    assert!(active_db.exists());
    assert!(stale_wal.exists());
    assert_eq!(summary.removed_databases, 0);
    assert_eq!(summary.removed_sidecars, 0);
    assert_eq!(summary.removed_bytes, 0);
    assert!(summary.removed_files.is_empty());

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn stale_database_with_sidecar_artifacts_reports_reclaimed_bytes() -> Result<()> {
    let root = temp_dir("rmu-db-cleanup-removable-sidecars");
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

    let stale_wal = std::path::PathBuf::from(format!("{}-wal", stale_db.display()));
    let stale_shm = std::path::PathBuf::from(format!("{}-shm", stale_db.display()));
    fs::write(&stale_wal, b"wal-bytes")?;
    fs::write(&stale_shm, b"shm-bytes")?;

    let config = DbStoreConfig {
        root_dir: root.clone(),
        ttl_days: 15,
        shared_store: true,
    };
    let summary = cleanup_stale_databases(&config, &active_db)?;

    assert!(!stale_db.exists());
    assert!(!stale_wal.exists());
    assert!(!stale_shm.exists());
    assert_eq!(summary.removed_databases, 1);
    assert!(summary.removed_sidecars <= 2);
    assert!(summary.removed_bytes > 0);
    assert!(
        summary
            .removed_files
            .iter()
            .any(|path| path == &stale_db.display().to_string())
    );
    assert!(!summary.removed_files.is_empty());

    let _ = fs::remove_dir_all(root);
    Ok(())
}
