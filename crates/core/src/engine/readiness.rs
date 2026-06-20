use anyhow::Result;

use super::Engine;
use super::readiness_scope::{derive_include_roots, requires_mixed_investigation_reindex};
use crate::model::{IndexProfile, IndexingOptions};

impl Engine {
    pub fn ensure_mixed_index_ready_for_paths(
        &self,
        auto_index: bool,
        required_paths: &[String],
    ) -> Result<bool> {
        let auto_indexed = self.ensure_index_ready_with_policy(auto_index)?;
        if !auto_index || !requires_mixed_investigation_reindex(self, required_paths)? {
            return Ok(auto_indexed);
        }

        let _ = self.index_path_with_options(&IndexingOptions {
            profile: Some(IndexProfile::Mixed),
            include_paths: derive_include_roots(required_paths),
            reindex: true,
            ..IndexingOptions::default()
        })?;
        Ok(true)
    }

    pub fn has_indexed_path(&self, path: &str) -> Result<bool> {
        let normalized = self.normalize_lookup_path(path)?;
        let conn = self.open_db_read_only()?;
        let exists = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM files WHERE path = ?1)",
            [&normalized],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(exists > 0)
    }
}
