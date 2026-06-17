use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde_yaml::{Mapping, Value};

use crate::utils::hash_bytes;

pub(super) const CANONICAL_DIRECTORIES: &[(&str, &str)] = &[
    ("architecture", "architecture_note"),
    ("artifacts", "artifact"),
    ("constraints", "constraint"),
    ("decisions", "decision"),
    ("glossary", "glossary_term"),
    ("modules", "module"),
    ("progress", "progress_entry"),
    ("risks", "risk"),
    ("tasks", "task"),
];

pub(super) const PROJECT_NODE_SLUG: &str = "_index";
pub(super) const SECTION_HUB_NODE_TYPE: &str = "section_hub";

const BODY_SECTION_ORDER: &[&str] = &["Summary", "Observations", "Relations", "References"];
const RELATION_KINDS: &[&str] = &[
    "relates_to",
    "depends_on",
    "supersedes",
    "documents",
    "blocks",
    "implements",
    "affects",
    "owned_by",
    "derived_from",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedRelation {
    pub relation_kind: String,
    pub target_slug: String,
    pub target_title: String,
}

#[derive(Debug, Clone)]
pub(super) struct ParsedNote {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub node_type: String,
    pub status: String,
    pub project: String,
    pub file_path: String,
    pub summary: String,
    pub observations: Vec<String>,
    pub relations: Vec<ParsedRelation>,
    pub references: Vec<String>,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
    pub raw_hash: String,
    pub file_mtime_ms: i64,
}

#[derive(Debug, Clone)]
pub(super) struct FrontmatterFields {
    pub id: String,
    pub node_type: String,
    pub title: String,
    pub status: String,
    pub project: String,
    pub created_at: String,
    pub updated_at: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct CanonicalNote {
    pub frontmatter: FrontmatterFields,
    pub slug: String,
    pub file_path: String,
    pub summary: String,
    pub observations: Vec<String>,
    pub relations: Vec<ParsedRelation>,
    pub references: Vec<String>,
    pub raw_hash: String,
    pub file_mtime_ms: i64,
}

#[derive(Debug, Clone)]
pub(super) struct CanonicalNoteParts {
    pub node_type: String,
    pub slug: String,
    pub title: String,
    pub status: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
}

impl CanonicalNote {
    pub(super) fn to_parsed_note(&self) -> ParsedNote {
        ParsedNote {
            id: self.frontmatter.id.clone(),
            slug: self.slug.clone(),
            title: self.frontmatter.title.clone(),
            node_type: self.frontmatter.node_type.clone(),
            status: self.frontmatter.status.clone(),
            project: self.frontmatter.project.clone(),
            file_path: self.file_path.clone(),
            summary: self.summary.clone(),
            observations: self.observations.clone(),
            relations: self.relations.clone(),
            references: self.references.clone(),
            aliases: self.frontmatter.aliases.clone(),
            tags: self.frontmatter.tags.clone(),
            body: render_note(self),
            created_at: self.frontmatter.created_at.clone(),
            updated_at: self.frontmatter.updated_at.clone(),
            raw_hash: self.raw_hash.clone(),
            file_mtime_ms: self.file_mtime_ms,
        }
    }
}

pub(super) fn scan_canonical_locations(memory_root: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::with_capacity(CANONICAL_DIRECTORIES.len() + 1);
    roots.push(memory_root.to_path_buf());
    roots.extend(
        CANONICAL_DIRECTORIES
            .iter()
            .map(|(directory, _)| memory_root.join(directory)),
    );
    roots
}

pub(super) fn root_markdown_files(memory_root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !memory_root.exists() {
        return Ok(files);
    }
    for entry in std::fs::read_dir(memory_root)
        .with_context(|| format!("failed to read {}", memory_root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|value| value.eq_ignore_ascii_case("md"))
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

pub(super) fn parse_canonical_note(
    memory_root: &Path,
    file_path: &Path,
    expected_type: Option<&str>,
) -> Result<ParsedNote> {
    Ok(parse_canonical_document(memory_root, file_path, expected_type)?.to_parsed_note())
}

pub(super) fn parse_canonical_document(
    memory_root: &Path,
    file_path: &Path,
    expected_type: Option<&str>,
) -> Result<CanonicalNote> {
    let raw = std::fs::read_to_string(file_path)
        .with_context(|| format!("failed to read {}", file_path.display()))?;
    let metadata = std::fs::metadata(file_path)
        .with_context(|| format!("failed to stat {}", file_path.display()))?;
    let file_mtime_ms = metadata
        .modified()
        .ok()
        .and_then(system_time_to_unix_ms)
        .unwrap_or_default();
    let raw_hash = hash_bytes(raw.as_bytes());
    let (frontmatter, body) = split_frontmatter(&raw)?;
    let expected = expected_type.unwrap_or_else(|| expected_type_for_path(memory_root, file_path));
    let (frontmatter_fields, slug) = parse_frontmatter(
        &frontmatter,
        legacy_slug_from_path(memory_root, file_path).as_deref(),
    )?;
    if !canonical_location_accepts(frontmatter_fields.node_type.as_str(), expected) {
        anyhow::bail!(
            "note type `{}` does not match canonical location for `{}`",
            frontmatter_fields.node_type,
            file_path.display()
        );
    }
    let sections = parse_body_sections(body, &frontmatter_fields.title)?;
    Ok(CanonicalNote {
        frontmatter: frontmatter_fields,
        slug,
        file_path: relative_file_path(memory_root, file_path),
        summary: sections.summary,
        observations: sections.observations,
        relations: sections.relations,
        references: sections.references,
        raw_hash,
        file_mtime_ms,
    })
}

pub(super) fn render_note(note: &CanonicalNote) -> String {
    let frontmatter = &note.frontmatter;
    let mut out = String::new();
    out.push_str("---\n");
    push_yaml_line(&mut out, "id", &frontmatter.id);
    push_yaml_line(&mut out, "slug", &note.slug);
    push_yaml_line(
        &mut out,
        "type",
        canonical_type_title(&frontmatter.node_type),
    );
    push_yaml_line(&mut out, "title", &frontmatter.title);
    push_yaml_line(&mut out, "status", &frontmatter.status);
    push_yaml_line(&mut out, "project", &frontmatter.project);
    push_yaml_line(&mut out, "created_at", &frontmatter.created_at);
    push_yaml_line(&mut out, "updated_at", &frontmatter.updated_at);
    push_yaml_list(&mut out, "tags", &frontmatter.tags);
    push_yaml_list(&mut out, "aliases", &frontmatter.aliases);
    out.push_str("---\n\n");
    out.push_str("# ");
    out.push_str(frontmatter.title.trim());
    out.push_str("\n\n");
    render_section(&mut out, "Summary", &render_summary_lines(&note.summary));
    render_section(
        &mut out,
        "Observations",
        &render_bullets(&note.observations),
    );
    render_section(
        &mut out,
        "Relations",
        &render_relation_lines(&note.relations),
    );
    render_section(&mut out, "References", &render_bullets(&note.references));
    out.trim_end().to_string() + "\n"
}

pub(super) fn canonical_note_path(
    memory_root: &Path,
    node_type: &str,
    title: &str,
    status: &str,
) -> PathBuf {
    let file_name = canonical_file_name(title, status);
    if matches!(node_type, "project" | SECTION_HUB_NODE_TYPE) {
        memory_root.join(file_name)
    } else {
        let directory = canonical_directory(node_type);
        memory_root.join(directory).join(file_name)
    }
}

pub(super) fn canonical_file_name(title: &str, status: &str) -> String {
    format!("{}.md", canonical_file_stem(title, status))
}

pub(super) fn canonical_file_stem(title: &str, status: &str) -> String {
    let mut stem = normalize_title_filename(title).unwrap_or_else(|| "Untitled".to_string());
    if let Some(suffix) = historical_filename_suffix(status) {
        stem.push(' ');
        stem.push('(');
        stem.push_str(suffix);
        stem.push(')');
    }
    stem
}

pub(super) fn canonical_directory(node_type: &str) -> &'static str {
    if let Some((directory, _)) = CANONICAL_DIRECTORIES
        .iter()
        .find(|(_, mapped_type)| mapped_type == &node_type)
    {
        directory
    } else {
        panic!("unsupported canonical node type `{node_type}`");
    }
}

pub(super) fn canonical_type_title(node_type: &str) -> &'static str {
    match node_type {
        "project" => "Project",
        "section_hub" => "SectionHub",
        "module" => "Module",
        "decision" => "Decision",
        "architecture_note" => "ArchitectureNote",
        "task" => "Task",
        "progress_entry" => "ProgressEntry",
        "risk" => "Risk",
        "constraint" => "Constraint",
        "glossary_term" => "GlossaryTerm",
        "artifact" => "Artifact",
        _ => panic!("unsupported node type `{node_type}`"),
    }
}

pub(super) fn normalize_node_type(raw: &str) -> Option<String> {
    let normalized = raw.trim().replace(['-', ' '], "_").to_ascii_lowercase();
    match normalized.as_str() {
        "project" => Some("project".to_string()),
        "sectionhub" | "section_hub" => Some("section_hub".to_string()),
        "module" => Some("module".to_string()),
        "decision" => Some("decision".to_string()),
        "architecturenote" | "architecture_note" => Some("architecture_note".to_string()),
        "task" => Some("task".to_string()),
        "progressentry" | "progress_entry" => Some("progress_entry".to_string()),
        "risk" => Some("risk".to_string()),
        "constraint" => Some("constraint".to_string()),
        "glossaryterm" | "glossary_term" => Some("glossary_term".to_string()),
        "artifact" => Some("artifact".to_string()),
        _ => None,
    }
}

pub(super) fn normalize_relation_kind(raw: &str) -> Option<String> {
    let normalized = raw.trim().replace(['-', ' '], "_").to_ascii_lowercase();
    if RELATION_KINDS.contains(&normalized.as_str()) {
        Some(normalized)
    } else {
        None
    }
}

pub(super) fn canonical_note_from_parts(
    memory_root: &Path,
    project_name: &str,
    parts: CanonicalNoteParts,
) -> CanonicalNote {
    let path = canonical_note_path(memory_root, &parts.node_type, &parts.title, &parts.status);
    let note = CanonicalNote {
        frontmatter: FrontmatterFields {
            id: parts.id,
            node_type: parts.node_type,
            title: parts.title.trim().to_string(),
            status: parts.status.trim().to_string(),
            project: project_name.to_string(),
            created_at: parts.created_at,
            updated_at: parts.updated_at,
            aliases: parts.aliases,
            tags: parts.tags,
        },
        slug: parts.slug,
        file_path: relative_file_path(memory_root, &path),
        summary: parts.summary.trim().to_string(),
        observations: Vec::new(),
        relations: Vec::new(),
        references: Vec::new(),
        raw_hash: String::new(),
        file_mtime_ms: 0,
    };
    let rendered = render_note(&note);
    CanonicalNote {
        raw_hash: hash_bytes(rendered.as_bytes()),
        ..note
    }
}

pub(super) fn normalize_slug(raw: &str) -> Option<String> {
    if raw.trim() == PROJECT_NODE_SLUG {
        return Some(PROJECT_NODE_SLUG.to_string());
    }
    let mut slug = String::with_capacity(raw.len());
    let mut last_was_dash = false;
    for character in raw.chars() {
        let next = if character.is_ascii_alphanumeric() {
            character.to_ascii_lowercase()
        } else if matches!(character, '-' | '_' | ' ') {
            '-'
        } else {
            continue;
        };
        if next == '-' {
            if slug.is_empty() || last_was_dash {
                continue;
            }
            last_was_dash = true;
        } else {
            last_was_dash = false;
        }
        slug.push(next);
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() { None } else { Some(slug) }
}

pub(super) fn normalize_title_filename(raw: &str) -> Option<String> {
    let mut stem = String::new();
    let mut last_was_space = false;
    for character in raw.trim().chars() {
        if character.is_control() {
            continue;
        }
        let next = match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => ' ',
            value if value.is_whitespace() => ' ',
            value => value,
        };
        if next == ' ' {
            if stem.is_empty() || last_was_space {
                continue;
            }
            last_was_space = true;
        } else {
            last_was_space = false;
        }
        stem.push(next);
    }
    let trimmed = stem.trim_matches([' ', '.']).to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

pub(super) fn now_timestamp_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after unix epoch")
        .as_millis()
        .to_string()
}

fn relative_file_path(memory_root: &Path, file_path: &Path) -> String {
    file_path
        .strip_prefix(memory_root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn expected_type_for_path(memory_root: &Path, file_path: &Path) -> &'static str {
    let relative = relative_file_path(memory_root, file_path);
    let relative_path = Path::new(&relative);
    if relative_path.parent().is_none()
        || relative_path
            .parent()
            .is_some_and(|parent| parent.as_os_str().is_empty())
    {
        return "root";
    }
    let Some(parent) = relative_path.parent() else {
        panic!("canonical note missing parent directory");
    };
    let Some(directory) = parent.file_name().and_then(|value| value.to_str()) else {
        panic!("canonical note directory should be valid utf-8");
    };
    canonical_directory_type(directory)
}

fn canonical_directory_type(directory: &str) -> &'static str {
    CANONICAL_DIRECTORIES
        .iter()
        .find(|(candidate, _)| candidate == &directory)
        .map(|(_, node_type)| *node_type)
        .unwrap_or_else(|| panic!("unsupported canonical directory `{directory}`"))
}

fn canonical_location_accepts(node_type: &str, expected: &str) -> bool {
    if expected == "root" {
        return matches!(node_type, "project" | SECTION_HUB_NODE_TYPE);
    }
    node_type == expected
}

fn split_frontmatter(raw: &str) -> Result<(Mapping, &str)> {
    if !raw.starts_with("---\n") && !raw.starts_with("---\r\n") {
        anyhow::bail!("canonical note must start with frontmatter");
    }
    let rest = &raw[4..];
    let marker = "\n---";
    let Some(end) = rest.find(marker) else {
        anyhow::bail!("unterminated frontmatter block");
    };
    let yaml_block = &rest[..end];
    let body = &rest[end + marker.len()..];
    let value = serde_yaml::from_str::<Value>(yaml_block)?;
    let mapping = value
        .as_mapping()
        .cloned()
        .context("frontmatter must be a YAML object")?;
    Ok((mapping, body.trim_start_matches(['\r', '\n'])))
}

fn parse_frontmatter(
    frontmatter: &Mapping,
    legacy_slug_hint: Option<&str>,
) -> Result<(FrontmatterFields, String)> {
    ensure_flat_frontmatter(frontmatter)?;
    let id = required_yaml_string(frontmatter, "id")?;
    let slug = match frontmatter.get(Value::String("slug".to_string())) {
        Some(_) => normalize_slug(&required_yaml_string(frontmatter, "slug")?)
            .context("frontmatter `slug` must normalize to non-empty canonical form")?,
        None => legacy_slug_hint
            .and_then(normalize_slug)
            .context("frontmatter missing required `slug`")?,
    };
    let node_type_raw = required_yaml_string(frontmatter, "type")?;
    let node_type = normalize_node_type(&node_type_raw)
        .ok_or_else(|| anyhow::anyhow!("unsupported canonical type `{node_type_raw}`"))?;
    let title = required_yaml_string(frontmatter, "title")?;
    let status = required_yaml_string(frontmatter, "status")?;
    let project = required_yaml_string(frontmatter, "project")?;
    let created_at = required_yaml_string(frontmatter, "created_at")?;
    let updated_at = required_yaml_string(frontmatter, "updated_at")?;
    let aliases = yaml_string_list(frontmatter, "aliases")?;
    let tags = yaml_string_list(frontmatter, "tags")?;
    let expected_prefix = format!("{node_type}-");
    if node_type != "project" && !id.starts_with(&expected_prefix) {
        anyhow::bail!(
            "canonical note id `{id}` must start with `{expected_prefix}` for slug `{slug}`"
        );
    }
    Ok((
        FrontmatterFields {
            id,
            node_type,
            title,
            status,
            project,
            created_at,
            updated_at,
            aliases,
            tags,
        },
        slug,
    ))
}

fn ensure_flat_frontmatter(frontmatter: &Mapping) -> Result<()> {
    for (key, value) in frontmatter {
        let key = key
            .as_str()
            .context("frontmatter keys must be strings")?
            .to_string();
        match value {
            Value::String(_) | Value::Bool(_) | Value::Number(_) | Value::Null => {}
            Value::Sequence(items) => {
                for item in items {
                    if !matches!(item, Value::String(_)) {
                        anyhow::bail!(
                            "frontmatter `{key}` must be a flat list of strings, not nested values"
                        );
                    }
                }
            }
            _ => anyhow::bail!("frontmatter `{key}` must stay flat in MVP"),
        }
    }
    Ok(())
}

fn required_yaml_string(frontmatter: &Mapping, key: &str) -> Result<String> {
    let value = frontmatter
        .get(Value::String(key.to_string()))
        .context(format!("frontmatter missing required `{key}`"))?;
    let raw = match value {
        Value::String(raw) => raw.trim().to_string(),
        Value::Number(raw) => raw.to_string(),
        Value::Bool(raw) => raw.to_string(),
        _ => String::new(),
    };
    if raw.is_empty() {
        anyhow::bail!("frontmatter `{key}` must be non-empty string");
    }
    Ok(raw)
}

fn yaml_string_list(frontmatter: &Mapping, key: &str) -> Result<Vec<String>> {
    let Some(value) = frontmatter.get(Value::String(key.to_string())) else {
        return Ok(Vec::new());
    };
    match value {
        Value::Sequence(items) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
                    .context(format!(
                        "frontmatter `{key}` must contain only non-empty strings"
                    ))
            })
            .collect(),
        _ => anyhow::bail!("frontmatter `{key}` must be a flat list of strings"),
    }
}

