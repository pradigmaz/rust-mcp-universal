use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use time::OffsetDateTime;

pub(super) fn touch_database_metadata_impl(conn: &Connection, project_root: &Path) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;

    let project_root_text = project_root.display().to_string();
    let last_access =
        OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?;

    conn.execute(
        "INSERT INTO meta(key, value) VALUES ('project_root', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![project_root_text],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES ('last_access_utc', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![last_access],
    )?;

    Ok(())
}

pub(super) fn load_last_access(path: &Path) -> Option<OffsetDateTime> {
    if let Some(from_db) = last_access_from_meta(path) {
        return Some(from_db);
    }

    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(OffsetDateTime::from)
}

pub(super) fn same_path(a: &Path, b: &Path) -> bool {
    let a_norm = a.canonicalize().unwrap_or_else(|_| a.to_path_buf());
    let b_norm = b.canonicalize().unwrap_or_else(|_| b.to_path_buf());
    a_norm == b_norm
}

pub(super) fn sqlite_sidecar_paths_impl(db_path: &Path) -> [PathBuf; 2] {
    [
        append_suffix(db_path, "-wal"),
        append_suffix(db_path, "-shm"),
    ]
}

fn last_access_from_meta(path: &Path) -> Option<OffsetDateTime> {
    let conn = Connection::open(path).ok()?;
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'last_access_utc'",
            [],
            |row| row.get(0),
        )
        .optional()
        .ok()
        .flatten();
    let raw = value?;
    OffsetDateTime::parse(&raw, &time::format_description::well_known::Rfc3339).ok()
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut out = path.as_os_str().to_owned();
    out.push(suffix);
    PathBuf::from(out)
}
