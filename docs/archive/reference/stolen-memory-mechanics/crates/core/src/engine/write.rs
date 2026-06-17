use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::Connection;
use serde_json::{Value, json};

use crate::bootstrap::{NormalizedStatus, normalize_status};
use crate::model::{
    AddObservationInput, CreateNodeInput, CreateNodeResult, DuplicateCandidate, LinkNodesInput,
    LinkNodesResult, NodeWriteRef, ObservationWriteResult, UpdateNodeInput, UpdateNodeResult,
};
use crate::utils::hash_bytes;

use super::Engine;
use super::canonical::{
    CanonicalNote, CanonicalNoteParts, PROJECT_NODE_SLUG, ParsedRelation, SECTION_HUB_NODE_TYPE,
    canonical_note_from_parts, canonical_note_path, normalize_node_type, normalize_relation_kind,
    normalize_slug, now_timestamp_string, parse_canonical_document, parse_canonical_note,
    render_note,
};
use super::graph_hubs::{
    humanize_project_name, is_system_node_type, required_section_hub_slug,
    section_hub_title_for_node_type, sync_graph_hubs,
};
use super::notes::scan_markdown_files;

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
static FORCE_SYNC_FAILURE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
pub struct WriteFailure {
    pub code: String,
    pub message: String,
    pub details: Value,
}

impl WriteFailure {
    fn new(code: &str, message: impl Into<String>, details: Value) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            details,
        }
    }
}

impl Display for WriteFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for WriteFailure {}

pub fn as_write_failure(err: &anyhow::Error) -> Option<&WriteFailure> {
    err.chain()
        .find_map(|cause| cause.downcast_ref::<WriteFailure>())
}

#[derive(Debug, Clone)]
struct ResolvedNode {
    id: String,
    slug: String,
    title: String,
    node_type: String,
    file_path: String,
}

pub(super) fn sync_after_write(engine: &Engine) -> Result<crate::model::RebuildIndexResult> {
    #[cfg(test)]
    if FORCE_SYNC_FAILURE.load(Ordering::SeqCst) {
        anyhow::bail!("forced sync failure");
    }
    sync_graph_hubs(&engine.memory_root, &engine.context.project_name())?;
    engine.rebuild_index()
}

pub(super) fn create_node(
    engine: &Engine,
    input: CreateNodeInput,
) -> std::result::Result<CreateNodeResult, WriteFailure> {
    let node_type = normalize_node_type(&input.node_type).ok_or_else(|| {
        unsupported_patch(
            "unsupported node type",
            json!({ "node_type": input.node_type }),
        )
    })?;
    if is_system_node_type(&node_type) {
        return Err(system_node_type_failure(
            "create_node does not allow creating system-managed section hubs",
            json!({ "node_type": node_type }),
        ));
    }
    let title = normalize_required_text(&input.title, "title")?;
    let slug = if node_type == "project" {
        PROJECT_NODE_SLUG.to_string()
    } else if let Some(raw_slug) = input.slug.as_deref() {
        normalize_slug(raw_slug).ok_or_else(|| {
            unsupported_patch(
                "slug must normalize to non-empty canonical form",
                json!({ "slug": raw_slug }),
            )
        })?
    } else {
        normalize_slug(&title).ok_or_else(|| {
            unsupported_patch(
                "title must normalize to non-empty slug",
                json!({ "title": title }),
            )
        })?
    };
    let aliases = normalize_list(input.aliases);
    ensure_no_duplicate_candidate(engine, &node_type, &slug, &title, &aliases)?;
    let status =
        normalize_optional_text(input.status.as_deref()).unwrap_or_else(|| "active".to_string());
    let note_path = canonical_note_path(&engine.memory_root, &node_type, &title, &status);
    if note_path.exists() {
        return Err(WriteFailure::new(
            "E_ALREADY_EXISTS",
            format!("canonical note already exists at {}", note_path.display()),
            json!({
                "slug": slug,
                "title": title,
                "file_path": relative_path(engine, &note_path)
            }),
        ));
    }
    let summary = normalize_multiline_text(input.summary.as_deref().unwrap_or_default());
    let tags = normalize_list(input.tags);
    let timestamp = now_timestamp_string();
    let id = create_node_id(engine, &node_type, &slug);
    let mut note = canonical_note_from_parts(
        &engine.memory_root,
        &engine.context.project_name(),
        CanonicalNoteParts {
            node_type,
            slug,
            title,
            status,
            summary,
            tags,
            aliases,
            id,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        },
    );
    let project_title = current_project_hub_title(engine);
    ensure_machine_alias(&mut note);
    ensure_required_system_links(&mut note, &project_title);
    persist_new_note(engine, &note_path, &note)?;
    finalize_sync_after_create(engine, &note)
}

pub(super) fn add_observation(
    engine: &Engine,
    input: AddObservationInput,
) -> std::result::Result<ObservationWriteResult, WriteFailure> {
    let resolved = resolve_node(engine, &input.node)?;
    ensure_mutable_node(&resolved)?;
    let mut note = load_note(engine, &resolved)?;
    let observation = normalize_required_inline(&input.content, "content")?;
    let normalized = normalize_for_dedupe(&observation);
    let added = if note
        .observations
        .iter()
        .any(|item| normalize_for_dedupe(item) == normalized)
    {
        false
    } else {
        note.observations.push(observation);
        true
    };
    if !added {
        return Ok(ObservationWriteResult {
            node: node_ref(&note),
            added: false,
            sync_status: "unchanged".to_string(),
        });
    }
    note.frontmatter.updated_at = now_timestamp_string();
    ensure_machine_alias(&mut note);
    ensure_required_system_links(&mut note, &current_project_hub_title(engine));
    persist_existing_note(engine, &mut note, &resolved)?;
    finalize_sync_after_observation(engine, &note, true)
}

