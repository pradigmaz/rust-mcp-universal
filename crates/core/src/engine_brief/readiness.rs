use anyhow::Result;
use rusqlite::Connection;

use crate::engine::Engine;
use crate::engine::compatibility;
use crate::index_scope_meta::load_effective_index_scope_from_meta;
use crate::model::{IndexProfile, IndexingOptions};

impl Engine {
    pub fn ensure_index_ready(&self) -> Result<bool> {
        self.ensure_index_ready_with_policy(true)
    }

    pub fn ensure_index_ready_with_policy(&self, auto_index: bool) -> Result<bool> {
        if !self.db_path.exists() {
            if !auto_index {
                return Err(super::index_not_ready_error());
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
                    return Err(super::index_requires_reindex_error(reason));
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
            return Err(super::index_not_ready_error());
        }
        let _ = self.index_path()?;
        Ok(true)
    }
}

pub(super) fn count_files(conn: &Connection) -> Result<usize> {
    let count: i64 = conn.query_row("SELECT COUNT(1) FROM files", [], |row| row.get(0))?;
    Ok(usize::try_from(count).unwrap_or(usize::MAX))
}

pub(super) fn uses_legacy_default_scope(conn: &Connection, engine: &Engine) -> Result<bool> {
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
