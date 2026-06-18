use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};
use serde_json::{Value, json};

use crate::ServerState;
use crate::rpc_tools::parsing::{parse_optional_usize_in_range, reject_unknown_fields};
use crate::rpc_tools::result::tool_result;

pub(super) fn record_tool_usage(state: &ServerState, tool: &str, result: &anyhow::Result<Value>) {
    let db_path = state
        .db_path
        .clone()
        .unwrap_or_else(|| state.project_path.join(".rmu").join("index.db"));
    if !db_path.exists() {
        return;
    }

    let Ok(conn) = Connection::open(&db_path) else {
        return;
    };

    if ensure_usage_table(&conn).is_err() {
        return;
    }

    let ok = result.is_ok();
    let response_bytes = result
        .as_ref()
        .ok()
        .and_then(|value| serde_json::to_vec(value).ok())
        .map(|bytes| bytes.len())
        .unwrap_or(0);
    let created_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let created_at_ms = i64::try_from(created_at_ms).unwrap_or(i64::MAX);
    let response_bytes = i64::try_from(response_bytes).unwrap_or(i64::MAX);

    let _ = conn.execute(
        "INSERT INTO mcp_tool_usage (tool, ok, response_bytes, created_at_ms)
         VALUES (?1, ?2, ?3, ?4)",
        params![tool, ok as i64, response_bytes, created_at_ms],
    );
}

pub(super) fn usage_stats(args: &Value, state: &ServerState) -> anyhow::Result<Value> {
    reject_unknown_fields(args, "usage_stats", &["limit"])?;
    let limit = parse_optional_usize_in_range(args, "usage_stats", "limit", 1, 100, 5)?;

    let db_path = state
        .db_path
        .clone()
        .unwrap_or_else(|| state.project_path.join(".rmu").join("index.db"));
    if !db_path.exists() {
        return tool_result(json!({"summary": empty_summary(), "recent": []}));
    }
    let Ok(conn) = Connection::open(db_path) else {
        return tool_result(json!({"summary": empty_summary(), "recent": []}));
    };

    if ensure_usage_table(&conn).is_err() {
        return tool_result(json!({"summary": empty_summary(), "recent": []}));
    }

    let summary = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(ok), 0), COALESCE(SUM(response_bytes), 0)
             FROM mcp_tool_usage",
            [],
            |row| {
                Ok(json!({
                    "calls": row.get::<_, i64>(0)?,
                    "ok": row.get::<_, i64>(1)?,
                    "response_bytes": row.get::<_, i64>(2)?,
                }))
            },
        )
        .unwrap_or_else(|_| empty_summary());

    let mut stmt = conn.prepare(
        "SELECT tool, ok, response_bytes, created_at_ms
         FROM mcp_tool_usage
         ORDER BY id DESC
         LIMIT ?1",
    )?;
    let recent = stmt
        .query_map(params![limit as i64], |row| {
            Ok(json!({
                "tool": row.get::<_, String>(0)?,
                "ok": row.get::<_, i64>(1)? != 0,
                "response_bytes": row.get::<_, i64>(2)?,
                "created_at_ms": row.get::<_, i64>(3)?,
            }))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    tool_result(json!({"summary": summary, "recent": recent}))
}

fn ensure_usage_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS mcp_tool_usage (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tool TEXT NOT NULL,
            ok INTEGER NOT NULL,
            response_bytes INTEGER NOT NULL,
            created_at_ms INTEGER NOT NULL
        );",
    )
}

fn empty_summary() -> Value {
    json!({"calls": 0, "ok": 0, "response_bytes": 0})
}
