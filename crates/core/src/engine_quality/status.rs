use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};

use crate::engine::Engine;
use crate::model::QualityStatus;
use crate::quality::CURRENT_QUALITY_RULESET_VERSION;

const META_QUALITY_STATUS: &str = "quality.status";
const META_QUALITY_RULESET_VERSION: &str = "quality.ruleset_version";
const META_QUALITY_LAST_REFRESH_UTC: &str = "quality.last_refresh_utc";
const META_QUALITY_LAST_ERROR_UTC: &str = "quality.last_error_utc";
const META_QUALITY_LAST_ERROR_RULE_ID: &str = "quality.last_error_rule_id";

pub(super) fn quality_tables_available(conn: &Connection) -> Result<bool> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('file_quality', 'file_rule_violations', 'file_quality_metrics')",
    )?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows.len() == 3)
}

pub(super) fn read_quality_status(conn: &Connection) -> Result<QualityStatus> {
    if !quality_tables_available(conn)? {
        return Ok(QualityStatus::Unavailable);
    }

    let stored = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [META_QUALITY_STATUS],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let stored = stored
        .as_deref()
        .and_then(QualityStatus::parse)
        .unwrap_or(QualityStatus::Stale);

    if stored == QualityStatus::Degraded {
        return Ok(QualityStatus::Degraded);
    }
    if quality_refresh_needed_conn(conn)? {
        return Ok(QualityStatus::Stale);
    }
    Ok(QualityStatus::Ready)
}

pub(crate) fn quality_index_needs_refresh(engine: &Engine) -> Result<bool> {
    if !engine.db_path.exists() {
        return Ok(false);
    }
    let conn = engine.open_db_read_only()?;
    Ok(read_quality_status(&conn)? != QualityStatus::Ready)
}

pub(super) fn write_quality_status_ready(tx: &rusqlite::Transaction<'_>) -> Result<()> {
    write_quality_meta(
        tx,
        QualityStatus::Ready,
        None,
        None,
        Some(CURRENT_QUALITY_RULESET_VERSION),
    )
}

pub(super) fn write_quality_status_degraded(
    tx: &rusqlite::Transaction<'_>,
    last_error_rule_id: Option<&str>,
) -> Result<()> {
    write_quality_meta(
        tx,
        QualityStatus::Degraded,
        Some(now_rfc3339()?.as_str()),
        last_error_rule_id,
        Some(CURRENT_QUALITY_RULESET_VERSION),
    )
}

pub(super) fn write_quality_status_unavailable(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![META_QUALITY_STATUS, QualityStatus::Unavailable.as_str()],
    )?;
    Ok(())
}

fn quality_refresh_needed_conn(conn: &Connection) -> Result<bool> {
    let missing_quality_rows: i64 = conn.query_row(
        r#"
        SELECT COUNT(1)
        FROM files f
        LEFT JOIN file_quality q ON q.path = f.path
        WHERE q.path IS NULL
        "#,
        [],
        |row| row.get(0),
    )?;
    if missing_quality_rows > 0 {
        return Ok(true);
    }

    let outdated_rows: i64 = conn.query_row(
        "SELECT COUNT(1) FROM file_quality WHERE quality_ruleset_version != ?1",
        [CURRENT_QUALITY_RULESET_VERSION],
        |row| row.get(0),
    )?;
    if outdated_rows > 0 {
        return Ok(true);
    }

    let missing_metrics: i64 = conn.query_row(
        "SELECT COUNT(1) FROM file_quality WHERE quality_metric_hash = ''",
        [],
        |row| row.get(0),
    )?;
    Ok(missing_metrics > 0)
}

fn write_quality_meta(
    tx: &rusqlite::Transaction<'_>,
    status: QualityStatus,
    last_error_utc: Option<&str>,
    last_error_rule_id: Option<&str>,
    ruleset_version: Option<i64>,
) -> Result<()> {
    let refreshed_at = now_rfc3339()?;
    upsert_meta(tx, META_QUALITY_STATUS, status.as_str())?;
    upsert_meta(tx, META_QUALITY_LAST_REFRESH_UTC, &refreshed_at)?;
    if let Some(version) = ruleset_version {
        upsert_meta(tx, META_QUALITY_RULESET_VERSION, &version.to_string())?;
    }
    upsert_meta(tx, META_QUALITY_LAST_ERROR_UTC, last_error_utc.unwrap_or(""))?;
    upsert_meta(
        tx,
        META_QUALITY_LAST_ERROR_RULE_ID,
        last_error_rule_id.unwrap_or(""),
    )?;
    Ok(())
}

fn upsert_meta(tx: &rusqlite::Transaction<'_>, key: &str, value: &str) -> Result<()> {
    tx.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn now_rfc3339() -> Result<String> {
    Ok(time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?)
}