struct ParsedSections {
    summary: String,
    observations: Vec<String>,
    relations: Vec<ParsedRelation>,
    references: Vec<String>,
}

fn parse_body_sections(body: &str, title: &str) -> Result<ParsedSections> {
    let lines = body.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        anyhow::bail!("canonical note body must contain H1 and sections");
    }
    let h1 = lines[0].trim();
    if h1 != format!("# {title}") {
        anyhow::bail!("canonical note H1 must match frontmatter title");
    }

    let mut sections = BTreeSet::new();
    let mut current_section = None::<&str>;
    let mut summary_lines = Vec::new();
    let mut observations = Vec::new();
    let mut relations = Vec::new();
    let mut references = Vec::new();

    for line in lines.iter().skip(1) {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            if current_section == Some("Summary") {
                summary_lines.push(String::new());
            }
            continue;
        }

        if let Some(section_name) = trimmed.strip_prefix("## ") {
            if !BODY_SECTION_ORDER.contains(&section_name) {
                anyhow::bail!("unsupported section heading `## {section_name}`");
            }
            sections.insert(section_name.to_string());
            current_section = Some(section_name);
            continue;
        }

        match current_section {
            Some("Summary") => summary_lines.push(trimmed.to_string()),
            Some("Observations") => observations.push(parse_bullet_line(trimmed, "Observations")?),
            Some("Relations") => relations.push(parse_relation_line(trimmed)?),
            Some("References") => references.push(parse_bullet_line(trimmed, "References")?),
            None => anyhow::bail!("canonical note content before first section is unsupported"),
            Some(other) => anyhow::bail!("unsupported section content in `{other}`"),
        }
    }

    for required in BODY_SECTION_ORDER {
        if !sections.contains(*required) {
            anyhow::bail!("canonical note missing required `## {required}` section");
        }
    }

    Ok(ParsedSections {
        summary: trim_blank_lines(&summary_lines).join("\n"),
        observations,
        relations,
        references,
    })
}

