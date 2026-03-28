use assert_cmd::cargo::cargo_bin_cmd;
use rusqlite::Connection;
use std::path::PathBuf;
use tempfile::tempdir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

#[test]
fn preflight_json_reports_ok_status() {
    let project = tempdir().expect("temp dir");
    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--json",
            "preflight",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("preflight payload");
    let status = payload["status"].as_str().unwrap_or_default();
    assert!(matches!(status, "ok" | "warning"));
    if status == "warning" {
        assert_eq!(payload["stale_process_suspected"], serde_json::json!(true));
        assert!(
            payload["same_binary_other_pids"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        );
    }
    assert!(payload["same_binary_other_pids"].is_array());
    assert!(payload["running_binary_version"].is_string());
    assert!(payload["running_binary_stale"].is_boolean());
    assert!(
        payload.get("stale_process_probe_binary_path").is_none()
            || payload["stale_process_probe_binary_path"].is_string()
    );
    assert!(payload["errors"].is_array());
    assert!(payload["warnings"].is_array());
    assert!(payload["safe_recovery_hint"].is_string());
}

#[test]
fn preflight_json_reports_incompatible_status_for_future_schema() {
    let project = tempdir().expect("temp dir");
    let db_dir = project.path().join(".rmu");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
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
            "--db-path",
            db_path.to_str().expect("utf-8 db path"),
            "--json",
            "preflight",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("preflight payload");
    assert_eq!(payload["status"], serde_json::json!("incompatible"));
    assert!(payload["running_binary_version"].is_string());
    assert_eq!(payload["running_binary_stale"], serde_json::json!(false));
    assert!(
        payload.get("stale_process_probe_binary_path").is_none()
            || payload["stale_process_probe_binary_path"].is_string()
    );
    assert!(
        payload["errors"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(payload["warnings"].is_array());
    assert!(
        payload["safe_recovery_hint"]
            .as_str()
            .is_some_and(|hint| !hint.is_empty())
    );
}

#[cfg(windows)]
#[test]
fn preflight_json_detects_running_mcp_server_via_probe_binary_path() {
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    let project = tempdir().expect("temp dir");
    let server_binary = workspace_root().join("target/debug/rmu-mcp-server.exe");
    let mut child = Command::new(server_binary)
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rmu-mcp-server");

    thread::sleep(Duration::from_millis(750));

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--json",
            "preflight",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("preflight payload");
    assert_eq!(payload["status"], serde_json::json!("warning"));
    assert_eq!(payload["stale_process_suspected"], serde_json::json!(true));
    assert_eq!(payload["running_binary_stale"], serde_json::json!(false));
    assert!(
        payload["same_binary_other_pids"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(
        payload["stale_process_probe_binary_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("rmu-mcp-server.exe"))
    );
    assert!(payload["warnings"].is_array());

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn preflight_json_reports_incompatible_status_for_stale_running_binary() {
    let project = tempdir().expect("temp dir");
    let assert = cargo_bin_cmd!("rmu-cli")
        .env("RMU_TEST_PROCESS_STARTED_AT_MS", "1000")
        .env("RMU_TEST_BINARY_MODIFIED_AT_MS", "4001")
        .args([
            "--project-path",
            project.path().to_str().expect("utf-8 path"),
            "--json",
            "preflight",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout must be utf8");
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("preflight payload");
    assert_eq!(payload["status"], serde_json::json!("incompatible"));
    assert_eq!(payload["running_binary_stale"], serde_json::json!(true));
    assert!(payload["running_binary_version"].is_string());
    assert!(
        payload["errors"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(payload["warnings"].is_array());
}
