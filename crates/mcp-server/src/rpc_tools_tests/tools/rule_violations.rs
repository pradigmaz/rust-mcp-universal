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
        json!("quality-core-v13")
    );
    assert_eq!(
        result["structuredContent"]["quality_summary"]["status"],
        json!("ready")
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
    assert!(
        result["structuredContent"]["summary"]["violating_files"]
            .as_u64()
            .unwrap_or_default()
            >= 1
    );
    assert_eq!(
        result["structuredContent"]["hits"][0]["path"],
        json!("<masked:app.toml>")
    );
    assert_eq!(
        result["structuredContent"]["hits"][0]["violations"][0]["rule_id"],
        json!("max_non_empty_lines_config")
    );
    assert!(
        result["structuredContent"]["hits"][0]["risk_score"]["score"]
            .as_f64()
            .is_some()
    );
    assert!(
        result["structuredContent"]["hits"][0]["metrics"]
            .as_array()
            .expect("metrics array")
            .iter()
            .any(|metric| metric["metric_id"] == json!("non_empty_lines"))
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn workspace_brief_returns_repair_hint_for_incompatible_index() {
    let project_dir = temp_dir("rmu-mcp-tests-brief-repair-hint");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(project_dir.join("src/lib.rs"), "pub fn broken_meta() {}\n").expect("write file");

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
    conn.execute("DELETE FROM meta WHERE key = 'index_format_version'", [])
        .expect("delete compatibility meta");

    let result = handle_tool_call(
        Some(json!({
            "name": "workspace_brief",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("workspace_brief should return a repair hint instead of failing");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["repair_hint"]["action"],
        json!("reindex")
    );
    assert_eq!(
        result["structuredContent"]["quality_summary"]["status"],
        json!("unavailable")
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
    assert!(
        result["structuredContent"]["summary"]["violating_files"]
            .as_u64()
            .unwrap_or_default()
            >= 1
    );
    assert_eq!(
        result["structuredContent"]["summary"]["status"],
        json!("ready")
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn rule_violations_auto_index_uses_mixed_profile_for_fresh_typescript_repos() {
    let project_dir = temp_dir("rmu-mcp-tests-rule-violations-auto-index-mixed");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::create_dir_all(project_dir.join("docs")).expect("create docs");
    fs::write(
        project_dir.join("src/main.ts"),
        (0..301)
            .map(|idx| format!("export const line_{idx} = {idx};\n"))
            .collect::<String>(),
    )
    .expect("write ts source");
    fs::write(
        project_dir.join("docs/design.md"),
        (0..301)
            .map(|idx| format!("docs_line_{idx}\n"))
            .collect::<String>(),
    )
    .expect("write docs");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "rule_violations",
            "arguments": {
                "auto_index": true,
                "limit": 10
            }
        })),
        &mut state,
    )
    .expect("rule_violations auto_index should succeed on a fresh TS repo");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["summary"]["status"],
        json!("ready")
    );
    assert!(
        result["structuredContent"]["summary"]["violating_files"]
            .as_u64()
            .unwrap_or_default()
            >= 1
    );
    assert_eq!(
        result["structuredContent"]["hits"][0]["path"],
        json!("src/main.ts")
    );

    let status = handle_tool_call(
        Some(json!({
            "name": "index_status",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("index_status should succeed");
    assert_eq!(status["structuredContent"]["files"], json!(1));

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn rule_violations_accept_metric_value_sorting_when_metric_context_is_provided() {
    let project_dir = temp_dir("rmu-mcp-tests-rule-violations-metric-value");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        (0..301)
            .map(|idx| format!("line_{idx}\n"))
            .collect::<String>(),
    )
    .expect("write src");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "rule_violations",
            "arguments": {
                "auto_index": true,
                "sort_by": "metric_value",
                "sort_metric_id": "non_empty_lines"
            }
        })),
        &mut state,
    )
    .expect("rule_violations metric_value sorting should succeed");

    assert_eq!(result["isError"], json!(false));
    let first_hit = &result["structuredContent"]["hits"][0];
    assert_eq!(first_hit["path"], json!("src/lib.rs"));
    assert!(
        first_hit["metrics"]
            .as_array()
            .expect("metrics array")
            .iter()
            .any(|metric| metric["metric_id"] == json!("non_empty_lines"))
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn rule_violations_auto_index_uses_rust_monorepo_for_fresh_rust_workspaces() {
    let project_dir = temp_dir("rmu-mcp-tests-rule-violations-auto-index-rust");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::create_dir_all(project_dir.join("docs")).expect("create docs");
    fs::write(
        project_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("write cargo");
    fs::write(
        project_dir.join("src/lib.rs"),
        (0..301)
            .map(|idx| format!("pub const LINE_{idx}: usize = {idx};\n"))
            .collect::<String>(),
    )
    .expect("write rust source");
    fs::write(
        project_dir.join("docs/design.md"),
        (0..301)
            .map(|idx| format!("docs_line_{idx}\n"))
            .collect::<String>(),
    )
    .expect("write docs");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "rule_violations",
            "arguments": {
                "auto_index": true,
                "limit": 10
            }
        })),
        &mut state,
    )
    .expect("rule_violations auto_index should succeed on a fresh Rust workspace");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["summary"]["status"],
        json!("ready")
    );
    assert!(
        result["structuredContent"]["summary"]["violating_files"]
            .as_u64()
            .unwrap_or_default()
            >= 1
    );
    assert_eq!(
        result["structuredContent"]["hits"][0]["path"],
        json!("src/lib.rs")
    );

    let status = handle_tool_call(
        Some(json!({
            "name": "index_status",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("index_status should succeed");
    assert_eq!(status["structuredContent"]["files"], json!(2));

    let _ = fs::remove_dir_all(project_dir);
}
