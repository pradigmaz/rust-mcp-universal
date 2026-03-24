use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::tempdir;

fn run_quality_hotspots(project_root: &std::path::Path, aggregation: &str) -> Value {
    let assert = cargo_bin_cmd!("rmu-cli")
        .arg("--project-path")
        .arg(project_root.to_str().expect("utf-8 project path"))
        .arg("--json")
        .arg("quality-hotspots")
        .arg("--aggregation")
        .arg(aggregation)
        .arg("--auto-index")
        .assert()
        .success();
    serde_json::from_slice(&assert.get_output().stdout).expect("json stdout")
}

#[test]
fn quality_hotspots_file_mode_returns_risk_scores() {
    let project = tempdir().expect("tempdir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::write(
        project.path().join("src/lib.rs"),
        "pub fn noisy() {\n  let _value = \"this line is intentionally very very very very very very very very very very very very very very very very very very very long\";\n}\n",
    )
    .expect("write source");

    let result = run_quality_hotspots(project.path(), "file");

    assert_eq!(result["summary"]["aggregation"], serde_json::json!("file"));
    assert!(result["summary"]["evaluated_buckets"].is_u64());
    assert!(result["buckets"].as_array().is_some());
    assert!(
        result["buckets"][0]["risk_score"]["score"].is_number(),
        "file buckets should expose risk_score"
    );
}

#[test]
fn quality_hotspots_module_mode_falls_back_to_directory_without_zones() {
    let project = tempdir().expect("tempdir");
    std::fs::create_dir_all(project.path().join("src/a")).expect("create src/a");
    std::fs::create_dir_all(project.path().join("src/b")).expect("create src/b");
    std::fs::write(project.path().join("src/a/lib.rs"), "pub fn a() {}\n").expect("write a");
    std::fs::write(project.path().join("src/b/lib.rs"), "pub fn b() {}\n").expect("write b");

    let directory = run_quality_hotspots(project.path(), "directory");
    let module = run_quality_hotspots(project.path(), "module");

    let directory_ids = directory["buckets"]
        .as_array()
        .expect("directory buckets")
        .iter()
        .map(|bucket| bucket["bucket_id"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    let module_ids = module["buckets"]
        .as_array()
        .expect("module buckets")
        .iter()
        .map(|bucket| bucket["bucket_id"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();

    assert_eq!(directory_ids, module_ids);
}
