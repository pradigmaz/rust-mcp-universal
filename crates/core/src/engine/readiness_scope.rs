use std::path::Path;

use anyhow::Result;

use super::Engine;
use crate::index_scope_meta::load_effective_index_scope_from_meta;
use crate::model::IndexProfile;

pub(super) fn requires_mixed_investigation_reindex(
    engine: &Engine,
    required_paths: &[String],
) -> Result<bool> {
    let conn = engine.open_db_read_only()?;
    let effective_profile = load_effective_index_scope_from_meta(&conn)?
        .and_then(|options| options.profile)
        .or_else(|| engine.resolve_default_index_profile(None));
    if matches!(effective_profile, Some(IndexProfile::RustMonorepo)) {
        return Ok(true);
    }

    for path in required_paths {
        if !path_exists_within_project(&engine.project_root, path) {
            continue;
        }
        if !engine.has_indexed_path(path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn derive_include_roots(required_paths: &[String]) -> Vec<String> {
    let mut roots = required_paths
        .iter()
        .filter_map(|path| derive_include_root(path))
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    roots
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