pub(super) fn link_nodes(
    engine: &Engine,
    input: LinkNodesInput,
) -> std::result::Result<LinkNodesResult, WriteFailure> {
    mutate_link(engine, input, true)
}

pub(super) fn unlink_nodes(
    engine: &Engine,
    input: LinkNodesInput,
) -> std::result::Result<LinkNodesResult, WriteFailure> {
    mutate_link(engine, input, false)
}

pub(super) fn update_node(
    engine: &Engine,
    input: UpdateNodeInput,
) -> std::result::Result<UpdateNodeResult, WriteFailure> {
    let resolved = resolve_node(engine, &input.node)?;
    ensure_mutable_node(&resolved)?;
    let mut note = load_note(engine, &resolved)?;
    let mut changed = false;
    let original_title = note.frontmatter.title.clone();

    if let Some(title) = input.title.as_deref() {
        let title = normalize_required_text(title, "title")?;
        if note.frontmatter.title != title {
            note.frontmatter.title = title;
            changed = true;
        }
    }
    if let Some(status) = input.status.as_deref() {
        let status = normalize_required_text(status, "status")?;
        if note.frontmatter.status != status {
            note.frontmatter.status = status;
            changed = true;
        }
    }
    if let Some(summary) = input.summary.as_deref() {
        let summary = normalize_multiline_text(summary);
        if note.summary != summary {
            note.summary = summary;
            changed = true;
        }
    }
    if let Some(tags) = input.tags {
        let tags = normalize_list(tags);
        if note.frontmatter.tags != tags {
            note.frontmatter.tags = tags;
            changed = true;
        }
    }
    if let Some(aliases) = input.aliases {
        let aliases = normalize_list(aliases);
        if note.frontmatter.aliases != aliases {
            note.frontmatter.aliases = aliases;
            changed = true;
        }
    }

    if changed {
        ensure_no_duplicate_candidate_for_update(
            engine,
            &note.slug,
            &note.frontmatter.node_type,
            &note.frontmatter.title,
            &note.frontmatter.aliases,
        )?;
        ensure_machine_alias(&mut note);
        let project_title = if note.slug == PROJECT_NODE_SLUG {
            note.frontmatter.title.clone()
        } else {
            current_project_hub_title(engine)
        };
        ensure_required_system_links(&mut note, &project_title);
    }

    if !changed {
        return Ok(UpdateNodeResult {
            node: node_ref(&note),
            changed: false,
            sync_status: "unchanged".to_string(),
        });
    }

    note.frontmatter.updated_at = now_timestamp_string();
    persist_existing_note(engine, &mut note, &resolved)?;
    if note.frontmatter.title != original_title {
        refresh_relation_titles(
            engine,
            &note.slug,
            &note.frontmatter.title,
            Some(&note.slug),
        )?;
    }
    finalize_sync_after_update(engine, &note, true)
}

fn mutate_link(
    engine: &Engine,
    input: LinkNodesInput,
    add: bool,
) -> std::result::Result<LinkNodesResult, WriteFailure> {
    let source = resolve_node(engine, &input.source)?;
    let target = resolve_node(engine, &input.target)?;
    let relation_kind = normalize_relation_kind(&input.relation_kind).ok_or_else(|| {
        unsupported_patch(
            "unsupported relation kind",
            json!({ "relation_kind": input.relation_kind }),
        )
    })?;
    if !add
        && relation_kind == "documents"
        && is_required_documents_target(&source.node_type, &target.slug)
    {
        return Err(WriteFailure::new(
            "E_REQUIRED_LINK",
            "system graph link is required for canonical notes",
            json!({
                "source": source.slug,
                "target": target.slug,
                "relation_kind": relation_kind
            }),
        ));
    }
    ensure_mutable_node(&source)?;
    ensure_mutable_link_target(&target)?;
    let mut note = load_note(engine, &source)?;

    let before = note.relations.len();
    if add {
        if !note.relations.iter().any(|relation| {
            relation.relation_kind == relation_kind && relation.target_slug == target.slug
        }) {
            note.relations.push(ParsedRelation {
                relation_kind: relation_kind.clone(),
                target_slug: target.slug.clone(),
                target_title: target.title.clone(),
            });
        }
    } else {
        note.relations.retain(|relation| {
            !(relation.relation_kind == relation_kind && relation.target_slug == target.slug)
        });
    }
    ensure_machine_alias(&mut note);
    ensure_required_system_links(&mut note, &current_project_hub_title(engine));
    let changed = note.relations.len() != before;
    if !changed {
        return Ok(LinkNodesResult {
            source: node_ref(&note),
            target: resolved_ref(&target),
            relation_kind,
            changed: false,
            sync_status: "unchanged".to_string(),
        });
    }

    note.frontmatter.updated_at = now_timestamp_string();
    persist_existing_note(engine, &mut note, &source)?;
    finalize_sync_after_link(engine, &note, &target, &relation_kind)
}

