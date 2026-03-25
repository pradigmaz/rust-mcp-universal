use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use assert_cmd::cargo::cargo_bin_cmd;
use rusqlite::Connection;
use tempfile::tempdir;

#[test]
fn parse_errors_are_emitted_as_json_when_json_flag_is_present() {
    let assert = cargo_bin_cmd!("rmu-cli")
        .args(["--json", "search"])
        .assert()
        .code(2);

    assert!(assert.get_output().stderr.is_empty());
    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be JSON");
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert_eq!(payload["code"], serde_json::json!("E_PARSE_ARGS"));
    let error = payload["error"].as_str().unwrap_or_default();
    assert!(error.contains("--query <QUERY>"));
}

#[test]
fn global_json_flag_works_after_subcommand() {
    let project = tempdir().expect("temp dir");
    let db_path = project.path().join(".rmu/index.db");
    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "status",
            "--json",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("status should output JSON");
    assert!(payload.get("files").is_some());
    assert!(!db_path.exists());
}

#[test]
fn runtime_errors_keep_json_envelope() {
    let project = tempdir().expect("temp dir");
    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--json",
            "delete-index",
        ])
        .assert()
        .code(1);

    assert!(assert.get_output().stderr.is_empty());
    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be JSON");
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert_eq!(payload["code"], serde_json::json!("E_CONFIRM_REQUIRED"));
    let error = payload["error"].as_str().unwrap_or_default();
    assert!(error.contains("delete-index requires --yes"));
}

#[test]
fn panics_in_json_mode_keep_runtime_envelope_without_stderr_noise() {
    let project = tempdir().expect("temp dir");
    let assert = cargo_bin_cmd!("rmu-cli")
        .env("RMU_TEST_PANIC", "1")
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--json",
            "status",
        ])
        .assert()
        .code(1);

    assert!(assert.get_output().stderr.is_empty());
    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be JSON");
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert_eq!(payload["code"], serde_json::json!("E_RUNTIME"));
    let error = payload["error"].as_str().unwrap_or_default();
    assert!(error.contains("contract test panic"));
    assert!(!stdout.contains("thread 'main'"));
    assert!(!stdout.contains("panicked at"));
}

#[test]
fn broken_stdout_in_json_mode_falls_back_to_stderr_without_raw_panic_text() {
    let project = tempdir().expect("temp dir");
    let fallback_path = project.path().join("stderr-should-win.json");
    let mut child = Command::new(assert_cmd::cargo::cargo_bin!("rmu-cli"))
        .env("RMU_JSON_FALLBACK_PATH", &fallback_path)
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--json",
            "status",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rmu-cli");

    drop(child.stdout.take());
    let mut stderr_handle = child.stderr.take().expect("stderr pipe");
    let status = child.wait().expect("wait for child");
    let mut stderr = Vec::new();
    stderr_handle
        .read_to_end(&mut stderr)
        .expect("read stderr bytes");

    assert_eq!(status.code(), Some(1));
    let stderr_text = String::from_utf8(stderr).expect("stderr must be utf8");
    let payload: serde_json::Value =
        serde_json::from_str(&stderr_text).expect("stderr fallback should be JSON");
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert_eq!(payload["code"], serde_json::json!("E_RUNTIME"));
    assert!(!stderr_text.contains("thread 'main'"));
    assert!(!stderr_text.contains("panicked at"));
    assert!(!fallback_path.exists());
}

