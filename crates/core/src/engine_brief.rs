use anyhow::{Result, anyhow};
use rusqlite::Connection;

use crate::engine::Engine;
use crate::engine::compatibility;
use crate::engine_quality::load_quality_summary;
use crate::index_scope_meta::load_effective_index_scope_from_meta;
use crate::model::IndexProfile;
use crate::model::IndexingOptions;
use crate::model::{WorkspaceBrief, WorkspaceLanguageStat, WorkspaceTopSymbol};

#[path = "engine_brief/repair.rs"]
mod repair;

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

        let (files, compatibility, legacy_default_scope) = {
            let conn = if auto_index {
                self.open_db()?
            } else {
                self.open_db_read_only()?
            };
            let files = count_files(&conn)?;
            let compatibility = compatibility::evaluate_index_compatibility(&conn)?;
            let legacy_default_scope = if auto_index {
                uses_legacy_default_scope(&conn, self)?
            } else {
                false
            };
            (files, compatibility, legacy_default_scope)
        };

        if files > 0 {
            if legacy_default_scope {
                let _ = self.index_path_with_options(&IndexingOptions {
                    reindex: true,
                    ..IndexingOptions::default()
                })?;
                return Ok(true);
            }
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
        if !auto_index {
            if let Some(repair_hint) = repair::read_only_repair_hint(self)? {
                return repair::build_repair_brief(self, repair_hint);
            }
        }

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
            repair_hint: None,
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

fn uses_legacy_default_scope(conn: &Connection, engine: &Engine) -> Result<bool> {
    let Some(default_profile) = engine.resolve_default_index_profile(None) else {
        return Ok(false);
    };

    let is_legacy_scope = match load_effective_index_scope_from_meta(conn)? {
        Some(options) => {
            options.profile.is_none()
                && options.include_paths.is_empty()
                && options.exclude_paths.is_empty()
        }
        None => true,
    };
    if !is_legacy_scope {
        return Ok(false);
    }

    match default_profile {
        IndexProfile::RustMonorepo => Ok(true),
        IndexProfile::Mixed => legacy_index_contains_doc_languages(conn),
        IndexProfile::DocsHeavy => Ok(false),
    }
}

fn legacy_index_contains_doc_languages(conn: &Connection) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(1) FROM files WHERE language IN ('markdown', 'text')",
        [],
        |row| row.get(0),
    )?;
    Ok(count > 0)
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::Engine;
    use crate::engine::test_index_path_with_options_impl;
    use crate::model::IndexingOptions;

    fn temp_project_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    #[test]
    fn ensure_index_ready_repairs_legacy_unscoped_non_rust_index_with_docs() -> anyhow::Result<()> {
        let project_dir = temp_project_dir("rmu-engine-brief-legacy-non-rust-scope");
        fs::create_dir_all(project_dir.join("src"))?;
        fs::create_dir_all(project_dir.join("docs"))?;
        fs::write(
            project_dir.join("src/main.ts"),
            "export const legacyMixedRepair = 1;\n",
        )?;
        fs::write(
            project_dir.join("docs/guide.md"),
            "legacy_unscoped_docs_marker\n",
        )?;

        let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
        let _ = test_index_path_with_options_impl(&engine, &IndexingOptions::default())?;

        let before = engine.index_status()?;
        assert_eq!(before.files, 2, "legacy unscoped index should include docs");

        assert!(engine.ensure_index_ready_with_policy(true)?);

        let after = engine.index_status()?;
        assert_eq!(after.files, 1, "mixed-scope repair should prune docs");
        let conn = engine.open_db_read_only()?;
        let remaining_docs: i64 = conn.query_row(
            "SELECT COUNT(1) FROM files WHERE language IN ('markdown', 'text')",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(
            remaining_docs, 0,
            "repaired index should not retain docs/text files"
        );

        let _ = fs::remove_dir_all(project_dir);
        Ok(())
    }
}
