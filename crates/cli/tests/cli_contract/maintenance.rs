use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

#[test]
fn db_maintenance_json_reports_size_and_prune_fields() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::write(
        project.path().join("src/lib.rs"),
        "pub fn maintenance_fixture_symbol() {}\n",
    )
    .expect("write fixture");
    let project_path = project.path().to_str().expect("utf-8 path");

    cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "semantic-index",
            "--reindex",
        ])
        .assert()
        .success();

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "db-maintenance",
            "--stats",
            "--prune",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    assert_eq!(payload["options"]["stats"], serde_json::json!(true));
    assert_eq!(payload["options"]["prune"], serde_json::json!(true));
    assert!(payload["stats"]["page_size"].is_number());
    assert!(payload["stats"]["total_size_bytes"].is_number());
    assert!(payload["stats"]["approx_free_bytes"].is_number());
    assert!(payload["prune"]["removed_databases"].is_number());
    assert!(payload["prune"]["removed_sidecars"].is_number());
    assert!(payload["prune"]["removed_bytes"].is_number());
}