#[test]
fn parse_errors_with_closed_stdout_and_stderr_use_env_fallback_file() {
    let project = tempdir().expect("temp dir");
    let fallback_path = project.path().join("parse-error-fallback.json");
    let mut child = Command::new(assert_cmd::cargo::cargo_bin!("rmu-cli"))
        .env("RMU_JSON_FALLBACK_PATH", &fallback_path)
        .args(["--json", "search"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rmu-cli");

    close_child_stdio(&mut child);
    let status = child.wait().expect("wait for child");

    assert_eq!(status.code(), Some(2));
    let payload = read_json_file(&fallback_path);
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert_eq!(payload["code"], serde_json::json!("E_PARSE_ARGS"));
    let error = payload["error"].as_str().unwrap_or_default();
    assert!(error.contains("--query <QUERY>"));
}

#[test]
fn runtime_errors_with_closed_stdout_and_stderr_use_env_fallback_file() {
    let project = tempdir().expect("temp dir");
    let fallback_path = project.path().join("runtime-error-fallback.json");
    let mut child = Command::new(assert_cmd::cargo::cargo_bin!("rmu-cli"))
        .env("RMU_JSON_FALLBACK_PATH", &fallback_path)
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--json",
            "delete-index",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rmu-cli");

    close_child_stdio(&mut child);
    let status = child.wait().expect("wait for child");

    assert_eq!(status.code(), Some(1));
    let payload = read_json_file(&fallback_path);
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert_eq!(payload["code"], serde_json::json!("E_CONFIRM_REQUIRED"));
    let error = payload["error"].as_str().unwrap_or_default();
    assert!(error.contains("delete-index requires --yes"));
}

#[test]
fn panics_with_closed_stdout_and_stderr_use_env_fallback_file_without_raw_panic_text() {
    let project = tempdir().expect("temp dir");
    let fallback_path = project.path().join("panic-fallback.json");
    let mut child = Command::new(assert_cmd::cargo::cargo_bin!("rmu-cli"))
        .env("RMU_JSON_FALLBACK_PATH", &fallback_path)
        .env("RMU_TEST_PANIC", "1")
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--json",
            "status",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rmu-cli");

    close_child_stdio(&mut child);
    let status = child.wait().expect("wait for child");

    assert_eq!(status.code(), Some(1));
    let payload_text = fs::read_to_string(&fallback_path).expect("read fallback payload");
    let payload: serde_json::Value =
        serde_json::from_str(&payload_text).expect("fallback file should contain JSON");
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert_eq!(payload["code"], serde_json::json!("E_RUNTIME"));
    let error = payload["error"].as_str().unwrap_or_default();
    assert!(error.contains("contract test panic"));
    assert!(!payload_text.contains("thread 'main'"));
    assert!(!payload_text.contains("panicked at"));
}

#[test]
fn blank_env_fallback_uses_automatic_temp_file_sink() {
    let project = tempdir().expect("temp dir");
    let automatic_fallback_path = automatic_json_fallback_path();
    let _ = fs::remove_file(&automatic_fallback_path);
    let mut child = Command::new(assert_cmd::cargo::cargo_bin!("rmu-cli"))
        .env("RMU_JSON_FALLBACK_PATH", "   ")
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--json",
            "delete-index",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rmu-cli");

    close_child_stdio(&mut child);
    let status = child.wait().expect("wait for child");

    assert_eq!(status.code(), Some(1));
    let payload = read_json_file(&automatic_fallback_path);
    assert_eq!(payload["ok"], serde_json::json!(false));
    assert_eq!(payload["code"], serde_json::json!("E_CONFIRM_REQUIRED"));
    let _ = fs::remove_file(&automatic_fallback_path);
}

fn close_child_stdio(child: &mut Child) {
    drop(child.stdout.take());
    drop(child.stderr.take());
}

fn read_json_file(path: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(path).expect("read fallback file");
    serde_json::from_str(&raw).expect("fallback file should contain JSON")
}

fn automatic_json_fallback_path() -> std::path::PathBuf {
    env::temp_dir().join("rmu-cli-json-error-latest.json")
}

#[test]
fn compatibility_errors_use_structured_json_details() {
    let project = tempdir().expect("temp dir");
    let db_dir = project.path().join(".rmu");
    fs::create_dir_all(&db_dir).expect("create db dir");
    let db_path = db_dir.join("index.db");
    let conn = Connection::open(&db_path).expect("open db");
    conn.execute_batch("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);")
        .expect("create meta");
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)",
        rusqlite::params!["schema_version", "999"],
    )
    .expect("insert schema version");
    drop(conn);

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--json",
            "search",
            "--query",
            "compatibility",
        ])
        .assert()
        .code(1);

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be JSON");
    assert_eq!(payload["code"], serde_json::json!("E_COMPATIBILITY"));
    assert_eq!(
        payload["details"]["kind"],
        serde_json::json!("compatibility")
    );
    assert!(payload["details"]["running_binary_version"].is_string());
    assert!(payload["details"].get("stale_process_suspected").is_none());
    assert!(payload["details"]["safe_recovery_hint"].is_string());
}
