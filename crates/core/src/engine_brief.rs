use anyhow::{Result, anyhow};

use crate::engine::Engine;
use crate::engine_quality::load_quality_summary;
use crate::model::{WorkspaceBrief, WorkspaceLanguageStat, WorkspaceTopSymbol};

mod readiness;
#[path = "engine_brief/repair.rs"]
mod repair;
mod summary;

impl Engine {
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
        let languages = summary::load_top_languages(self, 8)?;
        let top_symbols = summary::load_top_symbols(self, 12)?;
        let quality_summary = load_quality_summary(self)?;

        Ok(WorkspaceBrief {
            auto_indexed,
            index_status: status.clone(),
            languages,
            top_symbols,
            quality_summary,
            recommendations: summary::make_recommendations(&status),
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

pub(crate) fn load_top_languages_for_brief(
    engine: &Engine,
    limit: usize,
) -> Result<Vec<WorkspaceLanguageStat>> {
    summary::load_top_languages(engine, limit)
}

pub(crate) fn load_top_symbols_for_brief(
    engine: &Engine,
    limit: usize,
) -> Result<Vec<WorkspaceTopSymbol>> {
    summary::load_top_symbols(engine, limit)
}

pub(crate) fn make_recommendations(status: &crate::model::IndexStatus) -> Vec<String> {
    summary::make_recommendations(status)
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