fn parse_bullet_line(line: &str, section: &str) -> Result<String> {
    let Some(content) = line.trim().strip_prefix("- ") else {
        anyhow::bail!("`{section}` entries must use `- ` bullets");
    };
    let trimmed = content.trim();
    if trimmed.is_empty() {
        anyhow::bail!("`{section}` bullets must be non-empty");
    }
    Ok(trimmed.to_string())
}

fn parse_relation_line(line: &str) -> Result<ParsedRelation> {
    let content = parse_bullet_line(line, "Relations")?;
    let Some((kind_raw, target_raw)) = content.split_once(' ') else {
        anyhow::bail!("relation entry must include relation kind and wikilink target");
    };
    let relation_kind = normalize_relation_kind(kind_raw)
        .ok_or_else(|| anyhow::anyhow!("unsupported relation kind `{kind_raw}`"))?;
    let (target_slug, target_title) = parse_wikilink(target_raw)?;
    Ok(ParsedRelation {
        relation_kind,
        target_slug,
        target_title,
    })
}

fn parse_wikilink(raw: &str) -> Result<(String, String)> {
    let trimmed = raw.trim();
    if !trimmed.starts_with("[[") || !trimmed.ends_with("]]") {
        anyhow::bail!("relation target must be a wikilink");
    }
    let inner = &trimmed[2..trimmed.len() - 2];
    let (target, display) = inner
        .split_once('|')
        .map_or((inner, inner), |(target, display)| (target, display));
    let slug = target
        .split('#')
        .next()
        .unwrap_or(target)
        .trim()
        .to_string();
    let title = display.trim().to_string();
    if slug.is_empty() || title.is_empty() {
        anyhow::bail!("relation wikilink target and title must be non-empty");
    }
    Ok((slug, title))
}

