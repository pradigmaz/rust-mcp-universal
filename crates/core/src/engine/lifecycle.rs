use std::fs;
use std::path::{Path, PathBuf};

use crate::db_store;
use crate::db_store::touch_database_metadata;
use crate::model::MigrationMode;
use crate::model::{DbPruneResult, DeleteIndexResult};
use crate::rebuild_lock::RebuildLockGuard;
use anyhow::{Context, Result, bail};
use rusqlite::Connection;

use super::Engine;
use super::compatibility;
use super::schema;

pub(super) fn resolve_paths(
    project_root: PathBuf,
    db_path: Option<PathBuf>,
) -> Result<(PathBuf, PathBuf, bool)> {
    let uses_default_store = db_path.is_none();
    let db_path = match db_path {
        Some(db_path) => db_path,
        None => db_store::default_db_path_for_project(&project_root)?,
    };
    let parent = db_path
        .parent()
        .context("database path has no parent directory")?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    Ok((project_root, db_path, uses_default_store))
}

pub(super) fn resolve_paths_read_only(
    project_root: PathBuf,
    db_path: Option<PathBuf>,
) -> Result<(PathBuf, PathBuf)> {
    let db_path = match db_path {
        Some(db_path) => db_path,
        None => db_store::default_db_path_for_project(&project_root)?,
    };
    Ok((project_root, db_path))
}

pub(super) fn init_db(engine: &Engine) -> Result<()> {
    let database_preexisted = engine.db_path.exists();
    let mut conn = open_db_path(&engine.db_path)?;
    compatibility::ensure_schema_preflight(&conn)?;
    if engine.migration_mode == MigrationMode::Off {
        if !database_preexisted {
            bail!("migration_mode=off requires pre-existing initialized database");
        }
        if !schema::required_schema_exists(&conn)? {
            bail!(
                "migration_mode=off requires existing RMU schema and forbids automatic initialization"
            );
        }
        return Ok(());
    }
    conn.execute_batch(schema::INIT_DB_SCHEMA_SQL)?;
    schema::apply_schema_migrations(&mut conn, &engine.db_path, database_preexisted)?;
    compatibility::reconcile_schema_and_index_meta(&conn)?;
    Ok(())
}

pub(super) fn open_db(engine: &Engine) -> Result<Connection> {
    let preexisting = engine.db_path.exists();
    let conn = open_db_path(&engine.db_path)?;
    if preexisting {
        touch_existing_access_with_conn_best_effort(engine, &conn);
    }
    Ok(conn)
}

pub(super) fn open_db_read_only(engine: &Engine) -> Result<Connection> {
    touch_existing_access_best_effort(engine);
    let conn =
        Connection::open_with_flags(&engine.db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .with_context(|| format!("failed to open db {}", engine.db_path.display()))?;
    conn.execute_batch(schema::OPEN_DB_READ_ONLY_PRAGMAS_SQL)
        .context("failed to apply sqlite pragmas")?;
    Ok(conn)
}

pub(super) fn cleanup_stale_indexes(engine: &Engine) -> Result<DbPruneResult> {
    let config = db_store::store_config(&engine.project_root);
    db_store::cleanup_stale_databases(&config, &engine.db_path)
}

pub(super) fn delete_index_storage(engine: &Engine) -> Result<DeleteIndexResult> {
    let _rebuild_lock = RebuildLockGuard::acquire(&engine.db_path)?;
    let mut removed_files = Vec::new();
    let mut candidate_paths = vec![engine.db_path.clone()];
    candidate_paths.extend(db_store::sqlite_sidecar_paths(&engine.db_path));
    for path in candidate_paths {
        if fs::metadata(&path).is_err() {
            continue;
        }
        fs::remove_file(&path).with_context(|| format!("failed to remove {}", path.display()))?;
        removed_files.push(path.display().to_string());
    }

    Ok(DeleteIndexResult {
        db_path: engine.db_path.display().to_string(),
        removed_count: removed_files.len(),
        removed_files,
    })
}

pub(super) fn touch_access(engine: &Engine) -> Result<()> {
    let Some(_touch_lock) = RebuildLockGuard::try_acquire(&engine.db_path)? else {
        return Ok(());
    };
    let conn = open_db_path(&engine.db_path)?;
    touch_database_metadata(&conn, &engine.project_root)
}

fn open_db_path(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("failed to open db {}", db_path.display()))?;
    conn.execute_batch(schema::OPEN_DB_PRAGMAS_SQL)
        .context("failed to apply sqlite pragmas")?;
    Ok(conn)
}

fn touch_existing_access_best_effort(engine: &Engine) {
    let _ = touch_existing_access(engine);
}

fn touch_existing_access(engine: &Engine) -> Result<()> {
    if !engine.db_path.exists() {
        return Ok(());
    }
    let Some(_touch_lock) = RebuildLockGuard::try_acquire(&engine.db_path)? else {
        return Ok(());
    };
    let conn =
        Connection::open_with_flags(&engine.db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)
            .with_context(|| format!("failed to open db {}", engine.db_path.display()))?;
    conn.execute_batch(schema::OPEN_DB_PRAGMAS_SQL)
        .context("failed to apply sqlite pragmas")?;
    touch_database_metadata(&conn, &engine.project_root)
}

fn touch_existing_access_with_conn_best_effort(engine: &Engine, conn: &Connection) {
    let _ = touch_existing_access_with_conn(engine, conn);
}

fn touch_existing_access_with_conn(engine: &Engine, conn: &Connection) -> Result<()> {
    if !engine.db_path.exists() {
        return Ok(());
    }
    let Some(_touch_lock) = RebuildLockGuard::try_acquire(&engine.db_path)? else {
        return Ok(());
    };
    touch_database_metadata(conn, &engine.project_root)
}