fn finalize_sync_after_create(
    engine: &Engine,
    note: &CanonicalNote,
) -> std::result::Result<CreateNodeResult, WriteFailure> {
    sync_after_write(engine).map_err(|err| {
        sync_after_write_failure(
            err,
            json!({
                "node": node_ref(note),
                "sync_status": "failed_after_write"
            }),
        )
    })?;
    Ok(CreateNodeResult {
        node: node_ref(note),
        sync_status: "synced".to_string(),
    })
}

fn finalize_sync_after_observation(
    engine: &Engine,
    note: &CanonicalNote,
    added: bool,
) -> std::result::Result<ObservationWriteResult, WriteFailure> {
    sync_after_write(engine).map_err(|err| {
        sync_after_write_failure(
            err,
            json!({
                "node": node_ref(note),
                "added": added,
                "sync_status": "failed_after_write"
            }),
        )
    })?;
    Ok(ObservationWriteResult {
        node: node_ref(note),
        added,
        sync_status: "synced".to_string(),
    })
}

fn finalize_sync_after_update(
    engine: &Engine,
    note: &CanonicalNote,
    changed: bool,
) -> std::result::Result<UpdateNodeResult, WriteFailure> {
    sync_after_write(engine).map_err(|err| {
        sync_after_write_failure(
            err,
            json!({
                "node": node_ref(note),
                "changed": changed,
                "sync_status": "failed_after_write"
            }),
        )
    })?;
    Ok(UpdateNodeResult {
        node: node_ref(note),
        changed,
        sync_status: "synced".to_string(),
    })
}

fn finalize_sync_after_link(
    engine: &Engine,
    note: &CanonicalNote,
    target: &ResolvedNode,
    relation_kind: &str,
) -> std::result::Result<LinkNodesResult, WriteFailure> {
    sync_after_write(engine).map_err(|err| {
        sync_after_write_failure(
            err,
            json!({
                "source": node_ref(note),
                "target": resolved_ref(target),
                "relation_kind": relation_kind,
                "changed": true,
                "sync_status": "failed_after_write"
            }),
        )
    })?;
    Ok(LinkNodesResult {
        source: node_ref(note),
        target: resolved_ref(target),
        relation_kind: relation_kind.to_string(),
        changed: true,
        sync_status: "synced".to_string(),
    })
}

fn persist_new_note(
    engine: &Engine,
    path: &Path,
    note: &CanonicalNote,
) -> std::result::Result<(), WriteFailure> {
    let rendered = render_note(note);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| runtime_failure(err, "failed to create canonical directory"))?;
    }
    write_rendered(path, &rendered)
        .map_err(|err| runtime_failure(err, "failed to write canonical note"))?;
    let _ = engine;
    Ok(())
}

fn persist_existing_note(
    engine: &Engine,
    note: &mut CanonicalNote,
    resolved: &ResolvedNode,
) -> std::result::Result<(), WriteFailure> {
    let current_path = engine.memory_root.join(&resolved.file_path);
    let current_raw = std::fs::read_to_string(&current_path)
        .map_err(|err| runtime_failure(err, "failed to read note during conflict check"))?;
    let current_hash = hash_bytes(current_raw.as_bytes());
    let current_mtime = std::fs::metadata(&current_path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or_default();
    if current_hash != note.raw_hash || current_mtime != note.file_mtime_ms {
        return Err(WriteFailure::new(
            "E_CONFLICT",
            format!(
                "canonical note `{}` changed on disk during mutation",
                resolved.slug
            ),
            json!({ "slug": resolved.slug, "file_path": resolved.file_path }),
        ));
    }
    let next_path = canonical_note_path(
        &engine.memory_root,
        &note.frontmatter.node_type,
        &note.frontmatter.title,
        &note.frontmatter.status,
    );
    if next_path != current_path && next_path.exists() {
        return Err(WriteFailure::new(
            "E_ALREADY_EXISTS",
            format!("canonical note already exists at {}", next_path.display()),
            json!({
                "slug": note.slug,
                "file_path": relative_path(engine, &next_path)
            }),
        ));
    }
    if let Some(parent) = next_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| runtime_failure(err, "failed to create canonical directory"))?;
    }
    note.file_path = relative_path(engine, &next_path);
    let rendered = render_note(note);
    note.raw_hash = hash_bytes(rendered.as_bytes());
    write_rendered(&next_path, &rendered)
        .map_err(|err| runtime_failure(err, "failed to write canonical note"))?;
    if next_path != current_path && current_path.exists() {
        std::fs::remove_file(&current_path)
            .map_err(|err| runtime_failure(err, "failed to remove previous canonical note"))?;
        cleanup_empty_directory(current_path.parent());
    }
    note.file_mtime_ms = std::fs::metadata(&next_path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or_default();
    Ok(())
}

fn write_rendered(path: &Path, rendered: &str) -> Result<()> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("note.md");
    let temp_path = unique_sibling_path(path, &format!("{file_name}.tmp"));
    std::fs::write(&temp_path, rendered)?;
    replace_with_temp(path, &temp_path)?;
    Ok(())
}

#[cfg(not(windows))]
fn replace_with_temp(path: &Path, temp_path: &Path) -> Result<()> {
    match std::fs::rename(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            std::fs::copy(&temp_path, path)?;
            std::fs::remove_file(&temp_path)?;
            Ok(())
        }
    }
}

