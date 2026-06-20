use anyhow::Result;

use super::super::Engine;
use super::common::ensure_file_exists;
use super::related_scoring::build_related_hits;
use crate::model::RelatedFileHit;

impl Engine {
    pub fn related_files(&self, path: &str, limit: usize) -> Result<Vec<RelatedFileHit>> {
        let normalized_path = self.normalize_lookup_path(path)?;
        let limit = limit.max(1);
        let conn = self.open_db()?;
        ensure_file_exists(&conn, &normalized_path)?;
        build_related_hits(&conn, &normalized_path, limit)
    }
}
