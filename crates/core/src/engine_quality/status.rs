use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};

use crate::engine::Engine;
use crate::engine::storage::load_existing_quality_state_conn;
use crate::model::QualityStatus;
use crate::quality::{
    CURRENT_QUALITY_RULESET_VERSION, load_quality_policy, load_quality_policy_digest,
};

use super::scope::{apply_quality_scope_policy, build_full_quality_refresh_plan};

const META_QUALITY_STATUS: &str = "quality.status";
const META_QUALITY_RULESET_VERSION: &str = "quality.ruleset_version";
const META_QUALITY_LAST_REFRESH_UTC: &str = "quality.last_refresh_utc";
const META_QUALITY_LAST_ERROR_UTC: &str = "quality.last_error_utc";
const META_QUALITY_LAST_ERROR_RULE_ID: &str = "quality.last_error_rule_id";
const META_QUALITY_POLICY_DIGEST: &str = "quality.policy_digest";

pub(super) fn quality_tables_available(conn: &Connection) -> Result<bool> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('file_quality', 'file_rule_violations', 'file_quality_metrics')",
    )?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows.len() == 3)
}

pub(crate) fn compute_quality_status(engine: &Engine) -> Result<QualityStatus> {
    if !engine.db_path.exists() {
        return Ok(QualityStatus::Unavailable);
    }

    let conn = engine.open_db_read_only()?;
    if !quality_tables_available(&conn)? {
        return Ok(QualityStatus::Unavailable);
    }

    match stored_quality_status(&conn) {
        Ok(QualityStatus::Degraded) => return Ok(QualityStatus::Degraded),
        Err(_) => return Ok(QualityStatus::Degraded),
        _ => {}
    }

    let policy = match load_quality_policy(&engine.project_root) {
        Ok(policy) => policy,
        Err(_) => return Ok(QualityStatus::Degraded),
    };
    let plan = match build_full_quality_refresh_plan(engine, &conn) {
        Ok(plan) => plan,
        Err(_) => return Ok(QualityStatus::Degraded),
    };
    let plan = match apply_quality_scope_policy(&conn, plan, &policy) {
        Ok(plan) => plan,
        Err(_) => return Ok(QualityStatus::Degraded),
    };
    let policy_digest = match load_quality_policy_digest(&engine.project_root) {
        Ok(digest) => digest,
        Err(_) => return Ok(QualityStatus::Degraded),
    };
    if match quality_refresh_needed_conn(&conn, &plan) {
        Ok(needs_refresh) => {
            needs_refresh || stored_policy_digest(&conn)?.as_deref() != Some(policy_digest.as_str())
        }
        Err(_) => return Ok(QualityStatus::Degraded),
    } {
        return Ok(QualityStatus::Stale);
    }
    Ok(QualityStatus::Ready)
}

pub(crate) fn read_quality_degradation_reason(engine: &Engine) -> Result<Option<String>> {
    if !engine.db_path.exists() {
        return Ok(None);
    }

    let conn = engine.open_db_read_only()?;
    if !quality_tables_available(&conn)? {
        return Ok(None);
    }
    if stored_quality_status(&conn)? != QualityStatus::Degraded {
        return Ok(None);
    }

    Ok(conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [META_QUALITY_LAST_ERROR_RULE_ID],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }))
}

pub(super) fn write_quality_status_ready(
    tx: &rusqlite::Transaction<'_>,
    policy_digest: &str,
) -> Result<()> {
    write_quality_meta(
        tx,
        QualityStatus::Ready,
        None,
        None,
        Some(CURRENT_QUALITY_RULESET_VERSION),
        Some(policy_digest),
    )
}

pub(super) fn write_quality_status_degraded(
    tx: &rusqlite::Transaction<'_>,
    last_error_rule_id: Option<&str>,
    policy_digest: &str,
) -> Result<()> {
    write_quality_meta(
        tx,
        QualityStatus::Degraded,
        Some(now_rfc3339()?.as_str()),
        last_error_rule_id,
        Some(CURRENT_QUALITY_RULESET_VERSION),
        Some(policy_digest),
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

fn stored_quality_status(conn: &Connection) -> Result<QualityStatus> {
    Ok(conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [META_QUALITY_STATUS],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .as_deref()
        .and_then(QualityStatus::parse)
        .unwrap_or(QualityStatus::Stale))
}

fn quality_refresh_needed_conn(
    conn: &Connection,
    plan: &super::scope::QualityRefreshPlan,
) -> Result<bool> {
    let existing_quality = load_existing_quality_state_conn(conn)?;
    if !plan.deleted_paths.is_empty() {
        return Ok(true);
    }

    for path in &plan.refresh_paths {
        let Some(state) = existing_quality.get(path) else {
            return Ok(true);
        };
        if !state.is_complete(CURRENT_QUALITY_RULESET_VERSION) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn stored_policy_digest(conn: &Connection) -> Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM meta WHERE key = ?1",
        [META_QUALITY_POLICY_DIGEST],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(Into::into)
}

fn write_quality_meta(
    tx: &rusqlite::Transaction<'_>,
    status: QualityStatus,
    last_error_utc: Option<&str>,
    last_error_rule_id: Option<&str>,
    ruleset_version: Option<i64>,
    policy_digest: Option<&str>,
) -> Result<()> {
    let refreshed_at = now_rfc3339()?;
    upsert_meta(tx, META_QUALITY_STATUS, status.as_str())?;
    upsert_meta(tx, META_QUALITY_LAST_REFRESH_UTC, &refreshed_at)?;
    if let Some(version) = ruleset_version {
        upsert_meta(tx, META_QUALITY_RULESET_VERSION, &version.to_string())?;
    }
    upsert_meta(
        tx,
        META_QUALITY_LAST_ERROR_UTC,
        last_error_utc.unwrap_or(""),
    )?;
    upsert_meta(
        tx,
        META_QUALITY_LAST_ERROR_RULE_ID,
        last_error_rule_id.unwrap_or(""),
    )?;
    if let Some(policy_digest) = policy_digest {
        upsert_meta(tx, META_QUALITY_POLICY_DIGEST, policy_digest)?;
    }
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
