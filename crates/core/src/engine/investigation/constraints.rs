use std::collections::HashSet;

use anyhow::Result;

use crate::engine::Engine;
use crate::model::{ConstraintEvidence, InvestigationAnchor};

use super::common::{detect_language, read_source};

const EXCERPT_MAX_CHARS: usize = 160;

pub(super) fn collect_constraint_evidence(
    engine: &Engine,
    anchor: &InvestigationAnchor,
    paths: &[String],
) -> Result<Vec<ConstraintEvidence>> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for path in paths {
        let source = match read_source(&engine.project_root, path) {
            Ok(source) => source,
            Err(_) => continue,
        };
        let language = detect_language(path, "");
        for (index, line) in source.lines().enumerate() {
            let normalized_text = normalize_line(line);
            if normalized_text.is_empty() {
                continue;
            }
            let Some(adapter_match) = extract_with_adapter(path, &language, &normalized_text)
            else {
                continue;
            };
            let evidence = ConstraintEvidence::new(
                adapter_match.constraint_kind,
                adapter_match.source_kind,
                path.clone(),
                index + 1,
                index + 1,
                capped_excerpt(&normalized_text),
                adapter_match.strength,
                anchor.symbol.clone().unwrap_or_else(|| anchor.path.clone()),
                is_migration_path(path).then(|| path.clone()),
                adapter_match.confidence,
                normalized_text,
            );
            if seen.insert((
                evidence.path.clone(),
                evidence.line_start,
                evidence.constraint_kind.clone(),
                evidence.normalized_key.clone(),
            )) {
                items.push(evidence);
            }
        }
    }
    Ok(items)
}

#[derive(Debug, Clone, Copy)]
struct AdapterMatch {
    constraint_kind: &'static str,
    source_kind: &'static str,
    strength: &'static str,
    confidence: f32,
}

#[derive(Debug)]
struct ConstraintAdapterInput<'a> {
    path: &'a str,
    language: &'a str,
    lowered_path: String,
    lowered_line: String,
}

