use std::collections::HashSet;

use anyhow::Result;

use crate::engine::Engine;
use crate::model::{ConstraintEvidence, InvestigationAnchor};

use super::common::{detect_language, read_source};

#[path = "constraints_noise.rs"]
mod noise;
#[path = "constraints_python.rs"]
mod python;
#[path = "constraints_rust.rs"]
mod rust;
#[path = "constraints_sql.rs"]
mod sql;
#[path = "constraints_typescript.rs"]
mod typescript;

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
            if noise::should_ignore_constraint_line(&language, &normalized_text) {
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
    python::python_adapter,
    typescript::typescript_adapter,
    rust::rust_adapter,
    sql::sql_prisma_adapter,
    noise::generic_weak_fallback_adapter,
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
