use std::path::{Path, PathBuf};

pub(crate) fn write_symbol_body_fixture(project_root: &Path) {
    std::fs::create_dir_all(project_root.join("src")).expect("create src");
    std::fs::write(
        project_root.join("src/lib.rs"),
        "pub fn inspect_body() {\n    println!(\"ok\");\n}\n",
    )
    .expect("write symbol body fixture");
}

pub(crate) fn write_route_and_constraint_fixture(project_root: &Path) {
    std::fs::create_dir_all(project_root.join("app/services")).expect("create services");
    std::fs::create_dir_all(project_root.join("app/models")).expect("create models");
    std::fs::create_dir_all(project_root.join("tests")).expect("create tests");
    std::fs::write(
        project_root.join("app/services/lab_service.py"),
        "def resolve_lab():\n    return True\n",
    )
    .expect("write service");
    std::fs::write(
        project_root.join("app/models/lab.py"),
        "UniqueConstraint('subject_id', 'number', name='uq_lab_subject_number')\n",
    )
    .expect("write model");
    std::fs::write(
        project_root.join("tests/test_lab_service.py"),
        "def test_resolve_lab():\n    assert True\n",
    )
    .expect("write test");
}

pub(crate) fn write_cluster_and_divergence_fixture(project_root: &Path) {
    std::fs::create_dir_all(project_root.join("src/services")).expect("create services");
    std::fs::create_dir_all(project_root.join("src/generated")).expect("create generated");
    std::fs::create_dir_all(project_root.join("frontend/src")).expect("create frontend");
    std::fs::create_dir_all(project_root.join("migrations")).expect("create migrations");
    std::fs::create_dir_all(project_root.join("tests")).expect("create tests");
    std::fs::write(
        project_root.join("src/services/origin_service.rs"),
        "pub fn origin_resolution(key: &str) { origin_resolution_validator(key); helper_query(); }\nfn helper_query() {}\n",
    )
    .expect("write service");
    std::fs::write(
        project_root.join("src/services/origin_validator.rs"),
        "pub fn origin_resolution_validator(key: &str) { assert!(!key.is_empty()); }\n",
    )
    .expect("write validator");
    std::fs::write(
        project_root.join("src/generated/origin_client.generated.ts"),
        "// generated file - do not edit\nexport function originResolutionClient(key: string) { return `/api/origin/${key}`; }\n",
    )
    .expect("write generated client");
    std::fs::write(
        project_root.join("frontend/src/origin_page.tsx"),
        "import { originResolutionClient } from '../../src/generated/origin_client.generated';\nexport function OriginPage() { return originResolutionClient('ok'); }\n",
    )
    .expect("write frontend consumer");
    std::fs::write(
        project_root.join("migrations/001_create_origins.sql"),
        "CREATE TABLE origins (id INTEGER PRIMARY KEY, origin_key TEXT NOT NULL);\n",
    )
    .expect("write migration");
    std::fs::write(
        project_root.join("tests/test_origin_resolution.py"),
        "def test_origin_resolution():\n    assert True\n",
    )
    .expect("write test");
}

