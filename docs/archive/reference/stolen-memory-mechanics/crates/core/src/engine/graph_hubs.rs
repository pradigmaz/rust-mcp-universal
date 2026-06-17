use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::bootstrap::{NormalizedStatus, normalize_status};
use crate::utils::hash_bytes;

use super::canonical::{
    CanonicalNote, CanonicalNoteParts, PROJECT_NODE_SLUG, ParsedRelation, SECTION_HUB_NODE_TYPE,
    canonical_note_from_parts, canonical_note_path, normalize_slug, now_timestamp_string,
    parse_canonical_document, render_note,
};
use super::notes::scan_markdown_files;

#[derive(Debug, Clone, Copy)]
pub(super) struct SectionHubSpec {
    pub member_node_type: &'static str,
    pub slug: &'static str,
    pub label: &'static str,
}

const SECTION_HUB_SPECS: &[SectionHubSpec] = &[
    SectionHubSpec {
        member_node_type: "task",
        slug: "section-tasks",
        label: "Tasks",
    },
    SectionHubSpec {
        member_node_type: "risk",
        slug: "section-risks",
        label: "Risks",
    },
    SectionHubSpec {
        member_node_type: "decision",
        slug: "section-decisions",
        label: "Decisions",
    },
    SectionHubSpec {
        member_node_type: "module",
        slug: "section-modules",
        label: "Modules",
    },
    SectionHubSpec {
        member_node_type: "progress_entry",
        slug: "section-progress",
        label: "Progress",
    },
    SectionHubSpec {
        member_node_type: "architecture_note",
        slug: "section-architecture",
        label: "Architecture",
    },
    SectionHubSpec {
        member_node_type: "constraint",
        slug: "section-constraints",
        label: "Constraints",
    },
    SectionHubSpec {
        member_node_type: "artifact",
        slug: "section-artifacts",
        label: "Artifacts",
    },
    SectionHubSpec {
        member_node_type: "glossary_term",
        slug: "section-glossary",
        label: "Glossary",
    },
];

pub(super) fn section_hub_spec_for_node_type(node_type: &str) -> Option<&'static SectionHubSpec> {
    SECTION_HUB_SPECS
        .iter()
        .find(|spec| spec.member_node_type == node_type)
}

pub(super) fn required_section_hub_slug(node_type: &str) -> Option<&'static str> {
    section_hub_spec_for_node_type(node_type).map(|spec| spec.slug)
}

pub(super) fn section_hub_title_for_node_type(
    project_title: &str,
    node_type: &str,
) -> Option<String> {
    section_hub_spec_for_node_type(node_type).map(|spec| section_hub_title(project_title, spec))
}

pub(super) fn is_section_hub_slug(slug: &str) -> bool {
    SECTION_HUB_SPECS.iter().any(|spec| spec.slug == slug)
}

pub(super) fn is_system_node_type(node_type: &str) -> bool {
    node_type == SECTION_HUB_NODE_TYPE
}

pub(super) fn sync_graph_hubs(memory_root: &Path, project_name: &str) -> Result<()> {
    let files = scan_markdown_files(memory_root)?;
    let mut notes = files
        .iter()
        .map(|path| parse_canonical_document(memory_root, path, None))
        .collect::<Result<Vec<_>>>()?;

    let project_title = ensure_project_note(&mut notes, memory_root, project_name);
    ensure_section_hub_notes(&mut notes, memory_root, project_name, &project_title);

    let section_members = build_section_member_relations(&notes);
    for note in &mut notes {
        ensure_machine_alias(note);
        match note.frontmatter.node_type.as_str() {
            "project" => update_project_hub(note, &project_title),
            SECTION_HUB_NODE_TYPE => update_section_hub(note, &section_members, &project_title),
            _ => update_leaf_note(note, &project_title),
        }
    }

    let title_by_slug = notes
        .iter()
        .map(|note| (note.slug.clone(), note.frontmatter.title.clone()))
        .collect::<BTreeMap<_, _>>();
    for note in &mut notes {
        refresh_relation_titles(note, &title_by_slug, &project_title);
    }

    for note in &mut notes {
        persist_note(memory_root, note)?;
    }

    Ok(())
}

