use std::path::Path;

use anyhow::Result;

use super::Engine;
use crate::index_scope_meta::load_effective_index_scope_from_meta;
use crate::model::{IndexProfile, IndexingOptions};

impl Engine {
    pub fn ensure_mixed_index_ready_for_paths(
        &self,
        auto_index: bool,
        required_paths: &[String],
    ) -> Result<bool> {
        let auto_indexed = self.ensure_index_ready_with_policy(auto_index)?;
        if !auto_index || !self.requires_mixed_investigation_reindex(required_paths)? {
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

    fn requires_mixed_investigation_reindex(&self, required_paths: &[String]) -> Result<bool> {
        let conn = self.open_db_read_only()?;
        let effective_profile = load_effective_index_scope_from_meta(&conn)?
            .and_then(|options| options.profile)
            .or_else(|| self.resolve_default_index_profile(None));
        if matches!(effective_profile, Some(IndexProfile::RustMonorepo)) {
            return Ok(true);
        }

        for path in required_paths {
            if !path_exists_within_project(&self.project_root, path) {
                continue;
            }
            if !self.has_indexed_path(path)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn path_exists_within_project(project_root: &Path, path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return false;
    }

    let candidate = Path::new(trimmed);
    let full_path = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        project_root.join(candidate)
    };
    full_path.exists()
}

fn derive_include_roots(required_paths: &[String]) -> Vec<String> {
    let mut roots = required_paths
        .iter()
        .filter_map(|path| derive_include_root(path))
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    roots
}

fn derive_include_root(path: &str) -> Option<String> {
    let normalized = path.trim().replace('\\', "/");
    if normalized.is_empty() {
        return None;
    }

    let components = normalized.split('/').collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        if matches!(
            *component,
            "src" | "app" | "web" | "legacy" | "migrations" | "tests"
        ) {
            let root = components[..index].join("/");
            return (!root.is_empty()).then_some(root);
        }
    }

    Path::new(&normalized)
        .parent()
        .map(|parent| parent.to_string_lossy().replace('\\', "/"))
        .filter(|parent| !parent.is_empty() && parent != ".")
}