#[allow(dead_code)]
pub(crate) fn write_investigation_benchmark_fixture(project_root: &Path) -> (PathBuf, PathBuf) {
    std::fs::create_dir_all(project_root.join("src/services")).expect("create services");
    std::fs::create_dir_all(project_root.join("src/validators")).expect("create validators");
    std::fs::create_dir_all(project_root.join("src/queries")).expect("create queries");
    std::fs::create_dir_all(project_root.join("migrations")).expect("create migrations");
    std::fs::create_dir_all(project_root.join("tests")).expect("create tests");
    std::fs::create_dir_all(project_root.join("legacy")).expect("create legacy");
    std::fs::create_dir_all(project_root.join("web")).expect("create web");

    std::fs::write(
        project_root.join("src/services/origin_service.rs"),
        "pub fn resolve_origin(key: &str) -> bool {\n    validate_origin(key);\n    let _query = sqlx::query!(\"SELECT id FROM origins WHERE origin_key = $1\", key);\n    true\n}\n",
    )
    .expect("write rust service");
    std::fs::write(
        project_root.join("src/validators/origin_validator.rs"),
        "pub fn validate_origin(key: &str) {\n    assert!(!key.is_empty());\n}\n",
    )
    .expect("write validator");
    std::fs::write(
        project_root.join("src/queries/origin_query.sql"),
        "SELECT id FROM origins WHERE origin_key = $1;\n",
    )
    .expect("write sql query");
    std::fs::write(
        project_root.join("migrations/001_create_origins.sql"),
        "CREATE TABLE origins (id INTEGER PRIMARY KEY, origin_key TEXT NOT NULL);\nCREATE UNIQUE INDEX uq_origins_origin_key ON origins(origin_key);\n",
    )
    .expect("write migration");
    std::fs::write(
        project_root.join("tests/test_origin_flow.py"),
        "def test_resolve_origin():\n    assert True\n",
    )
    .expect("write test");
    std::fs::write(
        project_root.join("legacy/origin_service.py"),
        "def resolve_origin(key: str) -> bool:\n    ensure_origin(key)\n    query = \"SELECT id FROM origins WHERE origin_key = ?\"\n    return bool(query)\n",
    )
    .expect("write legacy service");
    std::fs::write(
        project_root.join("legacy/origin_validator.py"),
        "def ensure_origin(key: str) -> None:\n    assert key\n",
    )
    .expect("write legacy validator");
    std::fs::write(
        project_root.join("web/origin_client.ts"),
        "export function resolveOriginClient(key: string) {\n  return `/api/origin/${key}`;\n}\n",
    )
    .expect("write ts client");

    let dataset_path = project_root.join("investigation_dataset.json");
    std::fs::write(
        &dataset_path,
        r#"{
  "cases": [
    {
      "id": "symbol-body-rust",
      "tool": "symbol_body",
      "fixture": "tmp",
      "seed": "src/services/origin_service.rs",
      "seed_kind": "path",
      "expected_capability_status": "supported",
      "expected_assertions": [
        {"kind": "contains_language", "value": "rust"},
        {"kind": "min_body_items", "value": "1"}
      ]
    },
    {
      "id": "route-trace-rust",
      "tool": "route_trace",
      "fixture": "tmp",
      "seed": "src/services/origin_service.rs",
      "seed_kind": "path",
      "expected_capability_status": "partial",
      "expected_assertions": [
        {"kind": "contains_route_kind", "value": "service"},
        {"kind": "contains_gap", "value": "call_path_unresolved:query"}
      ]
    },
    {
      "id": "constraint-rust",
      "tool": "constraint_evidence",
      "fixture": "tmp",
      "seed": "src/services/origin_service.rs",
      "seed_kind": "path",
      "expected_capability_status": "supported",
      "expected_assertions": [
        {"kind": "contains_constraint_kind", "value": "index_constraint"},
        {"kind": "strong_constraint_present", "value": "strong"}
      ]
    },
    {
      "id": "divergence-origin",
      "tool": "divergence_report",
      "fixture": "tmp",
      "seed": "resolve_origin",
      "seed_kind": "query",
      "expected_capability_status": "supported",
      "expected_assertions": [
        {"kind": "min_divergence_axes", "value": "1"},
        {"kind": "expected_severity", "value": "likely_expected"}
      ]
    }
  ]
}"#,
    )
    .expect("write investigation dataset");

    let thresholds_path = project_root.join("investigation_thresholds.json");
    std::fs::write(
        &thresholds_path,
        r#"{
  "symbol_body_supported_success": 0.9,
  "route_trace_case_pass_rate": 0.8,
  "constraint_evidence_case_pass_rate": 0.85,
  "divergence_case_pass_rate": 0.85,
  "max_latency_p95_ms": 1000.0,
  "max_unsupported_case_rate": 0.5,
  "privacy_failures": 0
}"#,
    )
    .expect("write investigation thresholds");

    (dataset_path, thresholds_path)
}