fn ensure_project_note(
    notes: &mut Vec<CanonicalNote>,
    memory_root: &Path,
    project_name: &str,
) -> String {
    if let Some(note) = notes
        .iter()
        .find(|note| note.frontmatter.node_type == "project")
    {
        return note.frontmatter.title.clone();
    }

    let title = humanize_project_name(project_name);
    let project_slug = normalize_slug(project_name).unwrap_or_else(|| "memory-project".to_string());
    notes.push(canonical_note_from_parts(
        memory_root,
        project_name,
        CanonicalNoteParts {
            node_type: "project".to_string(),
            slug: PROJECT_NODE_SLUG.to_string(),
            title: title.clone(),
            status: "active".to_string(),
            summary: format!(
                "Project hub for `{project_name}`. Use this note as central map for tasks, risks, decisions, modules, and progress."
            ),
            tags: vec!["project".to_string(), "mcp".to_string()],
            aliases: vec![PROJECT_NODE_SLUG.to_string()],
            id: format!("project-{project_slug}"),
            created_at: now_timestamp_string(),
            updated_at: now_timestamp_string(),
        },
    ));
    title
}

fn ensure_section_hub_notes(
    notes: &mut Vec<CanonicalNote>,
    memory_root: &Path,
    project_name: &str,
    project_title: &str,
) {
    let existing = notes
        .iter()
        .map(|note| note.slug.clone())
        .collect::<Vec<_>>();
    for spec in SECTION_HUB_SPECS {
        if existing.iter().any(|slug| slug == spec.slug) {
            continue;
        }
        notes.push(canonical_note_from_parts(
            memory_root,
            project_name,
            CanonicalNoteParts {
                node_type: SECTION_HUB_NODE_TYPE.to_string(),
                slug: spec.slug.to_string(),
                title: section_hub_title(project_title, spec),
                status: "active".to_string(),
                summary: section_hub_summary(spec),
                tags: section_hub_tags(spec),
                aliases: vec![spec.slug.to_string()],
                id: format!("section_hub-{}", spec.slug),
                created_at: now_timestamp_string(),
                updated_at: now_timestamp_string(),
            },
        ));
    }
}

