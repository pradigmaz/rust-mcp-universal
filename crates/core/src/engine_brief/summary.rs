use anyhow::Result;
use rusqlite::Connection;

use crate::engine::Engine;
use crate::model::{IndexStatus, WorkspaceLanguageStat, WorkspaceTopSymbol};

pub(super) fn load_top_languages(
    engine: &Engine,
    limit: usize,
) -> Result<Vec<WorkspaceLanguageStat>> {
    if !engine.db_path.exists() {
        return Ok(Vec::new());
    }
    let conn = engine.open_db_read_only()?;
    load_languages(&conn, limit)
}

fn load_languages(conn: &Connection, limit: usize) -> Result<Vec<WorkspaceLanguageStat>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT language, COUNT(1) AS c
        FROM files
        GROUP BY language
        ORDER BY c DESC, language ASC
        LIMIT ?1
        "#,
    )?;

    let rows = stmt
        .query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
            let count: i64 = row.get(1)?;
            Ok(WorkspaceLanguageStat {
                language: row.get(0)?,
                files: usize::try_from(count).unwrap_or(usize::MAX),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(rows)
}

pub(super) fn load_top_symbols(engine: &Engine, limit: usize) -> Result<Vec<WorkspaceTopSymbol>> {
    if !engine.db_path.exists() {
        return Ok(Vec::new());
    }
    let conn = engine.open_db_read_only()?;
    load_symbols(&conn, limit)
}

fn load_symbols(conn: &Connection, limit: usize) -> Result<Vec<WorkspaceTopSymbol>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT name, COUNT(1) AS c
        FROM symbols
        WHERE LENGTH(name) >= 3
        GROUP BY name
        ORDER BY c DESC, name ASC
        LIMIT ?1
        "#,
    )?;

    let rows = stmt
        .query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
            let count: i64 = row.get(1)?;
            Ok(WorkspaceTopSymbol {
                name: row.get(0)?,
                count: usize::try_from(count).unwrap_or(usize::MAX),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(rows)
}

pub(super) fn make_recommendations(status: &IndexStatus) -> Vec<String> {
    let mut out = vec![
        "use intent-aware retrieval when exact-term lookup is not enough".to_string(),
        "request structured retrieval diagnostics when you need a machine-readable trace"
            .to_string(),
    ];
    if status.symbols == 0 {
        out.push("symbol graph is empty; refresh the index if this is unexpected".to_string());
    }
    if status.semantic_vectors == 0 {
        out.push(
            "vector coverage is empty; run a full refresh to repopulate ranking artifacts"
                .to_string(),
        );
    }
    if status.chunk_embeddings == 0 {
        out.push("chunk embedding cache is empty; the next full refresh will warm it".to_string());
    }
    out
}
