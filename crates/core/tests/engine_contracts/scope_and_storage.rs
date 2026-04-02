use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration as StdDuration;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use rmu_core::{
    Engine, IndexProfile, IndexingOptions, MigrationMode, PrivacyMode, QueryOptions,
    SemanticFailMode,
};
use rusqlite::Connection;
use time::{Duration, OffsetDateTime};

use crate::common::{cleanup_project, temp_project_dir};

#[path = "scope_and_storage/changed_since.rs"]
mod changed_since;
#[path = "scope_and_storage/changed_since_commit.rs"]
mod changed_since_commit;
#[path = "scope_and_storage/platform_and_graph_repairs.rs"]
mod platform_and_graph_repairs;
#[path = "scope_and_storage/profiles.rs"]
mod profiles;
#[path = "scope_and_storage/repair_preservation.rs"]
mod repair_preservation;
#[path = "scope_and_storage/scope_basics.rs"]
mod scope_basics;

fn source_mtime_for_path(db_path: &PathBuf, path: &str) -> Result<Option<i64>, Box<dyn Error>> {
    let conn = Connection::open(db_path)?;
    let value = conn.query_row(
        "SELECT source_mtime_unix_ms FROM files WHERE path = ?1",
        [path],
        |row| row.get::<_, Option<i64>>(0),
    )?;
    Ok(value)
}

fn run_git(project_dir: &std::path::Path, args: &[&str]) -> Result<(), Box<dyn Error>> {
    let status = Command::new("git")
        .current_dir(project_dir)
        .args(args)
        .status()?;
    assert!(status.success(), "git {:?} failed", args);
    Ok(())
}

fn file_graph_edge_count(db_path: &PathBuf) -> Result<i64, Box<dyn Error>> {
    let conn = Connection::open(db_path)?;
    let count = conn.query_row("SELECT COUNT(*) FROM file_graph_edges", [], |row| {
        row.get(0)
    })?;
    Ok(count)
}

#[cfg(unix)]
fn file_row_count(db_path: &PathBuf, path: &str) -> Result<i64, Box<dyn Error>> {
    let conn = Connection::open(db_path)?;
    let count = conn.query_row(
        "SELECT COUNT(1) FROM files WHERE path = ?1",
        [path],
        |row| row.get(0),
    )?;
    Ok(count)
}

#[cfg(unix)]
fn search_hit_count(engine: &Engine, query: &str) -> Result<usize, Box<dyn Error>> {
    let hits = engine.search(&QueryOptions {
        query: query.to_string(),
        limit: 10,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
        agent_intent_mode: None,
    })?;
    Ok(hits.len())
}

fn graph_edge_metadata_present(db_path: &PathBuf, path: &str) -> Result<bool, Box<dyn Error>> {
    let conn = Connection::open(db_path)?;
    let metadata = conn.query_row(
        "SELECT graph_edge_out_count, graph_edge_in_count, graph_edge_hash, graph_edge_fingerprint_version
         FROM files
         WHERE path = ?1",
        [path],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<i64>>(3)?,
            ))
        },
    )?;
    Ok(
        metadata.0.is_some()
            && metadata.1.is_some()
            && metadata.2.is_some()
            && metadata.3.is_some(),
    )
}