fn render_summary_lines(summary: &str) -> Vec<String> {
    if summary.trim().is_empty() {
        Vec::new()
    } else {
        summary
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect()
    }
}

fn render_bullets(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|item| format!("- {}", item.trim()))
        .collect::<Vec<_>>()
}

fn render_relation_lines(relations: &[ParsedRelation]) -> Vec<String> {
    relations
        .iter()
        .map(|relation| {
            format!(
                "- {} [[{}|{}]]",
                relation.relation_kind, relation.target_slug, relation.target_title
            )
        })
        .collect()
}

fn render_section(out: &mut String, name: &str, lines: &[String]) {
    out.push_str("## ");
    out.push_str(name);
    out.push('\n');
    if !lines.is_empty() {
        for line in lines {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push('\n');
}

fn push_yaml_line(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push_str(": ");
    out.push_str(value.trim());
    out.push('\n');
}

fn push_yaml_list(out: &mut String, key: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    out.push_str(key);
    out.push_str(":\n");
    for item in items {
        out.push_str("  - ");
        out.push_str(item.trim());
        out.push('\n');
    }
}

fn trim_blank_lines(lines: &[String]) -> Vec<String> {
    let mut start = 0;
    let mut end = lines.len();
    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    lines[start..end].to_vec()
}

fn system_time_to_unix_ms(value: SystemTime) -> Option<i64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
}

fn historical_filename_suffix(status: &str) -> Option<&'static str> {
    let normalized = status.trim().replace(['-', ' '], "_").to_ascii_lowercase();
    match normalized.as_str() {
        "resolved" => Some("Resolved"),
        "closed" => Some("Closed"),
        "done" => Some("Done"),
        "accepted" => Some("Accepted"),
        "superseded" => Some("Superseded"),
        "obsolete" => Some("Obsolete"),
        _ => None,
    }
}

