use std::path::{Path, PathBuf};

mod cli_contract {
    use super::*;

    mod benchmark;
    mod index_lifecycle;
    mod json_and_runtime;
    mod maintenance;
    mod navigation;
    mod navigation_python;

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
}