#[cfg(windows)]
fn replace_with_temp(path: &Path, temp_path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::rename(temp_path, path)?;
        return Ok(());
    }

    let backup_path = unique_sibling_path(path, "replace-backup");
    std::fs::rename(path, &backup_path)?;
    match std::fs::rename(temp_path, path) {
        Ok(()) => {
            let _ = std::fs::remove_file(&backup_path);
            Ok(())
        }
        Err(rename_err) => {
            let restore_result = std::fs::rename(&backup_path, path);
            let _ = std::fs::remove_file(temp_path);
            match restore_result {
                Ok(()) => Err(rename_err.into()),
                Err(restore_err) => Err(anyhow::anyhow!(
                    "failed to replace {}; original was preserved at {}; replace error: {}; restore error: {}",
                    path.display(),
                    backup_path.display(),
                    rename_err,
                    restore_err
                )),
            }
        }
    }
}

fn unique_sibling_path(path: &Path, prefix: &str) -> PathBuf {
    let mut attempt = 0usize;
    loop {
        let suffix = if attempt == 0 {
            now_timestamp_string()
        } else {
            format!("{}-{attempt}", now_timestamp_string())
        };
        let candidate = path.with_file_name(format!("{prefix}-{suffix}"));
        if !candidate.exists() {
            return candidate;
        }
        attempt += 1;
    }
}

fn cleanup_empty_directory(path: Option<&Path>) {
    let Some(path) = path else {
        return;
    };
    let _ = std::fs::remove_dir(path);
}

fn canonical_files(memory_root: &Path) -> Result<Vec<PathBuf>> {
    scan_markdown_files(memory_root)
}

fn ensure_no_duplicate_candidate(
    engine: &Engine,
    node_type: &str,
    slug: &str,
    title: &str,
    aliases: &[String],
) -> std::result::Result<(), WriteFailure> {
    let candidates = duplicate_candidates(engine, node_type, slug, title, aliases)?;
    if candidates.is_empty() {
        return Ok(());
    }
    let detail_candidates = serde_json::to_value(&candidates).unwrap_or_else(|_| json!([]));
    Err(WriteFailure::new(
        "E_DUPLICATE_CANDIDATE",
        format!(
            "create_node found {} existing canonical node(s) that should be updated instead",
            candidates.len()
        ),
        json!({
            "node_type": node_type,
            "slug": slug,
            "title": title,
            "candidates": detail_candidates
        }),
    ))
}

fn ensure_no_duplicate_candidate_for_update(
    engine: &Engine,
    current_slug: &str,
    node_type: &str,
    title: &str,
    aliases: &[String],
) -> std::result::Result<(), WriteFailure> {
    let candidates = duplicate_candidates(engine, node_type, current_slug, title, aliases)?
        .into_iter()
        .filter(|candidate| candidate.slug != current_slug)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Ok(());
    }
    let detail_candidates = serde_json::to_value(&candidates).unwrap_or_else(|_| json!([]));
    Err(WriteFailure::new(
        "E_DUPLICATE_CANDIDATE",
        format!(
            "update_node would collide with {} other canonical node(s)",
            candidates.len()
        ),
        json!({
            "node_type": node_type,
            "slug": current_slug,
            "title": title,
            "candidates": detail_candidates
        }),
    ))
}

fn duplicate_candidates(
    engine: &Engine,
    node_type: &str,
    slug: &str,
    title: &str,
    aliases: &[String],
) -> std::result::Result<Vec<DuplicateCandidate>, WriteFailure> {
    let wanted_title = normalize_lookup_text(title);
    let wanted_aliases = aliases
        .iter()
        .map(|alias| normalize_lookup_text(alias))
        .filter(|alias| !alias.is_empty())
        .collect::<Vec<_>>();
    let mut candidates = BTreeMap::<String, DuplicateCandidate>::new();

    for path in canonical_files(&engine.memory_root)
        .map_err(|err| runtime_failure(err, "failed to scan canonical locations"))?
    {
        let Ok(note) = parse_canonical_note(&engine.memory_root, &path, None) else {
            continue;
        };
        let note_status = normalize_status(&note.status);
        let note_title = normalize_lookup_text(&note.title);
        let note_aliases = note
            .aliases
            .iter()
            .map(|alias| normalize_lookup_text(alias))
            .filter(|alias| !alias.is_empty())
            .collect::<Vec<_>>();
        let mut reasons = Vec::new();

        if note.slug == slug {
            reasons.push("exact_slug");
        }

        if note.node_type == node_type {
            if !wanted_title.is_empty() && note_title == wanted_title {
                reasons.push("normalized_title");
            }
            if alias_overlap(&wanted_title, &wanted_aliases, &note_title, &note_aliases) {
                reasons.push("alias_match");
            }
        }

        if reasons.is_empty() {
            continue;
        }

        let exact_slug = reasons.contains(&"exact_slug");
        let current_conflict = note_status != NormalizedStatus::Closed;
        if !exact_slug && !current_conflict {
            continue;
        }

        let why_matched = reasons.join(", ");
        candidates.insert(
            note.slug.clone(),
            DuplicateCandidate {
                id: note.id.clone(),
                slug: note.slug.clone(),
                title: note.title.clone(),
                node_type: note.node_type.clone(),
                status: note.status.clone(),
                summary: note.summary.clone(),
                why_matched,
            },
        );
    }

    Ok(candidates.into_values().collect())
}