fn build_section_member_relations(
    notes: &[CanonicalNote],
) -> BTreeMap<String, Vec<ParsedRelation>> {
    let mut by_hub = BTreeMap::<String, Vec<(u8, String, ParsedRelation)>>::new();
    for note in notes {
        let Some(spec) = section_hub_spec_for_node_type(&note.frontmatter.node_type) else {
            continue;
        };
        let group = match normalize_status(&note.frontmatter.status) {
            NormalizedStatus::Closed => 1,
            NormalizedStatus::Open | NormalizedStatus::Unknown => 0,
        };
        by_hub.entry(spec.slug.to_string()).or_default().push((
            group,
            note.frontmatter.title.to_ascii_lowercase(),
            ParsedRelation {
                relation_kind: "documents".to_string(),
                target_slug: note.slug.clone(),
                target_title: note.frontmatter.title.clone(),
            },
        ));
    }

    by_hub
        .into_iter()
        .map(|(slug, mut entries)| {
            entries.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
            (
                slug,
                entries
                    .into_iter()
                    .map(|(_, _, relation)| relation)
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn update_project_hub(note: &mut CanonicalNote, project_title: &str) {
    let required = SECTION_HUB_SPECS
        .iter()
        .map(|spec| ParsedRelation {
            relation_kind: "documents".to_string(),
            target_slug: spec.slug.to_string(),
            target_title: section_hub_title(project_title, spec),
        })
        .collect::<Vec<_>>();
    let mut others = note
        .relations
        .iter()
        .filter(|relation| {
            relation.relation_kind != "documents" || !is_section_hub_slug(&relation.target_slug)
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut next = required;
    next.append(&mut others);
    set_note_relations(note, next);
}

fn update_section_hub(
    note: &mut CanonicalNote,
    section_members: &BTreeMap<String, Vec<ParsedRelation>>,
    project_title: &str,
) {
    if let Some(spec) = section_hub_spec_for_slug(&note.slug) {
        let expected_title = section_hub_title(project_title, spec);
        if note.frontmatter.title != expected_title {
            note.frontmatter.title = expected_title;
            touch_note(note);
        }
        if note.frontmatter.status != "active" {
            note.frontmatter.status = "active".to_string();
            touch_note(note);
        }
        let expected_summary = section_hub_summary(spec);
        if note.summary != expected_summary {
            note.summary = expected_summary;
            touch_note(note);
        }
        let expected_tags = section_hub_tags(spec);
        if note.frontmatter.tags != expected_tags {
            note.frontmatter.tags = expected_tags;
            touch_note(note);
        }
        note.observations.clear();
        note.references.clear();
        ensure_machine_alias(note);

        let mut relations = vec![ParsedRelation {
            relation_kind: "documents".to_string(),
            target_slug: PROJECT_NODE_SLUG.to_string(),
            target_title: project_title.to_string(),
        }];
        if let Some(members) = section_members.get(spec.slug) {
            relations.extend(members.iter().cloned());
        }
        set_note_relations(note, relations);
    }
}

fn update_leaf_note(note: &mut CanonicalNote, project_title: &str) {
    let Some(spec) = section_hub_spec_for_node_type(&note.frontmatter.node_type) else {
        return;
    };
    let mut others = note
        .relations
        .iter()
        .filter(|relation| {
            !(relation.relation_kind == "documents"
                && (relation.target_slug == PROJECT_NODE_SLUG || relation.target_slug == spec.slug))
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut next = vec![
        ParsedRelation {
            relation_kind: "documents".to_string(),
            target_slug: PROJECT_NODE_SLUG.to_string(),
            target_title: project_title.to_string(),
        },
        ParsedRelation {
            relation_kind: "documents".to_string(),
            target_slug: spec.slug.to_string(),
            target_title: section_hub_title(project_title, spec),
        },
    ];
    next.append(&mut others);
    set_note_relations(note, next);
}

fn set_note_relations(note: &mut CanonicalNote, next: Vec<ParsedRelation>) {
    if note.relations != next {
        note.relations = next;
        touch_note(note);
    }
}

fn touch_note(note: &mut CanonicalNote) {
    note.frontmatter.updated_at = now_timestamp_string();
}

fn refresh_relation_titles(
    note: &mut CanonicalNote,
    title_by_slug: &BTreeMap<String, String>,
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

fn persist_note(memory_root: &Path, note: &mut CanonicalNote) -> Result<()> {
    let current_path = if note.file_path.is_empty() {
        None
    } else {
        Some(memory_root.join(&note.file_path))
    };
    let next_path = canonical_note_path(
        memory_root,
        &note.frontmatter.node_type,
        &note.frontmatter.title,
        &note.frontmatter.status,
    );
    let rendered = render_note(note);
    let next_hash = hash_bytes(rendered.as_bytes());
    let path_changed = current_path.as_ref().is_some_and(|path| path != &next_path);
    let should_write = path_changed
        || !next_path.exists()
        || note.raw_hash != next_hash
        || note.file_path.is_empty();
    if !should_write {
        return Ok(());
    }
    if let Some(parent) = next_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&next_path, &rendered)?;
    if let Some(previous) = current_path {
        if previous != next_path && previous.exists() {
            let _ = std::fs::remove_file(&previous);
            cleanup_empty_directory(previous.parent());
        }
    }
    note.file_path = next_path
        .strip_prefix(memory_root)
        .unwrap_or(&next_path)
        .to_string_lossy()
        .replace('\\', "/");
    note.raw_hash = next_hash;
    note.file_mtime_ms = std::fs::metadata(&next_path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or_default();
    Ok(())
}

fn cleanup_empty_directory(path: Option<&Path>) {
    let Some(path) = path else {
        return;
    };
    let _ = std::fs::remove_dir(path);
}

fn ensure_machine_alias(note: &mut CanonicalNote) {
    if !note
        .frontmatter
        .aliases
        .iter()
        .any(|alias| alias == &note.slug)
    {
        note.frontmatter.aliases.push(note.slug.clone());
        touch_note(note);
    }
}

fn section_hub_spec_for_slug(slug: &str) -> Option<&'static SectionHubSpec> {
    SECTION_HUB_SPECS.iter().find(|spec| spec.slug == slug)
}

fn section_hub_title(project_title: &str, spec: &SectionHubSpec) -> String {
    format!("{project_title} {}", spec.label)
}

fn section_hub_summary(spec: &SectionHubSpec) -> String {
    format!(
        "System hub for {} notes in this project.",
        spec.label.to_ascii_lowercase()
    )
}

fn section_hub_tags(spec: &SectionHubSpec) -> Vec<String> {
    vec![
        "section-hub".to_string(),
        spec.label.to_ascii_lowercase(),
        "system".to_string(),
    ]
}

pub(super) fn humanize_project_name(raw: &str) -> String {
    raw.split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut word = first.to_uppercase().collect::<String>();
            word.push_str(chars.as_str());
            word
        })
        .collect::<Vec<_>>()
        .join(" ")
}
