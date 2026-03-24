use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

#[test]
fn delete_index_without_yes_does_not_initialize_default_db() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let default_db_dir = project.path().join(".rmu");
    let default_db_path = default_db_dir.join("index.db");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args(["--project-path", project_path, "delete-index"])
        .assert()
        .code(1);

    let stderr =
        String::from_utf8(assert.get_output().stderr.clone()).expect("stderr should be utf-8");
    assert!(stderr.contains("delete-index requires --yes"));
    assert!(
        !default_db_dir.exists(),
        "db directory should not be created"
    );
    assert!(!default_db_path.exists(), "db file should not be created");
}

#[test]
fn search_without_index_returns_stable_code_and_does_not_autoindex() {
    let project = tempdir().expect("temp dir");
    std::fs::write(
        project.path().join("sample.rs"),
        "fn answer() -> i32 { 42 }\n",
    )
    .expect("write fixture");
    let project_path = project.path().to_str().expect("utf-8 path");

    let search = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "search",
            "--query",
            "answer",
        ])
        .assert()
        .code(1);

    assert!(search.get_output().stderr.is_empty());
    let search_stdout =
        String::from_utf8(search.get_output().stdout.clone()).expect("stdout must be utf8");
    let search_payload: serde_json::Value =
        serde_json::from_str(&search_stdout).expect("stdout should be JSON");
    assert_eq!(search_payload["ok"], serde_json::json!(false));
    assert_eq!(
        search_payload["code"],
        serde_json::json!("E_INDEX_NOT_READY")
    );
    assert!(
        search_payload["error"]
            .as_str()
            .unwrap_or_default()
            .contains("index is empty")
    );

    let status = cargo_bin_cmd!("rmu-cli")
        .args(["--project-path", project_path, "--json", "status"])
        .assert()
        .success();
    let status_stdout =
        String::from_utf8(status.get_output().stdout.clone()).expect("stdout must be utf8");
    let status_payload: serde_json::Value =
        serde_json::from_str(&status_stdout).expect("status should output JSON");
    assert_eq!(status_payload["files"], serde_json::json!(0));
}

#[test]
fn search_with_auto_index_builds_index_and_returns_hits() {
    let project = tempdir().expect("temp dir");
    std::fs::write(
        project.path().join("sample.rs"),
        "fn answer_symbol() -> i32 { 42 }\n",
    )
    .expect("write fixture");
    let project_path = project.path().to_str().expect("utf-8 path");

    let search = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "search",
            "--query",
            "answer_symbol",
            "--auto-index",
        ])
        .assert()
        .success();

    let search_stdout =
        String::from_utf8(search.get_output().stdout.clone()).expect("stdout must be utf8");
    let search_payload: serde_json::Value =
        serde_json::from_str(&search_stdout).expect("stdout should be JSON");
    assert!(
        search_payload
            .as_array()
            .is_some_and(|hits| !hits.is_empty())
    );

    let status = cargo_bin_cmd!("rmu-cli")
        .args(["--project-path", project_path, "--json", "status"])
        .assert()
        .success();
    let status_stdout =
        String::from_utf8(status.get_output().stdout.clone()).expect("stdout must be utf8");
    let status_payload: serde_json::Value =
        serde_json::from_str(&status_stdout).expect("status should output JSON");
    assert!(status_payload["files"].as_u64().unwrap_or(0) >= 1);
}

#[test]
fn invalid_query_limits_fail_before_engine_initialization() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let default_db_dir = project.path().join(".rmu");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "search",
            "--query",
            "needle",
            "--limit",
            "0",
        ])
        .assert()
        .code(1);

    let stderr =
        String::from_utf8(assert.get_output().stderr.clone()).expect("stderr should be utf-8");
    assert!(stderr.contains("`limit` must be >= 1, got 0"));
    assert!(
        !default_db_dir.exists(),
        "db directory should not be created"
    );
}

#[test]
fn oversized_query_limit_fails_before_engine_initialization() {
    if usize::BITS < 64 {
        return;
    }

    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let default_db_dir = project.path().join(".rmu");
    let oversized_limit = (i64::MAX as u128 + 1).to_string();

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "search",
            "--query",
            "needle",
            "--limit",
            &oversized_limit,
        ])
        .assert()
        .code(1);

    let stderr =
        String::from_utf8(assert.get_output().stderr.clone()).expect("stderr should be utf-8");
    assert!(stderr.contains("`limit` must be <="));
    assert!(
        !default_db_dir.exists(),
        "db directory should not be created"
    );
}

#[test]
fn invalid_index_profile_fails_before_engine_initialization() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let default_db_dir = project.path().join(".rmu");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "index",
            "--profile",
            "unknown",
        ])
        .assert()
        .code(1);

    let stderr =
        String::from_utf8(assert.get_output().stderr.clone()).expect("stderr should be utf-8");
    assert!(
        stderr
            .contains("`profile` must be one of: rust-monorepo, mixed, docs-heavy (got `unknown`)")
    );
    assert!(
        !default_db_dir.exists(),
        "db directory should not be created"
    );
}