fn alias_overlap(
    wanted_title: &str,
    wanted_aliases: &[String],
    note_title: &str,
    note_aliases: &[String],
) -> bool {
    (!wanted_title.is_empty() && note_aliases.iter().any(|alias| alias == wanted_title))
        || (!note_title.is_empty() && wanted_aliases.iter().any(|alias| alias == note_title))
        || wanted_aliases
            .iter()
            .any(|alias| note_aliases.iter().any(|note_alias| note_alias == alias))
}

fn ensure_mutable_node(resolved: &ResolvedNode) -> std::result::Result<(), WriteFailure> {
    if is_system_node_type(&resolved.node_type) {
        return Err(system_node_type_failure(
            "section hubs are system-managed and cannot be edited directly",
            json!({
                "node": resolved.slug,
                "node_type": resolved.node_type
            }),
        ));
    }
    Ok(())
}

fn ensure_mutable_link_target(target: &ResolvedNode) -> std::result::Result<(), WriteFailure> {
    if is_system_node_type(&target.node_type) {
        return Err(system_node_type_failure(
            "section hubs are system-managed and cannot be linked directly",
            json!({
                "target": target.slug,
                "node_type": target.node_type
            }),
        ));
    }
    Ok(())
}

fn is_required_documents_target(source_node_type: &str, target_slug: &str) -> bool {
    target_slug == PROJECT_NODE_SLUG
        || required_section_hub_slug(source_node_type).is_some_and(|slug| slug == target_slug)
}

fn current_project_hub_title(engine: &Engine) -> String {
    scan_markdown_files(&engine.memory_root)
        .ok()
        .into_iter()
        .flatten()
        .find_map(|path| {
            parse_canonical_note(&engine.memory_root, &path, Some("project"))
                .ok()
                .map(|note| note.title)
        })
        .unwrap_or_else(|| humanize_project_name(&engine.context.project_name()))
}

fn ensure_machine_alias(note: &mut CanonicalNote) {
    upsert_alias(&mut note.frontmatter.aliases, &note.slug);
}

fn ensure_required_system_links(note: &mut CanonicalNote, project_title: &str) {
    if note.frontmatter.node_type == "project"
        || note.frontmatter.node_type == SECTION_HUB_NODE_TYPE
    {
        return;
    }
    if let Some(existing) = note.relations.iter_mut().find(|relation| {
        relation.relation_kind == "documents" && relation.target_slug == PROJECT_NODE_SLUG
    }) {
        existing.target_title = project_title.to_string();
    } else {
        note.relations.push(ParsedRelation {
            relation_kind: "documents".to_string(),
            target_slug: PROJECT_NODE_SLUG.to_string(),
            target_title: project_title.to_string(),
        });
    }
    if let Some(section_slug) = required_section_hub_slug(&note.frontmatter.node_type) {
        let section_title =
            section_hub_title_for_node_type(project_title, &note.frontmatter.node_type)
                .unwrap_or_else(|| section_slug.to_string());
        if let Some(existing) = note.relations.iter_mut().find(|relation| {
            relation.relation_kind == "documents" && relation.target_slug == section_slug
        }) {
            existing.target_title = section_title;
        } else {
            note.relations.push(ParsedRelation {
                relation_kind: "documents".to_string(),
                target_slug: section_slug.to_string(),
                target_title: section_title,
            });
        }
    }
}

fn refresh_relation_titles(
    engine: &Engine,
    target_slug: &str,
    target_title: &str,
    skip_slug: Option<&str>,
) -> std::result::Result<(), WriteFailure> {
    for path in canonical_files(&engine.memory_root)
        .map_err(|err| runtime_failure(err, "failed to scan canonical locations"))?
    {
        let Ok(mut note) = parse_canonical_document(&engine.memory_root, &path, None) else {
            continue;
        };
        if skip_slug.is_some_and(|slug| note.slug == slug) {
            continue;
        }
        let mut changed = false;
        for relation in &mut note.relations {
            if relation.target_slug == target_slug && relation.target_title != target_title {
                relation.target_title = target_title.to_string();
                changed = true;
            }
        }
        if !changed {
            continue;
        }
        note.frontmatter.updated_at = now_timestamp_string();
        let resolved = ResolvedNode {
            id: note.frontmatter.id.clone(),
            slug: note.slug.clone(),
            title: note.frontmatter.title.clone(),
            node_type: note.frontmatter.node_type.clone(),
            file_path: note.file_path.clone(),
        };
        persist_existing_note(engine, &mut note, &resolved)?;
    }
    Ok(())
}

fn upsert_alias(aliases: &mut Vec<String>, alias: &str) {
    let alias = alias.trim();
    if alias.is_empty() {
        return;
    }
    if aliases.iter().any(|existing| existing == alias) {
        return;
    }
    aliases.push(alias.to_string());
}

fn resolve_node(engine: &Engine, token: &str) -> std::result::Result<ResolvedNode, WriteFailure> {
    let conn = engine
        .open_connection()
        .map_err(|err| runtime_failure(err, "failed to open index for write resolver"))?;
    if let Some(node) = resolve_by_column(&conn, "id", token)? {
        return Ok(node);
    }
    if let Some(node) = resolve_by_column(&conn, "slug", token)? {
        return Ok(node);
    }
    if let Some(node) = resolve_by_lookup(&conn, token)? {
        return Ok(node);
    }
    Err(WriteFailure::new(
        "E_NOT_FOUND",
        format!("node `{token}` not found by id, slug, title, or alias"),
        json!({ "node": token }),
    ))
}

