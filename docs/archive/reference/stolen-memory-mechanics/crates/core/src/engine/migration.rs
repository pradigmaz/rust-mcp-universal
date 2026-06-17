use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::json;

use crate::ProjectContext;
use crate::context::{legacy_project_memory_root, project_memory_root};
use crate::model::{MigrateMemoryRootResult, MigrationSourceCandidate, StorageMode};

use super::canonical::{
    CanonicalNote, CanonicalNoteParts, PROJECT_NODE_SLUG, canonical_note_from_parts,
    canonical_note_path, now_timestamp_string, parse_canonical_document, render_note,
    scan_canonical_locations,
};
use super::graph_hubs::{humanize_project_name, sync_graph_hubs};
use super::notes::scan_markdown_files;
use super::{Engine, StateFailure};

pub(super) fn migrate_memory_root(
    engine: &Engine,
    target_storage_mode: StorageMode,
    dry_run: bool,
    source_root: Option<PathBuf>,
) -> Result<MigrateMemoryRootResult> {
    let destination_context = ProjectContext::resolve(&engine.project_root, target_storage_mode)?;
    let destination_root = destination_context.memory_root.clone();
    let legacy_sources = discover_legacy_sources(engine, &destination_root)?;
    let candidate_sources = collect_candidate_sources(&legacy_sources);
    if dry_run && source_root.is_none() && legacy_sources.len() > 1 {
        return Ok(MigrateMemoryRootResult {
            dry_run: true,
            target_storage_mode,
            source_root: None,
            destination_root: destination_root.display().to_string(),
            canonical_paths: Vec::new(),
            candidate_sources,
            destination_exists: destination_root.exists(),
            destination_has_memory: !existing_canonical_paths(
                &destination_root,
                &destination_root,
            )?
            .is_empty(),
            migrated: false,
            rebuilt: false,
            deleted_source_paths: Vec::new(),
            warnings: vec![format!(
                "multiple legacy memory roots were found ({}); apply requires explicit source_root",
                legacy_sources
                    .iter()
                    .map(|(root, _)| root.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )],
        });
    }
    let (source_root, source_paths) =
        select_source_root(engine, &legacy_sources, source_root.as_deref())?;
    let destination_paths = existing_canonical_paths(&destination_root, &destination_root)?;
    let destination_has_memory = !destination_paths.is_empty();

    let mut warnings = Vec::new();
    if source_paths.is_empty() {
        warnings
            .push("no legacy canonical Markdown found outside the active memory root".to_string());
    }
    if destination_has_memory {
        warnings.push(
            "destination memory root already contains canonical memory; migration refuses to merge"
                .to_string(),
        );
    }

    if dry_run || source_paths.is_empty() || destination_has_memory {
        return Ok(MigrateMemoryRootResult {
            dry_run,
            target_storage_mode,
            source_root: Some(source_root.display().to_string()),
            destination_root: destination_root.display().to_string(),
            canonical_paths: source_paths,
            candidate_sources,
            destination_exists: destination_root.exists(),
            destination_has_memory,
            migrated: false,
            rebuilt: false,
            deleted_source_paths: Vec::new(),
            warnings,
        });
    }

    let rewritten_paths = rewrite_canonical_notes(
        &source_root,
        &destination_root,
        &engine.context.project_name(),
    )?;
    if let Err(err) = sync_graph_hubs(&destination_root, &engine.context.project_name()) {
        cleanup_paths(&rewritten_paths);
        return Err(err);
    }
    let rebuild_result = match Engine::from_context(destination_context)?.rebuild_index() {
        Ok(result) if result.errors.is_empty() => result,
        Ok(result) => {
            cleanup_paths(&rewritten_paths);
            bail!(
                "rebuild_index reported parse failures after migration: {}",
                result.errors.join("; ")
            );
        }
        Err(err) => {
            cleanup_paths(&rewritten_paths);
            return Err(err);
        }
    };

    let deleted_source_paths = delete_source_paths(&source_root, &source_paths)?;
    let _ = rebuild_result;

    Ok(MigrateMemoryRootResult {
        dry_run: false,
        target_storage_mode,
        source_root: Some(source_root.display().to_string()),
        destination_root: destination_root.display().to_string(),
        canonical_paths: source_paths,
        candidate_sources,
        destination_exists: destination_root.exists(),
        destination_has_memory: false,
        migrated: true,
        rebuilt: true,
        deleted_source_paths,
        warnings,
    })
}

fn select_source_root(
    engine: &Engine,
    legacy_sources: &[(PathBuf, Vec<String>)],
    source_root: Option<&Path>,
) -> Result<(PathBuf, Vec<String>)> {
    match (legacy_sources, source_root) {
        ([], Some(source_root)) => Err(StateFailure::new(
            "E_INVALID_SOURCE_ROOT",
            format!(
                "source_root `{}` does not contain canonical legacy memory",
                source_root.display()
            ),
            json!({
                "source_root": source_root.display().to_string(),
                "candidate_sources": []
            }),
        )
        .into()),
        ([], None) => Ok((engine.project_root.clone(), Vec::new())),
        ([(root, paths)], None) => Ok((root.clone(), paths.clone())),
        (sources, Some(source_root)) => sources
            .iter()
            .find(|(root, _)| same_path(root, source_root))
            .map(|(root, paths)| (root.clone(), paths.clone()))
            .ok_or_else(|| {
                StateFailure::new(
                    "E_INVALID_SOURCE_ROOT",
                    format!(
                        "source_root `{}` is not one of the detected legacy memory roots",
                        source_root.display()
                    ),
                    json!({
                        "source_root": source_root.display().to_string(),
                        "candidate_sources": collect_candidate_sources(sources)
                    }),
                )
                .into()
            }),
        (sources, None) => Err(StateFailure::new(
            "E_AMBIGUOUS_SOURCE_ROOT",
            format!(
                "multiple legacy memory roots were found ({}); pass source_root explicitly",
                sources
                    .iter()
                    .map(|(root, _)| root.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            json!({
                "safe_recovery_hint": "call migrate_memory_root again with source_root set to one detected legacy root",
                "candidate_sources": collect_candidate_sources(sources)
            }),
        )
        .into()),
    }
}

fn collect_candidate_sources(
    legacy_sources: &[(PathBuf, Vec<String>)],
) -> Vec<MigrationSourceCandidate> {
    legacy_sources
        .iter()
        .map(|(root, canonical_paths)| MigrationSourceCandidate {
            root: root.display().to_string(),
            canonical_paths: canonical_paths.clone(),
        })
        .collect()
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = normalize_path_for_compare(left);
    let right = normalize_path_for_compare(right);
    left == right
}

fn normalize_path_for_compare(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let raw = canonical.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        raw.to_ascii_lowercase()
    } else {
        raw
    }
}

fn discover_legacy_sources(
    engine: &Engine,
    destination_root: &Path,
) -> Result<Vec<(PathBuf, Vec<String>)>> {
    let mut candidates = Vec::new();

    let project_root = engine.project_root.clone();
    let visible_project_root = project_memory_root(&engine.project_root);
    let hidden_project_root = legacy_project_memory_root(&engine.project_root);
    let current_memory_root = engine.memory_root.clone();

    for candidate in [
        project_root,
        visible_project_root,
        hidden_project_root,
        current_memory_root,
    ] {
        if candidate == destination_root || candidates.iter().any(|(root, _)| *root == candidate) {
            continue;
        }
        let paths = existing_canonical_paths(&candidate, &candidate)?;
        if !paths.is_empty() {
            candidates.push((candidate, paths));
        }
    }

    Ok(candidates)
}

fn existing_canonical_paths(root: &Path, prefix_root: &Path) -> Result<Vec<String>> {
    let mut existing = scan_markdown_files(root)?
        .into_iter()
        .map(|path| relative_string(prefix_root, &path))
        .collect::<Vec<_>>();
    existing.sort();
    existing.dedup();
    Ok(existing)
}

fn rewrite_canonical_notes(
    source_root: &Path,
    destination_root: &Path,
    project_name: &str,
) -> Result<Vec<PathBuf>> {
    let mut notes = load_source_notes(source_root, project_name)?;
    let project_title = notes
        .iter()
        .find(|note| note.frontmatter.node_type == "project")
        .map(|note| note.frontmatter.title.clone())
        .unwrap_or_else(|| humanize_project_name(project_name));
    let title_by_slug = notes
        .iter()
        .map(|note| (note.slug.clone(), note.frontmatter.title.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut planned = Vec::<(CanonicalNote, PathBuf)>::new();
    let mut seen_relative = std::collections::BTreeMap::<String, String>::new();
    for mut note in notes.drain(..) {
        note.frontmatter.project = project_name.to_string();
        ensure_machine_alias(&mut note);
        refresh_relation_titles_from_map(&mut note, &title_by_slug, &project_title);
        ensure_required_project_link(&mut note, &project_title);

        let destination = canonical_note_path(
            destination_root,
            &note.frontmatter.node_type,
            &note.frontmatter.title,
            &note.frontmatter.status,
        );
        let relative = relative_string(destination_root, &destination);
        if let Some(existing_slug) = seen_relative.insert(relative.clone(), note.slug.clone()) {
            return Err(StateFailure::new(
                "E_FILENAME_COLLISION",
                format!(
                    "migration would map multiple notes to `{relative}` ({existing_slug} and {})",
                    note.slug
                ),
                json!({
                    "relative_path": relative,
                    "existing_slug": existing_slug,
                    "conflicting_slug": note.slug
                }),
            )
            .into());
        }
        note.file_path = relative;
        planned.push((note, destination));
    }

    let mut copied = Vec::new();
    for (mut note, destination) in planned {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let rendered = render_note(&note);
        note.raw_hash = crate::utils::hash_bytes(rendered.as_bytes());
        std::fs::write(&destination, rendered)
            .with_context(|| format!("failed to write {}", destination.display()))?;
        copied.push(destination);
    }
    Ok(copied)
}

fn delete_source_paths(source_root: &Path, canonical_paths: &[String]) -> Result<Vec<String>> {
    let mut deleted = Vec::new();
    for relative in canonical_paths {
        let source = source_root.join(relative);
        if source.exists() {
            std::fs::remove_file(&source)
                .with_context(|| format!("failed to delete {}", source.display()))?;
            deleted.push(relative.clone());
        }
    }
    for location in scan_canonical_locations(source_root).into_iter().rev() {
        if location.is_dir() {
            let _ = std::fs::remove_dir(&location);
        }
    }
    Ok(deleted)
}

fn cleanup_paths(paths: &[PathBuf]) {
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}

fn load_source_notes(source_root: &Path, project_name: &str) -> Result<Vec<CanonicalNote>> {
    let files = scan_markdown_files(source_root)?;
    let mut notes = files
        .iter()
        .map(|path| parse_canonical_document(source_root, path, None))
        .collect::<Result<Vec<_>>>()?;
    let project_count = notes
        .iter()
        .filter(|note| note.frontmatter.node_type == "project")
        .count();
    if project_count > 1 {
        return Err(StateFailure::new(
            "E_MULTIPLE_PROJECT_HUBS",
            "migration found multiple project notes in the legacy memory root",
            json!({
                "project_root": source_root.display().to_string(),
                "project_note_count": project_count
            }),
        )
        .into());
    }
    if project_count == 0 {
        let timestamp = now_timestamp_string();
        notes.push(canonical_note_from_parts(
            source_root,
            project_name,
            CanonicalNoteParts {
                node_type: "project".to_string(),
                slug: PROJECT_NODE_SLUG.to_string(),
                title: humanize_project_name(project_name),
                status: "active".to_string(),
                summary: format!(
                    "Project hub for `{project_name}`. Use this note as the Obsidian root for modules, tasks, risks, decisions, and progress."
                ),
                tags: vec!["memory".to_string()],
                aliases: vec![PROJECT_NODE_SLUG.to_string()],
                id: format!("project-{project_name}"),
                created_at: timestamp.clone(),
                updated_at: timestamp,
            },
        ));
    }
    for note in &mut notes {
        if note.frontmatter.node_type == "project" {
            note.slug = PROJECT_NODE_SLUG.to_string();
        }
    }
    Ok(notes)
}

fn ensure_machine_alias(note: &mut CanonicalNote) {
    if !note
        .frontmatter
        .aliases
        .iter()
        .any(|alias| alias == &note.slug)
    {
        note.frontmatter.aliases.push(note.slug.clone());
    }
}

fn ensure_required_project_link(note: &mut CanonicalNote, project_title: &str) {
    if note.frontmatter.node_type == "project" {
        return;
    }
    if let Some(existing) = note.relations.iter_mut().find(|relation| {
        relation.relation_kind == "documents" && relation.target_slug == PROJECT_NODE_SLUG
    }) {
        existing.target_title = project_title.to_string();
        return;
    }
    note.relations.push(super::canonical::ParsedRelation {
        relation_kind: "documents".to_string(),
        target_slug: PROJECT_NODE_SLUG.to_string(),
        target_title: project_title.to_string(),
    });
}

fn refresh_relation_titles_from_map(
    note: &mut CanonicalNote,
    title_by_slug: &std::collections::BTreeMap<String, String>,
    project_title: &str,
) {
    for relation in &mut note.relations {
        if relation.target_slug == PROJECT_NODE_SLUG {
            relation.target_title = project_title.to_string();
            continue;
        }
        if let Some(title) = title_by_slug.get(&relation.target_slug) {
            relation.target_title = title.clone();
        }
    }
}

fn relative_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{Engine, StorageMode, as_state_failure};

    use super::PROJECT_NODE_SLUG;

    fn temp_root(prefix: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("obsidian-memory-migration-{prefix}-{suffix}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn write_legacy_note(root: &Path, relative: &str, title: &str, node_type: &str, body: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        let normalized_type = node_type
            .trim()
            .replace(['-', ' '], "_")
            .to_ascii_lowercase();
        let slug = if relative == "_index.md" {
            "_index".to_string()
        } else {
            Path::new(relative)
                .file_stem()
                .and_then(|value| value.to_str())
                .expect("slug")
                .to_string()
        };
        let content = format!(
            "---\nid: {normalized_type}-{slug}\ntype: {node_type}\ntitle: {title}\nstatus: active\nproject: workspace\ncreated_at: 1\nupdated_at: 1\n---\n\n# {title}\n\n## Summary\n{body}\n\n## Observations\n\n## Relations\n\n## References\n"
        );
        std::fs::write(path, content).expect("write note");
    }

    #[test]
    fn migrate_memory_root_dry_run_reports_legacy_paths() {
        let root = temp_root("dry-run");
        write_legacy_note(
            &root,
            "_index.md",
            "Workspace",
            "Project",
            "Project summary.",
        );
        write_legacy_note(
            &root,
            "decisions/auth.md",
            "Auth Decision",
            "Decision",
            "Auth summary.",
        );

        let engine = Engine::new_with_mode(&root, StorageMode::Codex).expect("engine");
        let result = engine
            .migrate_memory_root(StorageMode::Project, true, None)
            .expect("dry run");

        assert!(result.dry_run);
        assert_eq!(result.target_storage_mode, StorageMode::Project);
        assert_eq!(
            result.canonical_paths,
            vec!["_index.md", "decisions/auth.md"]
        );
        assert!(!result.destination_has_memory);
        assert!(!result.migrated);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn migrate_memory_root_apply_moves_legacy_markdown_and_rebuilds_index() {
        let root = temp_root("apply");
        write_legacy_note(
            &root,
            "_index.md",
            "Workspace",
            "Project",
            "Project summary.",
        );
        write_legacy_note(
            &root,
            "decisions/auth.md",
            "Auth Decision",
            "Decision",
            "Auth summary.",
        );

        let engine = Engine::new_with_mode(&root, StorageMode::Codex).expect("engine");
        let result = engine
            .migrate_memory_root(StorageMode::Project, false, None)
            .expect("apply migration");

        assert!(result.migrated);
        assert!(result.rebuilt);
        assert!(!root.join("_index.md").exists());
        assert!(!root.join("decisions").join("auth.md").exists());
        assert!(root.join("memory").join("Workspace.md").exists());
        assert!(
            root.join("memory")
                .join("decisions")
                .join("Auth Decision.md")
                .exists()
        );
        assert!(
            root.join("memory")
                .join(".derived")
                .join("index.db")
                .exists()
        );

        let migrated_engine =
            Engine::new_with_mode(&root, StorageMode::Project).expect("migrated engine");
        let search = migrated_engine.search_memory("Auth", 5).expect("search");
        assert_eq!(search[0].slug, "auth");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn migrate_memory_root_apply_moves_hidden_project_memory_into_visible_memory_root() {
        let root = temp_root("apply-hidden-project-memory");
        let hidden_root = root.join(".memory");
        write_legacy_note(
            &hidden_root,
            "_index.md",
            "Workspace",
            "Project",
            "Project summary.",
        );
        write_legacy_note(
            &hidden_root,
            "tasks/eval.md",
            "Eval",
            "Task",
            "Task summary.",
        );

        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
        let result = engine
            .migrate_memory_root(StorageMode::Project, false, None)
            .expect("apply migration");

        assert!(result.migrated);
        assert!(root.join("memory").join("Workspace.md").exists());
        assert!(root.join("memory").join("tasks").join("Eval.md").exists());
        assert!(!hidden_root.join("_index.md").exists());

        let migrated_engine =
            Engine::new_with_mode(&root, StorageMode::Project).expect("migrated engine");
        let search = migrated_engine.search_memory("Eval", 5).expect("search");
        assert_eq!(search[0].slug, "eval");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn migrate_memory_root_apply_moves_visible_project_memory_into_codex_root() {
        let root = temp_root("apply-visible-project-memory");
        let visible_root = root.join("memory");
        write_legacy_note(
            &visible_root,
            "_index.md",
            "Workspace",
            "Project",
            "Project summary.",
        );
        write_legacy_note(
            &visible_root,
            "tasks/eval.md",
            "Eval",
            "Task",
            "Task summary.",
        );

        let engine = Engine::new_with_mode(&root, StorageMode::Codex).expect("engine");
        let result = engine
            .migrate_memory_root(StorageMode::Codex, false, None)
            .expect("apply migration");

        assert!(result.migrated);
        assert!(engine.memory_root.join("Workspace.md").exists());
        assert!(engine.memory_root.join("tasks").join("Eval.md").exists());
        assert!(!visible_root.join("_index.md").exists());

        let search = engine.search_memory("Eval", 5).expect("search");
        assert_eq!(search[0].slug, "eval");

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(engine.memory_root);
    }

    #[test]
    fn migrate_memory_root_requires_explicit_source_root_when_ambiguous() {
        let root = temp_root("ambiguous-source");
        write_legacy_note(
            &root,
            "_index.md",
            "Workspace",
            "Project",
            "Project summary.",
        );
        let hidden_root = root.join(".memory");
        write_legacy_note(
            &hidden_root,
            "tasks/eval.md",
            "Eval",
            "Task",
            "Task summary.",
        );

        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
        let dry_run = engine
            .migrate_memory_root(StorageMode::Project, true, None)
            .expect("dry run");
        assert!(dry_run.source_root.is_none());
        assert_eq!(dry_run.candidate_sources.len(), 2);

        let err = engine
            .migrate_memory_root(StorageMode::Project, false, None)
            .expect_err("ambiguous source should fail");
        let failure = as_state_failure(&err).expect("state failure");
        assert_eq!(failure.code, "E_AMBIGUOUS_SOURCE_ROOT");
        assert_eq!(
            failure.details["candidate_sources"]
                .as_array()
                .expect("candidate sources")
                .len(),
            2
        );

        let applied = engine
            .migrate_memory_root(StorageMode::Project, false, Some(hidden_root.clone()))
            .expect("apply with explicit source root");
        assert!(
            applied.source_root.as_deref().is_some_and(
                |value| value.replace("\\\\?\\", "") == hidden_root.display().to_string()
            )
        );
        assert!(applied.migrated);
        assert!(root.join("memory").join("tasks").join("Eval.md").exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn migrate_memory_root_rejects_unknown_source_root() {
        let root = temp_root("invalid-source");
        let hidden_root = root.join(".memory");
        write_legacy_note(
            &hidden_root,
            "tasks/eval.md",
            "Eval",
            "Task",
            "Task summary.",
        );

        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
        let err = engine
            .migrate_memory_root(
                StorageMode::Project,
                false,
                Some(root.join("missing-source")),
            )
            .expect_err("invalid source root should fail");
        let failure = as_state_failure(&err).expect("state failure");
        assert_eq!(failure.code, "E_INVALID_SOURCE_ROOT");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn migrate_memory_root_synthesizes_project_hub_and_project_links_when_missing() {
        let root = temp_root("synth-hub");
        let hidden_root = root.join(".memory");
        write_legacy_note(
            &hidden_root,
            "tasks/eval.md",
            "Eval",
            "Task",
            "Task summary.",
        );

        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
        engine
            .migrate_memory_root(StorageMode::Project, false, Some(hidden_root.clone()))
            .expect("apply migration");

        let migrated_engine =
            Engine::new_with_mode(&root, StorageMode::Project).expect("migrated engine");
        let opened = migrated_engine
            .open_nodes(&["eval".to_string()])
            .expect("open task");
        assert!(opened[0].relations.iter().any(|relation| {
            relation.relation_kind == "documents" && relation.target_slug == PROJECT_NODE_SLUG
        }));
        assert!(opened[0].relations.iter().any(|relation| {
            relation.relation_kind == "documents" && relation.target_slug == "section-tasks"
        }));
        let project_files = std::fs::read_dir(root.join("memory"))
            .expect("read memory root")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        assert_eq!(project_files.len(), 10);

        let _ = std::fs::remove_dir_all(root);
    }
}
