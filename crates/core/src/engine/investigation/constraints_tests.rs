use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::engine::Engine;
use crate::model::InvestigationAnchor;

use super::*;

fn temp_project_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

fn fixture_engine(prefix: &str) -> anyhow::Result<(PathBuf, Engine)> {
    let project_dir = temp_project_dir(prefix);
    fs::create_dir_all(&project_dir)?;
    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    Ok((project_dir, engine))
}

fn test_anchor() -> InvestigationAnchor {
    InvestigationAnchor {
        path: "src/services/resolve_origin.rs".to_string(),
        language: "rust".to_string(),
        symbol: Some("resolve_origin".to_string()),
        kind: Some("function".to_string()),
        line: Some(1),
        column: Some(1),
    }
}

fn assert_canonical_fields(
    item: &crate::model::ConstraintEvidence,
    constraint_kind: &str,
    source_kind: &str,
    path_suffix: &str,
) {
    assert_eq!(item.constraint_kind, constraint_kind);
    assert_eq!(item.kind, constraint_kind);
    assert_eq!(item.source_kind, source_kind);
    assert!(item.path.ends_with(path_suffix));
    assert_eq!(item.source_path, item.path);
    assert!(item.line_start >= 1);
    assert_eq!(item.line_end, item.line_start);
    assert_eq!(
        item.source_span.as_ref().map(|span| span.start_line),
        Some(item.line_start)
    );
    assert!(!item.excerpt.is_empty());
    assert!(!item.normalized_key.is_empty());
    assert!((0.0..=1.0).contains(&item.confidence));
}

