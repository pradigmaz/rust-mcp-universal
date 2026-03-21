use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::index_scope::normalize_scope_pattern;
use crate::model::{IndexProfile, IndexingOptions};

const INDEX_SCOPE_META_VERSION: &str = "1";
const META_INDEX_SCOPE_VERSION: &str = "index.scope.version";
const META_INDEX_SCOPE_PROFILE: &str = "index.scope.profile";
const META_INDEX_SCOPE_INCLUDE_PATHS_JSON: &str = "index.scope.include_paths_json";
const META_INDEX_SCOPE_EXCLUDE_PATHS_JSON: &str = "index.scope.exclude_paths_json";

pub(crate) fn write_effective_index_scope_meta(
    tx: &Transaction<'_>,
    options: &IndexingOptions,
) -> Result<()> {
    let include_paths = canonicalize_scope_values(&options.include_paths);
    let exclude_paths = canonicalize_scope_values(&options.exclude_paths);

    upsert_meta(tx, META_INDEX_SCOPE_VERSION, INDEX_SCOPE_META_VERSION)?;
    upsert_meta(
        tx,
        META_INDEX_SCOPE_PROFILE,
        options.profile.map(IndexProfile::as_str).unwrap_or(""),
    )?;
    upsert_meta(
        tx,
        META_INDEX_SCOPE_INCLUDE_PATHS_JSON,
        &serde_json::to_string(&include_paths)?,
    )?;
    upsert_meta(
        tx,
        META_INDEX_SCOPE_EXCLUDE_PATHS_JSON,
        &serde_json::to_string(&exclude_paths)?,
    )?;
    Ok(())
}

pub(crate) fn load_effective_index_scope_from_meta(
    conn: &Connection,
) -> Result<Option<IndexingOptions>> {
    let Some(version) = read_meta(conn, META_INDEX_SCOPE_VERSION)? else {
        return Ok(None);
    };
    if version != INDEX_SCOPE_META_VERSION {
        return Ok(None);
    }

    let Some(include_paths_json) = read_meta(conn, META_INDEX_SCOPE_INCLUDE_PATHS_JSON)? else {
        return Ok(None);
    };
    let Some(exclude_paths_json) = read_meta(conn, META_INDEX_SCOPE_EXCLUDE_PATHS_JSON)? else {
        return Ok(None);
    };

    let include_paths = match parse_scope_values(&include_paths_json) {
        Some(values) => values,
        None => return Ok(None),
    };
    let exclude_paths = match parse_scope_values(&exclude_paths_json) {
        Some(values) => values,
        None => return Ok(None),
    };
    let profile = match read_meta(conn, META_INDEX_SCOPE_PROFILE)? {
        Some(raw) if raw.is_empty() => None,
        Some(raw) => match IndexProfile::parse(&raw) {
            Some(profile) => Some(profile),
            None => return Ok(None),
        },
        None => return Ok(None),
    };

    Ok(Some(IndexingOptions {
        profile,
        changed_since: None,
        changed_since_commit: None,
        include_paths,
        exclude_paths,
        reindex: false,
    }))
}

fn canonicalize_scope_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| normalize_scope_pattern(value))
        .filter(|value| !value.is_empty())
        .collect()
}

fn parse_scope_values(raw: &str) -> Option<Vec<String>> {
    let values = serde_json::from_str::<Vec<String>>(raw).ok()?;
    Some(canonicalize_scope_values(&values))
}

fn read_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| row.get(0))
        .optional()
        .map_err(Into::into)
}

fn upsert_meta(tx: &Transaction<'_>, key: &str, value: &str) -> Result<()> {
    tx.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}
