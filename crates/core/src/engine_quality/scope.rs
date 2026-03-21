use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use walkdir::WalkDir;

use crate::engine::Engine;
use crate::index_scope::IndexScope;
use crate::index_scope_meta::load_effective_index_scope_from_meta;
use crate::model::{IndexingOptions, QualityMode};
use crate::utils::{ProjectIgnoreMatcher, is_probably_ignored, normalize_path};

#[derive(Debug, Default)]
pub(super) struct QualityRefreshPlan {
    pub(super) refresh_paths: HashSet<String>,
    pub(super) deleted_paths: HashSet<String>,
}

pub(super) fn build_full_quality_refresh_plan(
    engine: &Engine,
    conn: &rusqlite::Connection,
) -> Result<QualityRefreshPlan> {
    match load_effective_index_scope_from_meta(conn)? {
        Some(options) => build_scoped_refresh_plan(engine, conn, &options),
        None => build_legacy_refresh_plan(engine, conn),
    }
}

pub(super) fn load_existing_quality_paths(
    conn: &rusqlite::Connection,
) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT path FROM file_quality")?;
    Ok(stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<HashSet<_>>>()?)
}

fn build_scoped_refresh_plan(
    engine: &Engine,
    conn: &rusqlite::Connection,
    options: &IndexingOptions,
) -> Result<QualityRefreshPlan> {
    let scope = IndexScope::new(options)?;
    let ignore_matcher = ProjectIgnoreMatcher::new(&engine.project_root)?;
    let existing_quality_paths = load_existing_quality_paths(conn)?;
    let refresh_paths = collect_scoped_project_paths(engine, &scope, &ignore_matcher);
    let deleted_paths = existing_quality_paths
        .difference(&refresh_paths)
        .cloned()
        .collect::<HashSet<_>>();

    Ok(QualityRefreshPlan {
        refresh_paths,
        deleted_paths,
    })
}

fn build_legacy_refresh_plan(
    engine: &Engine,
    conn: &rusqlite::Connection,
) -> Result<QualityRefreshPlan> {
    let ignore_matcher = ProjectIgnoreMatcher::new(&engine.project_root)?;
    let existing_quality_paths = load_existing_quality_paths(conn)?;
    let mut refresh_paths = load_indexed_file_paths(conn)?
        .into_iter()
        .filter(|path| is_currently_reachable(engine, &ignore_matcher, path))
        .collect::<HashSet<_>>();

    for path in load_quality_only_paths(conn)? {
        if is_currently_reachable(engine, &ignore_matcher, &path) {
            refresh_paths.insert(path);
        }
    }

    let deleted_paths = existing_quality_paths
        .difference(&refresh_paths)
        .cloned()
        .collect::<HashSet<_>>();

    Ok(QualityRefreshPlan {
        refresh_paths,
        deleted_paths,
    })
}

fn collect_scoped_project_paths(
    engine: &Engine,
    scope: &IndexScope,
    ignore_matcher: &ProjectIgnoreMatcher,
) -> HashSet<String> {
    let mut present_paths = HashSet::new();

    for walk_entry in WalkDir::new(&engine.project_root).into_iter() {
        let Ok(entry) = walk_entry else {
            continue;
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(&engine.project_root) else {
            continue;
        };
        if is_probably_ignored(relative) || ignore_matcher.is_ignored(relative, false) {
            continue;
        }
        let rel_text = normalize_path(relative);
        if !scope.allows(&rel_text) {
            continue;
        }
        present_paths.insert(rel_text);
    }

    present_paths
}

fn load_indexed_file_paths(conn: &rusqlite::Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT path FROM files")?;
    Ok(stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<HashSet<_>>>()?)
}

fn load_quality_only_paths(conn: &rusqlite::Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT path FROM file_quality WHERE quality_mode = ?1")?;
    Ok(stmt
        .query_map([QualityMode::QualityOnlyOversize.as_str()], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<HashSet<_>>>()?)
}

fn is_currently_reachable(
    engine: &Engine,
    ignore_matcher: &ProjectIgnoreMatcher,
    path: &str,
) -> bool {
    let relative = Path::new(path);
    if is_probably_ignored(relative) || ignore_matcher.is_ignored(relative, false) {
        return false;
    }

    engine
        .project_root
        .join(relative)
        .metadata()
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}
