use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use super::super::compatibility;
use super::super::schema::OPEN_DB_READ_ONLY_PRAGMAS_SQL;

#[derive(Debug, Default)]
pub(super) struct PreflightDbVersions {
    pub(super) db_schema_version: Option<u32>,
    pub(super) index_format_version: Option<u32>,
    pub(super) ann_version: Option<u32>,
}

pub(super) fn load_versions(path: &Path, errors: &mut Vec<String>) -> Result<PreflightDbVersions> {
    if !path.exists() {
        return Ok(PreflightDbVersions::default());
    }

    let conn = match open_db(path) {
        Ok(conn) => conn,
        Err(err) => {
            errors.push(err.to_string());
            return Ok(PreflightDbVersions::default());
        }
    };

    let versions = PreflightDbVersions {
        db_schema_version: read_meta_u32(&conn, "schema_version")?,
        index_format_version: read_meta_u32(&conn, "index_format_version")?,
        ann_version: read_meta_u32(&conn, "ann_version")?,
    };
    if let Err(err) = compatibility::ensure_schema_preflight(&conn) {
        errors.push(err.to_string());
    }
    Ok(versions)
}

pub(super) fn open_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open db {}", path.display()))?;
    conn.execute_batch(OPEN_DB_READ_ONLY_PRAGMAS_SQL)
        .context("failed to apply sqlite pragmas")?;
    Ok(conn)
}

pub(super) fn read_meta_u32(conn: &Connection, key: &str) -> Result<Option<u32>> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
        row.get::<_, String>(0)
    })
    .optional()?
    .map(|raw| {
        raw.parse::<u32>()
            .with_context(|| format!("meta key `{key}` has non-u32 value `{raw}`"))
    })
    .transpose()
}
