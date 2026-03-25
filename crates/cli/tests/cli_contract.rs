use std::path::{Path, PathBuf};

use serde_json::Value;

#[path = "../../core/tests/support/investigation_fixture.rs"]
mod investigation_fixture;
#[allow(dead_code)]
#[path = "../../mcp-server/src/rpc_tools_tests_helpers/schema.rs"]
mod schema_helper;

fn load_schema(file_name: &str) -> Value {
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schemas")
        .join(file_name);
    let raw = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|err| panic!("failed to read schema {}: {err}", schema_path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("failed to parse schema {}: {err}", schema_path.display()))
}

mod cli_contract {
    use super::*;

    mod benchmark;
    mod constraint_surface;
    mod index_lifecycle;
    mod investigation;
    mod json_and_runtime;
    mod maintenance;
    mod navigation;
    mod navigation_python;
    mod preflight;
    mod quality_hotspots;
    mod quality_matrix;

    fn write_query_benchmark_fixture(project_root: &Path) -> (PathBuf, PathBuf) {
        std::fs::create_dir_all(project_root.join("src")).expect("create src");
        std::fs::write(
            project_root.join("src/main.rs"),
            "fn benchmark_fixture_symbol() -> &'static str { \"ok\" }\n",
        )
        .expect("write rust source");

        let dataset_path = project_root.join("query_benchmark_dataset.json");
        std::fs::write(
            &dataset_path,
            r#"{
  "queries": [
    {
      "query": "benchmark_fixture_symbol",
      "qrels": [{"path": "src/main.rs", "relevance": 1.0}]
    }
  ]
}"#,
        )
        .expect("write benchmark dataset");

        let baseline_path = project_root.join("query_benchmark_baseline_summary.json");
        std::fs::write(
            &baseline_path,
            r#"{
  "median": {
    "recall_at_k": 0.10,
    "mrr_at_k": 0.10,
    "ndcg_at_k": 0.10,
    "avg_estimated_tokens": 1200.0,
    "latency_p50_ms": 15.0,
    "latency_p95_ms": 20.0
  }
}"#,
        )
        .expect("write baseline summary");

        (dataset_path, baseline_path)
    }

    fn write_utf8_bom_json(path: &Path, json_content: &str) {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(json_content.as_bytes());
        std::fs::write(path, bytes).expect("write utf-8 bom json");
    }

    fn assert_required_structure(value: &Value, schema: &Value, context: &str) {
        schema_helper::assert_required_structure(value, schema, context);
    }

    fn validate_schema_keyword_coverage(schema: &Value, context: &str) {
        schema_helper::validate_schema_keyword_coverage(schema, context)
            .unwrap_or_else(|err| panic!("schema keyword validation failed at {context}: {err}"));
    }
}