fn legacy_slug_from_path(memory_root: &Path, file_path: &Path) -> Option<String> {
    let relative = file_path
        .strip_prefix(memory_root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .replace('\\', "/");
    if relative == "_index.md" {
        Some(PROJECT_NODE_SLUG.to_string())
    } else {
        file_path
            .file_stem()
            .and_then(|value| value.to_str())
            .map(ToOwned::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        CanonicalNoteParts, PROJECT_NODE_SLUG, canonical_note_from_parts, parse_canonical_note,
        render_note,
    };

    #[test]
    fn canonical_note_roundtrip_preserves_sections() {
        let root = tempfile_dir("canonical-roundtrip");
        let note = canonical_note_from_parts(
            Path::new(&root),
            "canonical-roundtrip",
            CanonicalNoteParts {
                node_type: "decision".to_string(),
                slug: "auth-token".to_string(),
                title: "Auth Token".to_string(),
                status: "active".to_string(),
                summary: "Short summary.".to_string(),
                tags: vec!["auth".to_string()],
                aliases: vec!["Token Strategy".to_string(), "auth-token".to_string()],
                id: "decision-auth-token".to_string(),
                created_at: "1".to_string(),
                updated_at: "1".to_string(),
            },
        );
        let rendered = render_note(&note);
        let note_dir = Path::new(&root).join("decisions");
        std::fs::create_dir_all(&note_dir).expect("create dir");
        let note_path = note_dir.join("Auth Token.md");
        std::fs::write(&note_path, rendered).expect("write note");

        let parsed = parse_canonical_note(Path::new(&root), &note_path, Some("decision"))
            .expect("parse canonical note");
        assert_eq!(parsed.id, "decision-auth-token");
        assert_eq!(parsed.slug, "auth-token");
        assert_eq!(parsed.summary, "Short summary.");
        assert!(parsed.observations.is_empty());
        assert!(parsed.references.is_empty());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn canonical_project_roundtrip_supports_human_hub_filename() {
        let root = tempfile_dir("canonical-project-roundtrip");
        std::fs::create_dir_all(&root).expect("create root");
        let note = canonical_note_from_parts(
            Path::new(&root),
            "canonical-project-roundtrip",
            CanonicalNoteParts {
                node_type: "project".to_string(),
                slug: PROJECT_NODE_SLUG.to_string(),
                title: "Workspace Control Plane".to_string(),
                status: "active".to_string(),
                summary: "Hub summary.".to_string(),
                tags: vec!["memory".to_string()],
                aliases: vec!["Workspace".to_string(), PROJECT_NODE_SLUG.to_string()],
                id: "project-workspace".to_string(),
                created_at: "1".to_string(),
                updated_at: "1".to_string(),
            },
        );
        let rendered = render_note(&note);
        let note_path = Path::new(&root).join("Workspace Control Plane.md");
        std::fs::write(&note_path, rendered).expect("write project note");

        let parsed = parse_canonical_note(Path::new(&root), &note_path, Some("project"))
            .expect("parse project note");
        assert_eq!(parsed.slug, PROJECT_NODE_SLUG);
        assert_eq!(parsed.title, "Workspace Control Plane");

        let _ = std::fs::remove_dir_all(root);
    }

    fn tempfile_dir(prefix: &str) -> std::path::PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{suffix}"))
    }
}
