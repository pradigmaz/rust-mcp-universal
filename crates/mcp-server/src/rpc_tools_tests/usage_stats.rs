use rusqlite::Connection;
use serde_json::json;

use super::{handle_tool_call, state_for, temp_dir};

#[test]
fn tool_calls_persist_usage_stats() {
    let project_dir = temp_dir("rmu-mcp-usage-stats");
    std::fs::create_dir_all(project_dir.join(".rmu")).expect("create db parent");
    let db_path = project_dir.join(".rmu/index.db");
    Connection::open(&db_path).expect("create usage db");
    let mut state = state_for(project_dir.clone(), Some(db_path.clone()));

    handle_tool_call(
        Some(json!({
            "name": "set_project_path",
            "arguments": {"project_path": project_dir}
        })),
        &mut state,
    )
    .expect("set project path");

    let conn = Connection::open(db_path).expect("open usage db");
    let (tool, ok, response_bytes): (String, i64, i64) = conn
        .query_row(
            "SELECT tool, ok, response_bytes FROM mcp_tool_usage",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("usage row");

    assert_eq!(tool, "set_project_path");
    assert_eq!(ok, 1);
    assert!(response_bytes > 0);

    let stats = handle_tool_call(
        Some(json!({
            "name": "usage_stats",
            "arguments": {"limit": 1}
        })),
        &mut state,
    )
    .expect("usage stats");

    assert_eq!(stats["structuredContent"]["summary"]["calls"], json!(1));
    assert_eq!(
        stats["structuredContent"]["recent"][0]["tool"],
        json!("set_project_path")
    );
}

#[test]
fn tool_calls_persist_usage_stats_with_default_db_path() {
    let project_dir = temp_dir("rmu-mcp-usage-stats-default-db");
    std::fs::create_dir_all(project_dir.join(".rmu")).expect("create db parent");
    let db_path = project_dir.join(".rmu/index.db");
    Connection::open(&db_path).expect("create usage db");
    let mut state = state_for(project_dir.clone(), None);

    handle_tool_call(
        Some(json!({
            "name": "set_project_path",
            "arguments": {"project_path": project_dir}
        })),
        &mut state,
    )
    .expect("set project path");

    let stats = handle_tool_call(
        Some(json!({
            "name": "usage_stats",
            "arguments": {"limit": 1}
        })),
        &mut state,
    )
    .expect("usage stats");

    assert_eq!(stats["structuredContent"]["summary"]["calls"], json!(1));
    assert_eq!(
        stats["structuredContent"]["recent"][0]["tool"],
        json!("set_project_path")
    );
}
