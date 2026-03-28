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

#[test]
fn install_ignore_rules_defaults_to_git_info_exclude_without_initializing_db() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join(".git/info")).expect("create git info");
    let project_path = project.path().to_str().expect("utf-8 path");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "install-ignore-rules",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    assert_eq!(payload["target"], serde_json::json!("git-info-exclude"));
    assert_eq!(payload["created"], serde_json::json!(true));
    assert_eq!(payload["updated"], serde_json::json!(true));
    assert!(
        payload["path"]
            .as_str()
            .unwrap_or_default()
            .ends_with(".git/info/exclude")
    );
    assert!(
        project.path().join(".git/info/exclude").exists(),
        "exclude file should be created"
    );
    assert!(
        !project.path().join(".gitignore").exists(),
        "root .gitignore should remain untouched by default"
    );
    assert!(
        !project.path().join(".rmu").exists(),
        "install-ignore-rules should not initialize the default DB directory"
    );

    let exclude =
        std::fs::read_to_string(project.path().join(".git/info/exclude")).expect("read exclude");
    assert!(exclude.contains(".rmu/"));
    assert!(exclude.contains(".codex/"));
}

#[test]
fn preflight_json_reports_status_and_recovery_hint() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::write(
        project.path().join("src/lib.rs"),
        "pub fn preflight_fixture_symbol() {}\n",
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
        .args(["--project-path", project_path, "--json", "preflight"])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    assert!(payload["status"].is_string());
    assert!(payload["project_path"].is_string());
    assert!(payload["binary_path"].is_string());
    assert!(payload["running_binary_version"].is_string());
    assert!(payload["running_binary_stale"].is_boolean());
    assert!(
        payload.get("stale_process_probe_binary_path").is_none()
            || payload["stale_process_probe_binary_path"].is_string()
    );
    assert!(payload["supported_schema_version"].is_number());
    assert!(payload["db_schema_version"].is_number());
    assert!(payload["safe_recovery_hint"].is_string());
    assert!(payload["same_binary_other_pids"].is_array());
    assert!(payload["warnings"].is_array());
}

#[test]
fn preflight_text_reports_operator_facing_fields() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::write(
        project.path().join("src/lib.rs"),
        "pub fn preflight_text_fixture_symbol() {}\n",
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
        .args(["--project-path", project_path, "preflight"])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    assert!(stdout.contains("running_binary_version="));
    assert!(stdout.contains("running_binary_stale="));
    assert!(stdout.contains("supported_schema_version="));
    assert!(stdout.contains("db_schema_version="));
    assert!(stdout.contains("stale_process_suspected="));
    assert!(stdout.contains("safe_recovery_hint="));
}
