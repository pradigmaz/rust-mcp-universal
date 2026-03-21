use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use rmu_core::Engine;
use rusqlite::{Connection, params};

use crate::common::{
    CHILD_PROJECT_ENV, CHILD_ROOT_ENV, CHILD_SHARED_ENV, CHILD_TEST_ENV, cleanup_project,
    temp_project_dir,
};

#[test]
fn default_store_cleanup_runs_on_each_engine_new() -> Result<(), Box<dyn Error>> {
    if std::env::var_os(CHILD_TEST_ENV).is_some() {
        return run_default_store_cleanup_child();
    }

    let root = temp_project_dir("rmu-core-tests-default-store-cleanup");
    let shared_root = root.join("shared-store");
    let project_dir = root.join("project");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(&shared_root)?;
    fs::write(
        project_dir.join("src/main.rs"),
        "fn default_store_symbol() {}\n",
    )?;

    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--exact")
        .arg("default_store_cleanup::default_store_cleanup_runs_on_each_engine_new")
        .arg("--nocapture")
        .env(CHILD_TEST_ENV, "1")
        .env(CHILD_ROOT_ENV, &root)
        .env(CHILD_PROJECT_ENV, &project_dir)
        .env(CHILD_SHARED_ENV, &shared_root)
        .env("RMU_DB_ROOT", &shared_root)
        .env("RMU_DB_TTL_DAYS", "15");

    let output = command.output()?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "child test failed with status {}.\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        )
        .into());
    }

    cleanup_project(&root);
    Ok(())
}

fn run_default_store_cleanup_child() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(std::env::var(CHILD_ROOT_ENV)?);
    let project_dir = PathBuf::from(std::env::var(CHILD_PROJECT_ENV)?);
    let shared_root = PathBuf::from(std::env::var(CHILD_SHARED_ENV)?);

    let _first_engine = Engine::new(project_dir.clone(), None)?;

    let stale_db = shared_root.join("stale.db");
    let stale_conn = Connection::open(&stale_db)?;
    stale_conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;
    stale_conn.execute(
        "INSERT INTO meta(key, value) VALUES('last_access_utc', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["2000-01-01T00:00:00Z"],
    )?;
    drop(stale_conn);

    assert!(stale_db.exists());
    let _second_engine = Engine::new(project_dir, None)?;
    assert!(!stale_db.exists());

    cleanup_project(&root);
    Ok(())
}

#[test]
fn default_store_cleanup_preserves_read_only_heartbeat_databases() -> Result<(), Box<dyn Error>> {
    if std::env::var_os(CHILD_TEST_ENV).is_some() {
        return run_read_only_heartbeat_child();
    }

    let root = temp_project_dir("rmu-core-tests-default-store-heartbeat");
    let shared_root = root.join("shared-store");
    let project_dir = root.join("project");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(&shared_root)?;
    fs::write(
        project_dir.join("src/main.rs"),
        "fn default_store_read_only_heartbeat() {}\n",
    )?;

    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--exact")
        .arg("default_store_cleanup::default_store_cleanup_preserves_read_only_heartbeat_databases")
        .arg("--nocapture")
        .env(CHILD_TEST_ENV, "1")
        .env(CHILD_ROOT_ENV, &root)
        .env(CHILD_PROJECT_ENV, &project_dir)
        .env(CHILD_SHARED_ENV, &shared_root)
        .env("RMU_DB_ROOT", &shared_root)
        .env("RMU_DB_TTL_DAYS", "15");

    let output = command.output()?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "child test failed with status {}.\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        )
        .into());
    }

    cleanup_project(&root);
    Ok(())
}

fn run_read_only_heartbeat_child() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(std::env::var(CHILD_ROOT_ENV)?);
    let project_dir = PathBuf::from(std::env::var(CHILD_PROJECT_ENV)?);
    let shared_root = PathBuf::from(std::env::var(CHILD_SHARED_ENV)?);

    let engine = Engine::new(project_dir.clone(), None)?;
    engine.index_path()?;
    let live_db = engine.db_path.clone();

    let conn = Connection::open(&live_db)?;
    conn.execute(
        "UPDATE meta
         SET value = '2000-01-01T00:00:00Z'
         WHERE key = 'last_access_utc'",
        [],
    )?;
    drop(conn);

    let read_only_engine = Engine::new_read_only(project_dir.clone(), None)?;
    let status = read_only_engine.index_status()?;
    assert!(status.files >= 1);

    let live_conn = Connection::open(&live_db)?;
    let last_access: String = live_conn.query_row(
        "SELECT value FROM meta WHERE key = 'last_access_utc'",
        [],
        |row| row.get(0),
    )?;
    assert_ne!(last_access, "2000-01-01T00:00:00Z");
    drop(live_conn);

    let stale_db = shared_root.join("stale.db");
    let stale_conn = Connection::open(&stale_db)?;
    stale_conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;
    stale_conn.execute(
        "INSERT INTO meta(key, value) VALUES('last_access_utc', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["2000-01-01T00:00:00Z"],
    )?;
    drop(stale_conn);

    let other_project = root.join("other-project");
    fs::create_dir_all(other_project.join("src"))?;
    fs::write(
        other_project.join("src/lib.rs"),
        "fn default_store_cleanup_probe() {}\n",
    )?;
    let _cleanup_engine = Engine::new(other_project, None)?;

    assert!(live_db.exists());
    assert!(!stale_db.exists());

    cleanup_project(&root);
    Ok(())
}