fn resolve_by_column(
    conn: &Connection,
    column: &str,
    token: &str,
) -> std::result::Result<Option<ResolvedNode>, WriteFailure> {
    let sql =
        format!("SELECT id, slug, title, node_type, file_path FROM notes WHERE {column} = ?1");
    let mut statement = conn
        .prepare(&sql)
        .map_err(|err| runtime_failure(err, "failed to prepare resolver query"))?;
    let rows = statement
        .query_map([token], |row| {
            Ok(ResolvedNode {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                node_type: row.get(3)?,
                file_path: row.get(4)?,
            })
        })
        .map_err(|err| runtime_failure(err, "failed to execute resolver query"))?;
    let items = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|err| runtime_failure(err, "failed to collect resolver rows"))?;
    if items.len() > 1 {
        return Err(WriteFailure::new(
            "E_AMBIGUOUS_TARGET",
            format!("node `{token}` matched multiple records by {column}"),
            json!({ "node": token, "column": column }),
        ));
    }
    Ok(items.into_iter().next())
}

fn resolve_by_lookup(
    conn: &Connection,
    token: &str,
) -> std::result::Result<Option<ResolvedNode>, WriteFailure> {
    let lookup = normalize_lookup_text(token);
    if lookup.is_empty() {
        return Ok(None);
    }
    let mut statement = conn
        .prepare(
            "SELECT DISTINCT n.id, n.slug, n.title, n.node_type, n.file_path
             FROM notes n
             LEFT JOIN note_aliases a ON a.slug = n.slug
             WHERE lower(trim(n.title)) = ?1 OR lower(trim(a.alias)) = ?1
             ORDER BY n.slug",
        )
        .map_err(|err| runtime_failure(err, "failed to prepare resolver lookup query"))?;
    let rows = statement
        .query_map([lookup], |row| {
            Ok(ResolvedNode {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                node_type: row.get(3)?,
                file_path: row.get(4)?,
            })
        })
        .map_err(|err| runtime_failure(err, "failed to execute resolver lookup query"))?;
    let items = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|err| runtime_failure(err, "failed to collect resolver lookup rows"))?;
    if items.len() > 1 {
        return Err(WriteFailure::new(
            "E_AMBIGUOUS_TARGET",
            format!("node `{token}` matched multiple records by title or alias"),
            json!({ "node": token, "column": "title_or_alias" }),
        ));
    }
    Ok(items.into_iter().next())
}

fn load_note(
    engine: &Engine,
    resolved: &ResolvedNode,
) -> std::result::Result<CanonicalNote, WriteFailure> {
    let path = engine.memory_root.join(&resolved.file_path);
    parse_canonical_document(&engine.memory_root, &path, Some(&resolved.node_type)).map_err(|err| {
        WriteFailure::new(
            "E_MALFORMED_NOTE",
            format!("failed to parse canonical note `{}`: {err}", resolved.slug),
            json!({ "slug": resolved.slug, "file_path": resolved.file_path }),
        )
    })
}

fn node_ref(note: &CanonicalNote) -> NodeWriteRef {
    NodeWriteRef {
        id: note.frontmatter.id.clone(),
        slug: note.slug.clone(),
        title: note.frontmatter.title.clone(),
        node_type: note.frontmatter.node_type.clone(),
        file_path: note.file_path.clone(),
    }
}

fn resolved_ref(node: &ResolvedNode) -> NodeWriteRef {
    NodeWriteRef {
        id: node.id.clone(),
        slug: node.slug.clone(),
        title: node.title.clone(),
        node_type: node.node_type.clone(),
        file_path: node.file_path.clone(),
    }
}

fn create_node_id(engine: &Engine, node_type: &str, slug: &str) -> String {
    if node_type == "project" {
        let project = normalize_slug(&engine.context.project_name())
            .unwrap_or_else(|| "memory-project".to_string());
        format!("project-{project}")
    } else {
        format!("{node_type}-{slug}")
    }
}