#[test]
fn semantic_index_json_reports_profile_and_applies_profile_scope() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("docs")).expect("create docs");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::write(
        project.path().join("docs/guide.md"),
        "cli_docs_heavy_marker\n",
    )
    .expect("write docs");
    std::fs::write(
        project.path().join("src/main.rs"),
        "fn cli_code_marker() {}\n",
    )
    .expect("write src");
    let project_path = project.path().to_str().expect("utf-8 path");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "semantic-index",
            "--profile",
            "docs-heavy",
            "--reindex",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert_eq!(payload["profile"], serde_json::json!("docs-heavy"));
    assert_eq!(payload["indexed"], serde_json::json!(1));

    let status = cargo_bin_cmd!("rmu-cli")
        .args(["--project-path", project_path, "--json", "status"])
        .assert()
        .success();
    let status_stdout =
        String::from_utf8(status.get_output().stdout.clone()).expect("stdout must be utf8");
    let status_payload: serde_json::Value =
        serde_json::from_str(&status_stdout).expect("status should output JSON");
    assert_eq!(status_payload["files"], serde_json::json!(1));
}

#[test]
fn invalid_changed_since_fails_before_engine_initialization() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let default_db_dir = project.path().join(".rmu");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "index",
            "--changed-since",
            "2026-03-15T10:00:00",
        ])
        .assert()
        .code(1);

    let stderr =
        String::from_utf8(assert.get_output().stderr.clone()).expect("stderr should be utf-8");
    assert!(stderr.contains("`changed_since` must be RFC3339 timestamp with timezone"));
    assert!(
        !default_db_dir.exists(),
        "db directory should not be created"
    );
}

#[test]
fn scope_preview_invalid_changed_since_fails_before_engine_initialization() {
    let project = tempdir().expect("temp dir");
    let project_path = project.path().to_str().expect("utf-8 path");
    let default_db_dir = project.path().join(".rmu");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "scope-preview",
            "--changed-since",
            "2026-03-15T10:00:00",
        ])
        .assert()
        .code(1);

    let stderr =
        String::from_utf8(assert.get_output().stderr.clone()).expect("stderr should be utf-8");
    assert!(stderr.contains("`changed_since` must be RFC3339 timestamp with timezone"));
    assert!(
        !default_db_dir.exists(),
        "db directory should not be created"
    );
}

#[test]
fn scope_preview_help_describes_selector_contract() {
    let assert = cargo_bin_cmd!("rmu-cli")
        .args(["scope-preview", "--help"])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    assert!(stdout.contains("scope-preview"));
    assert!(stdout.contains("--changed-since"));
    assert!(stdout.contains("--changed-since-commit"));
}

#[test]
fn scope_preview_json_reports_full_scope_arrays_without_creating_db() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::create_dir_all(project.path().join("vendor")).expect("create vendor");
    std::fs::create_dir_all(project.path().join("target")).expect("create target");
    std::fs::write(
        project.path().join("src/main.rs"),
        "fn preview_cli_kept() {}\n",
    )
    .expect("write src");
    std::fs::write(
        project.path().join("vendor/skip.rs"),
        "fn preview_cli_excluded() {}\n",
    )
    .expect("write vendor");
    std::fs::write(
        project.path().join("target/generated.rs"),
        "fn preview_cli_ignored() {}\n",
    )
    .expect("write target");
    let project_path = project.path().to_str().expect("utf-8 path");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "scope-preview",
            "--include",
            "**/*.rs",
            "--exclude",
            "vendor/**",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert_eq!(
        payload["candidate_paths"],
        serde_json::json!(["src/main.rs"])
    );
    assert_eq!(
        payload["excluded_by_scope_paths"],
        serde_json::json!(["vendor/skip.rs"])
    );
    assert_eq!(
        payload["ignored_paths"],
        serde_json::json!(["target/generated.rs"])
    );
    assert!(!project.path().join(".rmu").exists());
}

#[test]
fn scope_preview_text_truncates_bucket_output() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    for idx in 0..25 {
        std::fs::write(
            project.path().join(format!("src/file_{idx:02}.rs")),
            format!("fn preview_text_{idx}() {{}}\n"),
        )
        .expect("write rust source");
    }
    let project_path = project.path().to_str().expect("utf-8 path");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args(["--project-path", project_path, "scope-preview"])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    assert!(stdout.contains("candidate_paths="));
    assert!(stdout.contains("candidate_paths_truncated="));
    assert!(stdout.contains("… +5 more"));
}

#[test]
fn semantic_index_json_reports_changed_since_and_skip_count() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::write(
        project.path().join("src/old.rs"),
        "fn old_cli_symbol() {}\n",
    )
    .expect("write old");
    std::fs::write(
        project.path().join("src/fresh.rs"),
        "fn fresh_cli_symbol() {}\n",
    )
    .expect("write fresh");
    let project_path = project.path().to_str().expect("utf-8 path");

    cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "semantic-index",
            "--reindex",
        ])
        .assert()
        .success();

    std::thread::sleep(std::time::Duration::from_millis(1200));
    let cutoff = OffsetDateTime::now_utc()
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .expect("format cutoff");
    std::thread::sleep(std::time::Duration::from_millis(1200));
    std::fs::write(
        project.path().join("src/fresh.rs"),
        "fn fresh_cli_symbol() { println!(\"updated\"); }\n",
    )
    .expect("rewrite fresh");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "semantic-index",
            "--changed-since",
            &cutoff,
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert_eq!(payload["changed_since"], serde_json::json!(cutoff));
    assert_eq!(
        payload["skipped_before_changed_since"],
        serde_json::json!(1)
    );
    assert_eq!(payload["indexed"], serde_json::json!(1));
}
