use anyhow::{Result, anyhow};
use rusqlite::Connection;

use crate::engine::Engine;
use crate::engine::compatibility;
use crate::engine_quality::load_quality_summary;
use crate::model::IndexingOptions;
use crate::model::{WorkspaceBrief, WorkspaceLanguageStat, WorkspaceTopSymbol};

impl Engine {
    pub fn ensure_index_ready(&self) -> Result<bool> {
        self.ensure_index_ready_with_policy(true)
    }

    pub fn ensure_index_ready_with_policy(&self, auto_index: bool) -> Result<bool> {
        if !self.db_path.exists() {
            if !auto_index {
                return Err(index_not_ready_error());
            }
            let _ = self.index_path()?;
            return Ok(true);
        }

        let (files, compatibility) = {
            let conn = if auto_index {
                self.open_db()?
            } else {
                self.open_db_read_only()?
            };
            let files = count_files(&conn)?;
            let compatibility = compatibility::evaluate_index_compatibility(&conn)?;
            (files, compatibility)
        };

        if files > 0 {
            if let Some(reason) = compatibility.reason() {
                if !auto_index {
                    return Err(index_requires_reindex_error(reason));
                }
                let _ = self.index_path_with_options(&IndexingOptions {
                    reindex: true,
                    ..IndexingOptions::default()
                })?;
                return Ok(true);
            }
            return Ok(false);
        }
        if !auto_index {
            return Err(index_not_ready_error());
        }
        let _ = self.index_path()?;
        Ok(true)
    }

    pub fn workspace_brief(&self) -> Result<WorkspaceBrief> {
        self.workspace_brief_with_policy(true)
    }

    pub fn workspace_brief_with_policy(&self, auto_index: bool) -> Result<WorkspaceBrief> {
        let auto_indexed = self.ensure_index_ready_with_policy(auto_index)?;
        if auto_index {
            let _ = self.refresh_quality_if_needed();
        }
        let status = self.index_status()?;
        let languages = load_top_languages_for_brief(self, 8)?;
        let top_symbols = load_top_symbols_for_brief(self, 12)?;
        let quality_summary = load_quality_summary(self)?;

        Ok(WorkspaceBrief {
            auto_indexed,
            index_status: status.clone(),
            languages,
            top_symbols,
            quality_summary,
            recommendations: make_recommendations(&status),
        })
    }
}

pub(crate) fn index_not_ready_message() -> &'static str {
    "index is empty; run an indexing flow or enable automatic indexing before requesting a brief"
}

pub(crate) fn index_not_ready_error() -> anyhow::Error {
    anyhow!(index_not_ready_message())
}

pub(crate) fn index_requires_reindex_message(reason: &str) -> String {
    format!(
        "index is incompatible with the current binary ({reason}); run an explicit reindex flow before requesting a brief"
    )
}

pub(crate) fn index_requires_reindex_error(reason: &str) -> anyhow::Error {
    anyhow!(index_requires_reindex_message(reason))
}

fn count_files(conn: &Connection) -> Result<usize> {
    let count: i64 = conn.query_row("SELECT COUNT(1) FROM files", [], |row| row.get(0))?;
    Ok(usize::try_from(count).unwrap_or(usize::MAX))
}

pub(crate) fn load_top_languages_for_brief(
    engine: &Engine,
    limit: usize,
) -> Result<Vec<WorkspaceLanguageStat>> {
    if !engine.db_path.exists() {
        return Ok(Vec::new());
    }
    let conn = engine.open_db_read_only()?;
    load_top_languages(&conn, limit)
}

fn load_top_languages(conn: &Connection, limit: usize) -> Result<Vec<WorkspaceLanguageStat>> {
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

pub(crate) fn load_top_symbols_for_brief(
    engine: &Engine,
    limit: usize,
) -> Result<Vec<WorkspaceTopSymbol>> {
    if !engine.db_path.exists() {
        return Ok(Vec::new());
    }
    let conn = engine.open_db_read_only()?;
    load_top_symbols(&conn, limit)
}

fn load_top_symbols(conn: &Connection, limit: usize) -> Result<Vec<WorkspaceTopSymbol>> {
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

pub(crate) fn make_recommendations(status: &crate::model::IndexStatus) -> Vec<String> {
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
