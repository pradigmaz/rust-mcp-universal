use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::Result;
use rusqlite::{Connection, params};
use walkdir::WalkDir;

use crate::model::{ContextMode, SearchHit};
use crate::search_db::extract_tokens;

use super::super::intent::SearchIntent;

const TEST_QUERY_NEEDLES: &[&str] = &["test", "tests"];
const SERVICE_QUERY_NEEDLES: &[&str] = &["domain", "service", "orchestration", "module"];

pub(super) fn load_supplemental_hits(
    project_root: &Path,
    conn: &Connection,
    intent: &SearchIntent,
    query: &str,
    requested_limit: usize,
    context_mode: Option<ContextMode>,
    existing_hits: &[SearchHit],
) -> Result<Vec<SearchHit>> {
    let window = requested_limit.min(8);
    let mut needles = Vec::new();

    if intent.expects_test_surface()
        && !existing_hits
            .iter()
            .take(window)
            .any(|hit| is_test_path(&hit.path))
    {
        needles.extend(TEST_QUERY_NEEDLES.iter().copied());
    }
    if intent.expects_service_surface() && !existing_hits.iter().take(window).any(is_service_hit) {
        needles.extend(SERVICE_QUERY_NEEDLES.iter().copied());
    }
    if needles.is_empty() {
        return Ok(Vec::new());
    }

    let query_tokens = extract_tokens(query);
    let mut seen_paths = HashSet::new();
    let mut supplemental = Vec::new();
    for needle in needles {
        for mut hit in load_hits_for_query_needle(conn, needle, requested_limit)? {
            if !seen_paths.insert(hit.path.clone()) {
                continue;
            }
            if needle.contains("test") && !is_test_path(&hit.path) {
                continue;
            }
            if !needle.contains("test") && !is_service_hit(&hit) {
                continue;
            }
            hit.score += 0.30;
            hit.score += local_path_match_boost(&hit.path, &query_tokens);
            hit.score += intent.score_hit(&hit.path, &hit.preview, &hit.language, context_mode);
            supplemental.push(hit);
        }
    }

    if intent.expects_test_surface() && !supplemental.iter().any(|hit| is_test_path(&hit.path)) {
        supplemental.extend(load_filesystem_hits(
            project_root,
            requested_limit,
            query_tokens.as_slice(),
            intent,
            context_mode,
            HitKind::Tests,
        )?);
    }
    if intent.expects_service_surface() && !supplemental.iter().any(is_service_hit) {
        supplemental.extend(load_filesystem_hits(
            project_root,
            requested_limit,
            query_tokens.as_slice(),
            intent,
            context_mode,
            HitKind::Service,
        )?);
    }

    supplemental.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.path.cmp(&b.path))
    });
    supplemental.truncate(requested_limit.min(8));
    Ok(supplemental)
}

fn load_hits_for_query_needle(
    conn: &Connection,
    needle: &str,
    requested_limit: usize,
) -> Result<Vec<SearchHit>> {
    let limit = i64::try_from(requested_limit.saturating_mul(8)).unwrap_or(64);
    let mut stmt = conn.prepare(
        r#"
        SELECT path, sample, size_bytes, language
        FROM files
        WHERE LOWER(REPLACE(path, CHAR(92), '/')) LIKE '%' || LOWER(?1) || '%'
        ORDER BY path
        LIMIT ?2
        "#,
    )?;
    let rows = stmt
        .query_map(params![needle, limit], |row| {
            Ok(SearchHit {
                path: row.get(0)?,
                preview: row.get(1)?,
                score: 0.0,
                size_bytes: row.get(2)?,
                language: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn is_service_hit(hit: &SearchHit) -> bool {
    let path = hit.path.replace('\\', "/").to_ascii_lowercase();
    path.contains("/domain/")
        || path.contains("/services/")
        || path.contains("/orchestration/")
        || path.contains("/modules/")
        || path.ends_with("_service.py")
        || path.ends_with("_service.rs")
}

fn is_test_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    if normalized.contains(".pytest_cache/")
        || normalized.starts_with(".pytest_cache/")
        || normalized.contains("/__pycache__/")
        || normalized.ends_with(".pyc")
    {
        return false;
    }
    normalized.starts_with("tests/")
        || normalized.contains("/tests/")
        || normalized.contains("/test/")
        || normalized.contains("_tests/")
        || normalized.ends_with(".test.ts")
        || normalized.ends_with(".test.tsx")
        || normalized.ends_with(".test.js")
        || normalized.ends_with(".test.jsx")
        || normalized.ends_with(".test.py")
        || normalized.ends_with(".test.rs")
        || normalized.ends_with("_test.rs")
        || normalized.ends_with("_tests.rs")
        || normalized.ends_with("_spec.ts")
        || normalized.ends_with("_spec.tsx")
        || normalized.contains("test_")
}

fn local_path_match_boost(path: &str, tokens: &[String]) -> f32 {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let mut score = 0.0_f32;
    for token in tokens {
        if token.is_empty() {
            continue;
        }
        if normalized.contains(token) {
            score += 0.05;
        }
    }
    score.min(0.20)
}

#[derive(Clone, Copy)]
enum HitKind {
    Tests,
    Service,
}

fn load_filesystem_hits(
    project_root: &Path,
    requested_limit: usize,
    query_tokens: &[String],
    intent: &SearchIntent,
    context_mode: Option<ContextMode>,
    kind: HitKind,
) -> Result<Vec<SearchHit>> {
    let mut hits = Vec::new();
    let max_hits = requested_limit.min(4);
    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel_path = relative_path(project_root, path);
        let matches_kind = match kind {
            HitKind::Tests => is_test_path(&rel_path),
            HitKind::Service => is_service_rel_path(&rel_path),
        };
        if !matches_kind {
            continue;
        }
        let preview = fs::read_to_string(path)
            .ok()
            .map(|text| text.chars().take(260).collect::<String>())
            .unwrap_or_default();
        let language = infer_language_local(path);
        let mut hit = SearchHit {
            path: rel_path,
            preview,
            score: 0.34,
            size_bytes: entry
                .metadata()
                .map(|value| value.len() as i64)
                .unwrap_or(0),
            language,
        };
        hit.score += local_path_match_boost(&hit.path, query_tokens);
        hit.score += intent.score_hit(&hit.path, &hit.preview, &hit.language, context_mode);
        hits.push(hit);
        if hits.len() >= max_hits {
            break;
        }
    }
    Ok(hits)
}

fn relative_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn is_service_rel_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    normalized.contains("/domain/")
        || normalized.starts_with("domain/")
        || normalized.contains("/services/")
        || normalized.starts_with("services/")
        || normalized.contains("/orchestration/")
        || normalized.starts_with("orchestration/")
        || normalized.contains("/modules/")
        || normalized.starts_with("modules/")
        || normalized.ends_with("_service.py")
        || normalized.ends_with("_service.rs")
}

fn infer_language_local(path: &Path) -> String {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "py" => "python",
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "java" => "java",
        "json" => "json",
        "toml" => "toml",
        "yml" | "yaml" => "text",
        "md" => "markdown",
        _ => "text",
    }
    .to_string()
}
