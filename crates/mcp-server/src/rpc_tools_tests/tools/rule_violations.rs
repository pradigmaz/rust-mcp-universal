use std::fs;

use serde_json::json;

use super::*;

#[test]
fn workspace_brief_includes_quality_summary() {
    let project_dir = temp_dir("rmu-mcp-tests-brief-quality");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/lib.rs"),
        (0..301)
            .map(|idx| format!("line_{idx}\n"))
            .collect::<String>(),
    )
    .expect("write file");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    handle_tool_call(
        Some(json!({
            "name": "index",
            "arguments": { "reindex": true }
        })),
        &mut state,
    )
    .expect("index should succeed");

    let result = handle_tool_call(
        Some(json!({
            "name": "workspace_brief",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("workspace_brief should succeed");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["quality_summary"]["ruleset_id"],
        json!("file-size-v1")
    );
    assert_eq!(
        result["structuredContent"]["quality_summary"]["violating_files"],
        json!(1)
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn rule_violations_returns_filtered_hits_and_masks_paths() {
    let project_dir = temp_dir("rmu-mcp-tests-rule-violations");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::create_dir_all(project_dir.join("config")).expect("create config");
    fs::write(
        project_dir.join("src/lib.rs"),
        (0..301)
            .map(|idx| format!("line_{idx}\n"))
            .collect::<String>(),
    )
    .expect("write src");
    fs::write(
        project_dir.join("config/app.toml"),
        (0..101)
            .map(|idx| format!("key_{idx} = true\n"))
            .collect::<String>(),
    )
    .expect("write config");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    handle_tool_call(
        Some(json!({
            "name": "index",
            "arguments": { "reindex": true }
        })),
        &mut state,
    )
    .expect("index should succeed");

    let result = handle_tool_call(
        Some(json!({
            "name": "rule_violations",
            "arguments": {
                "path_prefix": "config/",
                "rule_ids": ["max_non_empty_lines_config"],
                "sort_by": "non_empty_lines",
                "privacy_mode": "mask"
            }
        })),
        &mut state,
    )
    .expect("rule_violations should succeed");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["summary"]["violating_files"],
        json!(1)
    );
    assert_eq!(
        result["structuredContent"]["hits"][0]["path"],
        json!("<masked:app.toml>")
    );
    assert_eq!(
        result["structuredContent"]["hits"][0]["violations"][0]["rule_id"],
        json!("max_non_empty_lines_config")
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn rule_violations_auto_index_backfills_missing_quality_rows() {
    let project_dir = temp_dir("rmu-mcp-tests-rule-violations-auto-index");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        (0..301)
            .map(|idx| format!("line_{idx}\n"))
            .collect::<String>(),
    )
    .expect("write src");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    handle_tool_call(
        Some(json!({
            "name": "index",
            "arguments": { "reindex": true }
        })),
        &mut state,
    )
    .expect("index should succeed");

    let conn = rusqlite::Connection::open(project_dir.join(".rmu/index.db")).expect("open db");
    conn.execute("DELETE FROM file_rule_violations", [])
        .expect("delete quality detail");
    conn.execute("DELETE FROM file_quality", [])
        .expect("delete quality rows");

    let result = handle_tool_call(
        Some(json!({
            "name": "rule_violations",
            "arguments": {
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("rule_violations auto_index should backfill quality rows");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["summary"]["violating_files"],
        json!(1)
    );

    let _ = fs::remove_dir_all(project_dir);
}
