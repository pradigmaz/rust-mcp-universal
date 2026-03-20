use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;
use time::OffsetDateTime;

use crate::db_store;

pub(super) fn create_pre_migration_backup(
    conn: &Connection,
    db_path: &Path,
    from_version: u32,
    to_version: u32,
) -> Result<()> {
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .context("failed to checkpoint WAL before migration backup")?;

    let parent = db_path
        .parent()
        .context("database path has no parent directory for backup")?;
    let backup_root = parent.join("migration_backups");
    fs::create_dir_all(&backup_root)
        .with_context(|| format!("failed to create backup dir {}", backup_root.display()))?;

    let stamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let prefix = format!("index.pre_migration.v{from_version}_to_v{to_version}.{stamp}");
    let backup_db = backup_root.join(format!("{prefix}.db"));
    backup_main_database(conn, db_path, &backup_db)?;

    let [wal_src, shm_src] = db_store::sqlite_sidecar_paths(db_path);
    let wal_dst = backup_root.join(format!("{prefix}.db-wal"));
    let shm_dst = backup_root.join(format!("{prefix}.db-shm"));
    copy_sidecar_if_exists_best_effort(&wal_src, &wal_dst)?;
    copy_sidecar_if_exists_best_effort(&shm_src, &shm_dst)?;
    Ok(())
}

fn backup_main_database(conn: &Connection, src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    if dst.exists() {
        let _ = fs::remove_file(dst);
    }

    let copy_result = fs::copy(src, dst);
    match copy_result {
        Ok(_) => Ok(()),
        Err(err)
            if err.kind() == std::io::ErrorKind::PermissionDenied
                || err.raw_os_error() == Some(1224) =>
        {
            let escaped = dst.display().to_string().replace('\'', "''");
            conn.execute_batch(&format!("VACUUM INTO '{escaped}'"))
                .with_context(|| {
                    format!(
                        "failed to create main database backup via VACUUM INTO {}",
                        dst.display()
                    )
                })?;
            Ok(())
        }
        Err(err) => Err(err)
            .with_context(|| format!("failed to copy {} to {}", src.display(), dst.display())),
    }
}

fn copy_sidecar_if_exists_best_effort(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    match fs::copy(src, dst) {
        Ok(_) => Ok(()),
        Err(err)
            if err.kind() == std::io::ErrorKind::PermissionDenied
                || err.raw_os_error() == Some(1224) =>
        {
            Ok(())
        }
        Err(err) => Err(err)
            .with_context(|| format!("failed to copy {} to {}", src.display(), dst.display())),
    }
}