#[test]
fn python_adapter_emits_model_constraint_shape() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-python-constraint")?;
    fs::create_dir_all(project_dir.join("app/models"))?;
    fs::write(
        project_dir.join("app/models/origin.py"),
        "UniqueConstraint(\"tenant_id\", \"origin_key\", name=\"uq_origin\")\n",
    )?;

    let items = collect_constraint_evidence(
        &engine,
        &test_anchor(),
        &[String::from("app/models/origin.py")],
    )?;
    let item = items.first().expect("python evidence");
    assert_canonical_fields(item, "model_constraint", "model_declaration", "origin.py");
    assert_eq!(item.strength, "strong");

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn python_adapter_emits_migration_index_constraint_shape() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-python-index-constraint")?;
    fs::create_dir_all(project_dir.join("alembic/versions"))?;
    fs::write(
        project_dir.join("alembic/versions/001_add_origin_index.py"),
        "op.create_index('idx_origins_tenant_key', 'origins', ['tenant_id', 'origin_key'])\n",
    )?;

    let items = collect_constraint_evidence(
        &engine,
        &test_anchor(),
        &[String::from("alembic/versions/001_add_origin_index.py")],
    )?;
    let item = items.first().expect("python index evidence");
    assert_canonical_fields(
        item,
        "index_constraint",
        "index_declaration",
        "001_add_origin_index.py",
    );
    assert_eq!(item.strength, "strong");

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn python_adapter_emits_model_index_constraint_shape() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-python-model-index")?;
    fs::create_dir_all(project_dir.join("app/models"))?;
    fs::write(
        project_dir.join("app/models/origin.py"),
        "Index('idx_origins_tenant_key', 'tenant_id', 'origin_key')\n",
    )?;

    let items = collect_constraint_evidence(
        &engine,
        &test_anchor(),
        &[String::from("app/models/origin.py")],
    )?;
    let item = items.first().expect("python model index evidence");
    assert_canonical_fields(item, "index_constraint", "index_declaration", "origin.py");
    assert_eq!(item.strength, "strong");

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn typescript_adapter_emits_model_constraint_shape() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-ts-constraint")?;
    fs::create_dir_all(project_dir.join("web"))?;
    fs::write(
        project_dir.join("web/origin.ts"),
        "originKey: { type: String, unique: true }\n",
    )?;

    let items =
        collect_constraint_evidence(&engine, &test_anchor(), &[String::from("web/origin.ts")])?;
    let item = items.first().expect("typescript evidence");
    assert_canonical_fields(item, "model_constraint", "model_declaration", "origin.ts");
    assert_eq!(item.strength, "strong");

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn rust_adapter_emits_weak_schema_hint_shape() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-rust-constraint")?;
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/schema.rs"),
        "table! { origins (id) { id -> Int4, origin_key -> Text, } }\n",
    )?;

    let items =
        collect_constraint_evidence(&engine, &test_anchor(), &[String::from("src/schema.rs")])?;
    let item = items.first().expect("rust evidence");
    assert_canonical_fields(item, "ddl_like_hint", "schema_hint", "schema.rs");
    assert_eq!(item.strength, "weak");

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn sql_adapter_emits_index_constraint_shape() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-sql-constraint")?;
    fs::create_dir_all(project_dir.join("migrations"))?;
    fs::write(
        project_dir.join("migrations/001_create_origins.sql"),
        "CREATE UNIQUE INDEX uq_origins_origin_key ON origins(origin_key);\n",
    )?;

    let items = collect_constraint_evidence(
        &engine,
        &test_anchor(),
        &[String::from("migrations/001_create_origins.sql")],
    )?;
    let item = items.first().expect("sql evidence");
    assert_canonical_fields(
        item,
        "index_constraint",
        "index_declaration",
        "001_create_origins.sql",
    );
    assert_eq!(item.strength, "strong");

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn prisma_adapter_emits_model_constraint_shape() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-prisma-constraint")?;
    fs::create_dir_all(project_dir.join("schema"))?;
    fs::write(
        project_dir.join("schema/schema.prisma"),
        "originKey String @unique\n",
    )?;

    let items = collect_constraint_evidence(
        &engine,
        &test_anchor(),
        &[String::from("schema/schema.prisma")],
    )?;
    let item = items.first().expect("prisma evidence");
    assert_canonical_fields(
        item,
        "model_constraint",
        "model_declaration",
        "schema.prisma",
    );
    assert_eq!(item.strength, "strong");

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn conflicting_model_and_migration_evidence_are_returned_separately() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-conflicting-constraint")?;
    fs::create_dir_all(project_dir.join("app/models"))?;
    fs::create_dir_all(project_dir.join("migrations"))?;
    fs::write(
        project_dir.join("app/models/origin.py"),
        "UniqueConstraint(\"tenant_id\", \"origin_key\", name=\"uq_model_origin\")\n",
    )?;
    fs::write(
        project_dir.join("migrations/001_create_origins.sql"),
        "ALTER TABLE origins ADD CONSTRAINT uq_db_origin UNIQUE (origin_key);\n",
    )?;

    let items = collect_constraint_evidence(
        &engine,
        &test_anchor(),
        &[
            String::from("app/models/origin.py"),
            String::from("migrations/001_create_origins.sql"),
        ],
    )?;

    assert!(items.iter().any(|item| {
        item.constraint_kind == "model_constraint"
            && item.source_kind == "model_declaration"
            && item.path.ends_with("origin.py")
    }));
    assert!(items.iter().any(|item| {
        item.constraint_kind == "migration_constraint"
            && item.source_kind == "migration_declaration"
            && item.path.ends_with("001_create_origins.sql")
    }));

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn generic_weak_fallback_stays_low_confidence_without_strong_backing() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-weak-fallback")?;
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/origin_guard.py"),
        "assert origin_key is not None\n",
    )?;

    let items = collect_constraint_evidence(
        &engine,
        &test_anchor(),
        &[String::from("src/origin_guard.py")],
    )?;
    let item = items.first().expect("weak fallback evidence");
    assert_canonical_fields(
        item,
        "runtime_guard",
        "runtime_guard_code",
        "origin_guard.py",
    );
    assert_eq!(item.strength, "weak");
    assert!(item.confidence <= 0.5);

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn comment_only_lines_do_not_emit_constraint_noise() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-comment-noise")?;
    fs::create_dir_all(project_dir.join("migrations"))?;
    fs::write(
        project_dir.join("migrations/001_comment_only.py"),
        "# Add constraints and indexes\n\"\"\"Revision ID: test\"\"\"\n",
    )?;

    let items = collect_constraint_evidence(
        &engine,
        &test_anchor(),
        &[String::from("migrations/001_comment_only.py")],
    )?;
    assert!(items.is_empty());

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn frontend_jsx_tokens_do_not_emit_constraint_noise() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-frontend-noise")?;
    fs::create_dir_all(project_dir.join("frontend/components"))?;
    fs::write(
        project_dir.join("frontend/components/lab_view.tsx"),
        "<div tabIndex={0}>{items.map((item, index) => item.name)}</div>\n",
    )?;

    let items = collect_constraint_evidence(
        &engine,
        &test_anchor(),
        &[String::from("frontend/components/lab_view.tsx")],
    )?;
    assert!(items.is_empty());

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn model_validate_calls_are_not_mistaken_for_runtime_guards() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-model-validate-noise")?;
    fs::create_dir_all(project_dir.join("app/api"))?;
    fs::write(
        project_dir.join("app/api/admin_labs.py"),
        "return LabDetailResponse.model_validate(lab)\n",
    )?;

    let items = collect_constraint_evidence(
        &engine,
        &test_anchor(),
        &[String::from("app/api/admin_labs.py")],
    )?;
    assert!(items.is_empty());

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn generic_http_exception_and_presence_guards_do_not_emit_constraint_noise() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-http-guard-noise")?;
    fs::create_dir_all(project_dir.join("backend/app/api"))?;
    fs::write(
        project_dir.join("backend/app/api/example.py"),
        "if not student:\n    raise HTTPException(status_code=404, detail=\"Student not found\")\n",
    )?;

    let items = collect_constraint_evidence(
        &engine,
        &test_anchor(),
        &[String::from("backend/app/api/example.py")],
    )?;
    assert!(items.is_empty());

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn validator_guards_with_constraint_keywords_still_emit_runtime_guard() -> anyhow::Result<()> {
    let (project_dir, engine) = fixture_engine("rmu-investigation-validator-guard")?;
    fs::create_dir_all(project_dir.join("backend/app/services"))?;
    fs::write(
        project_dir.join("backend/app/services/deadline_validator.py"),
        "if not deadline_limit:\n    raise ValueError(\"deadline limit is required\")\n",
    )?;

    let items = collect_constraint_evidence(
        &engine,
        &test_anchor(),
        &[String::from("backend/app/services/deadline_validator.py")],
    )?;
    assert!(
        items
            .iter()
            .any(|item| item.constraint_kind == "runtime_guard")
    );

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}
