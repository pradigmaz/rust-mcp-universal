use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::engine::Engine;
use crate::model::{ConceptSeedKind, SymbolBodyAmbiguityStatus, SymbolBodyResolutionKind};

fn temp_project_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

#[test]
fn symbol_body_extracts_rust_function_body() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-rust-body");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn interesting_symbol() {\n    println!(\"ok\");\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("interesting_symbol", ConceptSeedKind::Symbol, 3)?;
    assert_eq!(result.capability_status, "supported");
    assert_eq!(result.ambiguity_status, SymbolBodyAmbiguityStatus::None);
    assert!(
        result
            .items
            .iter()
            .any(|item| item.signature.contains("interesting_symbol"))
    );
    assert!(
        result
            .items
            .iter()
            .all(|item| item.resolution_kind == SymbolBodyResolutionKind::ExactSymbolSpan)
    );

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_path_seed_uses_chunk_excerpt_anchor_for_typescript_files() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-typescript-chunk");
    fs::create_dir_all(project_dir.join("web"))?;
    fs::write(
        project_dir.join("web/origin_client.ts"),
        "export function resolveOriginClient(key: string) {\n  return `/api/origin/${key}`;\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("web/origin_client.ts", ConceptSeedKind::Path, 3)?;

    assert_eq!(result.capability_status, "supported");
    assert_eq!(result.ambiguity_status, SymbolBodyAmbiguityStatus::None);
    assert!(result.items.iter().any(|item| {
        item.anchor.path == "web/origin_client.ts"
            && item.resolution_kind == SymbolBodyResolutionKind::ChunkExcerptAnchor
    }));

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_path_seed_handles_bracket_paths() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-bracket-path");
    fs::create_dir_all(project_dir.join("web/[code]/components"))?;
    fs::write(
        project_dir.join("web/[code]/components/page.tsx"),
        "export function ReportStudentTable() {\n  return <div>ok</div>;\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("web/[code]/components/page.tsx", ConceptSeedKind::Path, 3)?;

    assert_eq!(result.capability_status, "supported");
    assert!(result.items.iter().any(|item| {
        item.anchor.path == "web/[code]/components/page.tsx"
            && item.body.contains("ReportStudentTable")
    }));

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_path_line_seed_uses_nearest_indexed_lines_for_sql() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-sql-nearest");
    fs::create_dir_all(project_dir.join("queries"))?;
    fs::write(
        project_dir.join("queries/origin_query.sql"),
        "-- resolve origin\nSELECT id\nFROM origins\nWHERE origin_key = $1;\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("queries/origin_query.sql:3", ConceptSeedKind::PathLine, 3)?;

    assert_eq!(result.capability_status, "supported");
    assert!(result.items.iter().any(|item| {
        item.anchor.path == "queries/origin_query.sql"
            && item.resolution_kind == SymbolBodyResolutionKind::NearestIndexedLines
    }));

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_marks_multiple_exact_matches() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-multiple-exact");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/a.rs"),
        "pub fn duplicate_symbol() {\n    println!(\"a\");\n}\n",
    )?;
    fs::write(
        project_dir.join("src/b.rs"),
        "pub fn duplicate_symbol() {\n    println!(\"b\");\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("duplicate_symbol", ConceptSeedKind::Symbol, 5)?;

    assert_eq!(
        result.ambiguity_status,
        SymbolBodyAmbiguityStatus::MultipleExact
    );
    assert_eq!(result.items.len(), 2);
    assert!(
        result
            .items
            .iter()
            .all(|item| item.resolution_kind == SymbolBodyResolutionKind::ExactSymbolSpan)
    );

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_marks_partial_only_when_exact_match_is_absent() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-partial-only");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn resolve_origin() {\n    println!(\"ok\");\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("resolve", ConceptSeedKind::Symbol, 5)?;

    assert_eq!(result.capability_status, "supported");
    assert_eq!(result.ambiguity_status, SymbolBodyAmbiguityStatus::PartialOnly);
    assert!(result.items.iter().any(|item| {
        item.anchor.symbol.as_deref() == Some("resolve_origin")
            && item.resolution_kind == SymbolBodyResolutionKind::ExactSymbolSpan
    }));

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_path_line_seed_handles_multiline_python_signature() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-python-multiline");
    fs::create_dir_all(project_dir.join("app/services"))?;
    fs::write(
        project_dir.join("app/services/deadline_validator.py"),
        "async def get_max_allowed_grade_for_lab(\n    db,\n    lab,\n    current_lesson,\n    student_id=None,\n) -> int:\n    if student_id:\n        return 5\n    origin_lesson = await db.get('lesson', lab.lesson_id)\n    return 4\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body(
        "app/services/deadline_validator.py:8",
        ConceptSeedKind::PathLine,
        3,
    )?;

    let item = result.items.first().expect("python body item");
    assert_eq!(
        item.resolution_kind,
        SymbolBodyResolutionKind::ExactSymbolSpan
    );
    assert_eq!(item.span.start_line, 1);
    assert!(item.span.end_line >= 9);
    assert!(item.body.contains("origin_lesson"));

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_reports_unsupported_sources_for_unsupported_languages() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-unsupported");
    fs::create_dir_all(project_dir.join("docs"))?;
    fs::write(
        project_dir.join("docs/guide.md"),
        "# Investigation\n\nThis file is not a supported source.\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("docs/guide.md", ConceptSeedKind::Path, 3)?;

    assert_eq!(result.capability_status, "unsupported");
    assert!(result.items.is_empty());
    assert_eq!(result.ambiguity_status, SymbolBodyAmbiguityStatus::None);
    assert_eq!(result.unsupported_sources, vec!["markdown:docs/guide.md"]);

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}