type ConstraintAdapter = fn(&ConstraintAdapterInput<'_>) -> Option<AdapterMatch>;

const ADAPTERS: [ConstraintAdapter; 5] = [
    python_adapter,
    typescript_adapter,
    rust_adapter,
    sql_prisma_adapter,
    generic_weak_fallback_adapter,
];

fn extract_with_adapter(path: &str, language: &str, normalized_line: &str) -> Option<AdapterMatch> {
    let input = ConstraintAdapterInput {
        path,
        language,
        lowered_path: path.replace('\\', "/").to_ascii_lowercase(),
        lowered_line: normalized_line.to_ascii_lowercase(),
    };
    ADAPTERS.iter().find_map(|adapter| adapter(&input))
}

fn python_adapter(input: &ConstraintAdapterInput<'_>) -> Option<AdapterMatch> {
    if input.language != "python" {
        return None;
    }
    if input.lowered_line.contains("op.create_index(") && input.lowered_line.contains("unique") {
        return Some(strong_match("index_constraint", "index_declaration", 0.95));
    }
    if input.lowered_line.contains("op.create_unique_constraint")
        || input.lowered_line.contains("op.create_foreign_key")
        || input.lowered_line.contains("foreignkeyconstraint")
        || input.lowered_line.contains("checkconstraint")
    {
        return Some(strong_match(
            if is_migration_path(input.path) {
                "migration_constraint"
            } else {
                "model_constraint"
            },
            if is_migration_path(input.path) {
                "migration_declaration"
            } else {
                "model_declaration"
            },
            0.92,
        ));
    }
    if input.lowered_line.contains("uniqueconstraint")
        || input.lowered_line.contains("foreignkey(")
        || input.lowered_line.contains(" references ")
    {
        return Some(strong_match("model_constraint", "model_declaration", 0.9));
    }
    None
}

fn typescript_adapter(input: &ConstraintAdapterInput<'_>) -> Option<AdapterMatch> {
    if !matches!(input.language, "typescript" | "javascript") {
        return None;
    }
    if input.lowered_line.contains("createindex(") {
        return Some(strong_match("index_constraint", "index_declaration", 0.9));
    }
    if input.lowered_line.contains("unique: true") || input.lowered_line.contains("@unique") {
        return Some(strong_match("model_constraint", "model_declaration", 0.86));
    }
    if input.lowered_line.contains("references:") || input.lowered_line.contains("foreignkey") {
        return Some(strong_match("model_constraint", "model_declaration", 0.84));
    }
    None
}

fn rust_adapter(input: &ConstraintAdapterInput<'_>) -> Option<AdapterMatch> {
    if input.language != "rust" {
        return None;
    }
    if input.lowered_line.contains("table!")
        || input.lowered_line.contains("joinable!")
        || input
            .lowered_line
            .contains("allow_tables_to_appear_in_same_query!")
    {
        return Some(weak_match("ddl_like_hint", "schema_hint", 0.6));
    }
    if input.lowered_line.contains("sqlx::query!(")
        || input.lowered_line.contains("sqlx::query_as!(")
        || input.lowered_line.contains("query!(")
        || input.lowered_line.contains("query_as!(")
    {
        return Some(weak_match("runtime_guard", "runtime_guard_code", 0.55));
    }
    None
}

fn sql_prisma_adapter(input: &ConstraintAdapterInput<'_>) -> Option<AdapterMatch> {
    if input.language == "sql" {
        if input.lowered_line.contains("create unique index")
            || input.lowered_line.contains("unique index")
        {
            return Some(strong_match("index_constraint", "index_declaration", 0.95));
        }
        if input.lowered_line.contains("create index") {
            return Some(strong_match("index_constraint", "index_declaration", 0.88));
        }
        if input.lowered_line.contains("add constraint")
            || input.lowered_line.contains("foreign key")
            || input.lowered_line.contains(" references ")
            || input.lowered_line.contains(" check ")
        {
            return Some(strong_match(
                if is_migration_path(input.path) {
                    "migration_constraint"
                } else {
                    "model_constraint"
                },
                if is_migration_path(input.path) {
                    "migration_declaration"
                } else {
                    "schema_hint"
                },
                0.9,
            ));
        }
    }
    if input.language == "prisma" {
        if input.lowered_line.contains("@@index") {
            return Some(strong_match("index_constraint", "index_declaration", 0.9));
        }
        if input.lowered_line.contains("@@unique")
            || input.lowered_line.contains("@unique")
            || input.lowered_line.contains("@id")
            || input.lowered_line.contains("@relation")
        {
            return Some(strong_match("model_constraint", "model_declaration", 0.9));
        }
    }
    None
}

fn generic_weak_fallback_adapter(input: &ConstraintAdapterInput<'_>) -> Option<AdapterMatch> {
    if input.lowered_line.contains("validate")
        || input.lowered_line.contains("assert")
        || input.lowered_line.contains("ensure")
        || input.lowered_line.contains("guard")
    {
        return Some(weak_match("runtime_guard", "runtime_guard_code", 0.5));
    }
    if input.lowered_line.contains("index") || input.lowered_line.contains("constraint") {
        return Some(if is_schema_like_path(&input.lowered_path) {
            weak_match("ddl_like_hint", "schema_hint", 0.45)
        } else {
            weak_match("runtime_guard", "runtime_guard_code", 0.45)
        });
    }
    None
}

fn strong_match(
    constraint_kind: &'static str,
    source_kind: &'static str,
    confidence: f32,
) -> AdapterMatch {
    AdapterMatch {
        constraint_kind,
        source_kind,
        strength: "strong",
        confidence,
    }
}

fn weak_match(
    constraint_kind: &'static str,
    source_kind: &'static str,
    confidence: f32,
) -> AdapterMatch {
    AdapterMatch {
        constraint_kind,
        source_kind,
        strength: "weak",
        confidence,
    }
}

fn normalize_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn capped_excerpt(normalized_line: &str) -> String {
    let excerpt = normalized_line.trim();
    if excerpt.chars().count() <= EXCERPT_MAX_CHARS {
        excerpt.to_string()
    } else {
        let shortened = excerpt.chars().take(EXCERPT_MAX_CHARS).collect::<String>();
        format!("{shortened}...")
    }
}

fn is_migration_path(path: &str) -> bool {
    let lowered = path.replace('\\', "/").to_ascii_lowercase();
    lowered.starts_with("migrations/")
        || lowered.starts_with("alembic/")
        || lowered.starts_with("versions/")
        || lowered.contains("/migrations/")
        || lowered.contains("/alembic/")
        || lowered.contains("/versions/")
}

fn is_schema_like_path(lowered_path: &str) -> bool {
    lowered_path.contains("schema")
        || lowered_path.contains("model")
        || lowered_path.contains("entity")
        || lowered_path.ends_with(".sql")
        || lowered_path.ends_with(".prisma")
}

#[cfg(test)]
#[path = "constraints_tests.rs"]
mod tests;