fn normalize_required_text(value: &str, field: &str) -> std::result::Result<String, WriteFailure> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(unsupported_patch(
            format!("{field} must be non-empty"),
            json!({ "field": field }),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_required_inline(
    value: &str,
    field: &str,
) -> std::result::Result<String, WriteFailure> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    normalize_required_text(&normalized, field)
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_multiline_text(value: &str) -> String {
    value
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn normalize_list(items: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !out.iter().any(|existing| existing == trimmed) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn normalize_for_dedupe(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_lookup_text(value: &str) -> String {
    normalize_for_dedupe(value).to_ascii_lowercase()
}

fn relative_path(engine: &Engine, path: &Path) -> String {
    path.strip_prefix(&engine.memory_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn unsupported_patch(message: impl Into<String>, details: Value) -> WriteFailure {
    WriteFailure::new("E_UNSUPPORTED_PATCH", message, details)
}

fn system_node_type_failure(message: impl Into<String>, details: Value) -> WriteFailure {
    WriteFailure::new("E_SYSTEM_NODE_TYPE", message, details)
}

fn sync_after_write_failure(err: anyhow::Error, details: Value) -> WriteFailure {
    WriteFailure::new(
        "E_SYNC_AFTER_WRITE",
        format!("canonical Markdown changed but derived index sync failed: {err}"),
        json!({
            "recovery_hint": "run rebuild_index to resync derived state from Markdown truth",
            "reason": err.to_string(),
            "result": details
        }),
    )
}

fn runtime_failure(err: impl Display, message: &str) -> WriteFailure {
    WriteFailure::new("E_RUNTIME", format!("{message}: {err}"), json!({}))
}

#[cfg(test)]
pub(super) fn set_force_sync_failure(enabled: bool) {
    FORCE_SYNC_FAILURE.store(enabled, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use std::sync::{LazyLock, Mutex};

    use crate::engine::Engine;
    use crate::model::{
        AddObservationInput, CreateNodeInput, LinkNodesInput, StorageMode, UpdateNodeInput,
    };

    use super::{PROJECT_NODE_SLUG, set_force_sync_failure, write_rendered};

    static TEST_GUARD: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn create_node_and_update_flow_work() {
        let _guard = TEST_GUARD.lock().expect("test guard");
        let root = tempfile_dir("write-flow");
        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

        let created = engine
            .create_node(CreateNodeInput {
                node_type: "Decision".to_string(),
                title: "Auth Token".to_string(),
                slug: Some("auth-token".to_string()),
                status: Some("active".to_string()),
                summary: Some("Short summary.".to_string()),
                tags: vec!["auth".to_string()],
                aliases: vec!["Token Strategy".to_string()],
            })
            .expect("create node");
        assert_eq!(created.node.slug, "auth-token");
        assert_eq!(created.node.file_path, "decisions/Auth Token.md");

        let project = engine
            .update_node(UpdateNodeInput {
                node: PROJECT_NODE_SLUG.to_string(),
                title: Some("Workspace Hub".to_string()),
                status: None,
                summary: Some("Hub summary.".to_string()),
                tags: Some(vec!["memory".to_string()]),
                aliases: None,
            })
            .expect("update project hub");
        assert_eq!(project.node.file_path, "Workspace Hub.md");

        let updated = engine
            .update_node(UpdateNodeInput {
                node: "auth-token".to_string(),
                title: Some("Auth Token Decision".to_string()),
                status: None,
                summary: Some("Updated summary.".to_string()),
                tags: Some(vec!["security".to_string()]),
                aliases: None,
            })
            .expect("update node");
        assert!(updated.changed);

        let opened = engine
            .open_nodes(std::slice::from_ref(&created.node.slug))
            .expect("open");
        assert_eq!(opened[0].title, "Auth Token Decision");
        assert_eq!(opened[0].summary, "Updated summary.");
        assert_eq!(opened[0].file_path, "decisions/Auth Token Decision.md");
        assert!(opened[0].relations.iter().any(|relation| {
            relation.relation_kind == "documents" && relation.target_slug == PROJECT_NODE_SLUG
        }));
        assert!(opened[0].relations.iter().any(|relation| {
            relation.relation_kind == "documents" && relation.target_slug == "section-decisions"
        }));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn add_observation_and_link_are_idempotent() {
        let _guard = TEST_GUARD.lock().expect("test guard");
        let root = tempfile_dir("write-links");
        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
        let create = |title: &str, slug: &str| {
            engine
                .create_node(CreateNodeInput {
                    node_type: "Task".to_string(),
                    title: title.to_string(),
                    slug: Some(slug.to_string()),
                    status: None,
                    summary: None,
                    tags: Vec::new(),
                    aliases: Vec::new(),
                })
                .expect("create task")
        };
        create("Source Task", "source-task");
        create("Target Task", "target-task");

        let first = engine
            .add_observation(AddObservationInput {
                node: "source-task".to_string(),
                content: "Observed change".to_string(),
            })
            .expect("add observation");
        let second = engine
            .add_observation(AddObservationInput {
                node: "source-task".to_string(),
                content: "Observed   change".to_string(),
            })
            .expect("dedupe observation");
        assert!(first.added);
        assert!(!second.added);

        let link = engine
            .link_nodes(LinkNodesInput {
                source: "source-task".to_string(),
                target: "target-task".to_string(),
                relation_kind: "depends_on".to_string(),
            })
            .expect("link");
        let noop = engine
            .link_nodes(LinkNodesInput {
                source: "source-task".to_string(),
                target: "target-task".to_string(),
                relation_kind: "depends_on".to_string(),
            })
            .expect("link noop");
        assert!(link.changed);
        assert!(!noop.changed);

        let unlink = engine
            .unlink_nodes(LinkNodesInput {
                source: "source-task".to_string(),
                target: "target-task".to_string(),
                relation_kind: "depends_on".to_string(),
            })
            .expect("unlink");
        assert!(unlink.changed);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn create_node_blocks_current_duplicates_and_allows_historical_replacement() {
        let _guard = TEST_GUARD.lock().expect("test guard");
        let root = tempfile_dir("write-duplicates");
        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

        engine
            .create_node(CreateNodeInput {
                node_type: "Task".to_string(),
                title: "Shared Task".to_string(),
                slug: Some("shared-task".to_string()),
                status: Some("active".to_string()),
                summary: Some("Current summary".to_string()),
                tags: Vec::new(),
                aliases: vec!["Canonical Shared Task".to_string()],
            })
            .expect("create seed task");

        let by_slug = engine.create_node(CreateNodeInput {
            node_type: "Task".to_string(),
            title: "Different Title".to_string(),
            slug: Some("shared-task".to_string()),
            status: Some("active".to_string()),
            summary: None,
            tags: Vec::new(),
            aliases: Vec::new(),
        });
        let slug_failure = by_slug.expect_err("duplicate slug");
        assert_eq!(slug_failure.code, "E_DUPLICATE_CANDIDATE");
        assert_eq!(
            slug_failure.details["candidates"][0]["why_matched"],
            "exact_slug"
        );

        let by_title = engine.create_node(CreateNodeInput {
            node_type: "Task".to_string(),
            title: "Shared   Task".to_string(),
            slug: Some("shared-task-v2".to_string()),
            status: Some("active".to_string()),
            summary: None,
            tags: Vec::new(),
            aliases: Vec::new(),
        });
        let title_failure = by_title.expect_err("duplicate title");
        assert_eq!(title_failure.code, "E_DUPLICATE_CANDIDATE");
        assert!(
            title_failure.details["candidates"][0]["why_matched"]
                .as_str()
                .is_some_and(|why| why.contains("normalized_title"))
        );

        engine
            .update_node(UpdateNodeInput {
                node: "shared-task".to_string(),
                title: None,
                status: Some("superseded".to_string()),
                summary: None,
                tags: None,
                aliases: None,
            })
            .expect("mark historical");

        let replacement = engine
            .create_node(CreateNodeInput {
                node_type: "Task".to_string(),
                title: "Shared Task".to_string(),
                slug: Some("shared-task-v2".to_string()),
                status: Some("active".to_string()),
                summary: Some("Replacement summary".to_string()),
                tags: Vec::new(),
                aliases: Vec::new(),
            })
            .expect("create replacement");
        assert_eq!(replacement.node.slug, "shared-task-v2");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn detects_conflict_and_sync_failure() {
        let _guard = TEST_GUARD.lock().expect("test guard");
        let root = tempfile_dir("write-conflict");
        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
        let conflicted = engine
            .create_node(CreateNodeInput {
                node_type: "Risk".to_string(),
                title: "Token Leak".to_string(),
                slug: Some("token-leak".to_string()),
                status: None,
                summary: None,
                tags: Vec::new(),
                aliases: Vec::new(),
            })
            .expect("create risk");

        let note_path = std::path::Path::new(&root)
            .join("memory")
            .join("risks")
            .join("Token Leak.md");
        let on_disk = std::fs::read_to_string(&note_path).expect("read");
        let rewritten = on_disk.replace("## Summary", "manual change\n\n## Summary");
        write_rendered(&note_path, &rewritten).expect("manual mutate");
        let conflict = engine.update_node(UpdateNodeInput {
            node: conflicted.node.slug.clone(),
            title: None,
            status: Some("accepted".to_string()),
            summary: None,
            tags: None,
            aliases: None,
        });
        assert_eq!(conflict.expect_err("conflict").code, "E_MALFORMED_NOTE");
        write_rendered(&note_path, &on_disk).expect("restore canonical note");

        let sync_target = engine
            .create_node(CreateNodeInput {
                node_type: "Risk".to_string(),
                title: "Sync Failure".to_string(),
                slug: Some("sync-failure".to_string()),
                status: None,
                summary: None,
                tags: Vec::new(),
                aliases: Vec::new(),
            })
            .expect("create sync target");

        set_force_sync_failure(true);
        let sync_failure = engine.add_observation(AddObservationInput {
            node: sync_target.node.slug.clone(),
            content: "Needs follow-up".to_string(),
        });
        set_force_sync_failure(false);
        assert_eq!(
            sync_failure.expect_err("sync failure").code,
            "E_SYNC_AFTER_WRITE"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn section_hubs_are_system_managed_and_required_links_survive() {
        let _guard = TEST_GUARD.lock().expect("test guard");
        let root = tempfile_dir("write-section-hubs");
        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

        engine
            .create_node(CreateNodeInput {
                node_type: "Task".to_string(),
                title: "Shared Task".to_string(),
                slug: Some("shared-task".to_string()),
                status: Some("active".to_string()),
                summary: Some("Current task".to_string()),
                tags: Vec::new(),
                aliases: Vec::new(),
            })
            .expect("create task");

        let task = engine
            .open_nodes(&["shared-task".to_string()])
            .expect("open task");
        assert!(task[0].relations.iter().any(|relation| {
            relation.relation_kind == "documents" && relation.target_slug == "section-tasks"
        }));

        let required_unlink = engine.unlink_nodes(LinkNodesInput {
            source: "shared-task".to_string(),
            target: "section-tasks".to_string(),
            relation_kind: "documents".to_string(),
        });
        assert_eq!(
            required_unlink.expect_err("required link").code,
            "E_REQUIRED_LINK"
        );

        let system_update = engine.update_node(UpdateNodeInput {
            node: "section-tasks".to_string(),
            title: Some("Manual Tasks".to_string()),
            status: None,
            summary: None,
            tags: None,
            aliases: None,
        });
        assert_eq!(
            system_update.expect_err("system node").code,
            "E_SYSTEM_NODE_TYPE"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    fn tempfile_dir(prefix: &str) -> std::path::PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("{prefix}-{suffix}"));
        std::fs::create_dir_all(&root).expect("create root");
        root
    }
}
