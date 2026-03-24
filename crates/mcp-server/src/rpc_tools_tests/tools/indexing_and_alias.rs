use std::fs;
use std::process::Command;

use serde_json::json;
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use super::*;

#[test]
fn index_status_returns_flat_structured_content() {
    let project_dir = temp_dir("rmu-mcp-tests");
    fs::create_dir_all(&project_dir).expect("create temp dir");
    let db_path = project_dir.join(".rmu/index.db");

    let mut state = state_for(project_dir.clone(), Some(db_path.clone()));

    let result = handle_tool_call(
        Some(json!({
            "name": "index_status",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("index_status should succeed");

    assert_eq!(result["isError"], json!(false));
    assert!(result["structuredContent"]["files"].is_number());
    assert!(result["structuredContent"]["status"].is_null());
    assert!(!db_path.exists());
    assert!(!project_dir.join(".rmu").exists());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn workspace_brief_returns_snapshot() {
    let project_dir = temp_dir("rmu-mcp-tests-brief");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn sample_symbol_name() -> i32 { 1 }\n",
    )
    .expect("write file");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let index_result = handle_tool_call(
        Some(json!({
            "name": "semantic_index",
            "arguments": {
                "reindex": true
            }
        })),
        &mut state,
    )
    .expect("semantic_index should succeed");
    assert_eq!(index_result["isError"], json!(false));

    let result = handle_tool_call(
        Some(json!({
            "name": "workspace_brief",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("workspace_brief should succeed");

    assert_eq!(result["isError"], json!(false));
    assert!(result["structuredContent"]["index_status"]["files"].is_number());
    assert!(result["structuredContent"]["languages"].is_array());
    let recommendations = result["structuredContent"]["recommendations"]
        .as_array()
        .expect("recommendations should be array");
    for recommendation in recommendations {
        let recommendation = recommendation
            .as_str()
            .expect("recommendation should be string");
        assert!(!recommendation.contains("semantic-search"));
        assert!(!recommendation.contains("report --semantic"));
        assert!(!recommendation.contains("semantic-index"));
    }

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn workspace_brief_requires_index_and_does_not_create_db() {
    let project_dir = temp_dir("rmu-mcp-tests-brief-read-only");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn sample_symbol_name() -> i32 { 1 }\n",
    )
    .expect("write file");
    let db_path = project_dir.join(".rmu/index.db");
    let mut state = state_for(project_dir.clone(), Some(db_path.clone()));

    let err = handle_tool_call(
        Some(json!({
            "name": "workspace_brief",
            "arguments": {}
        })),
        &mut state,
    )
    .expect_err("workspace_brief should require an existing index");

    assert!(err.to_string().contains("index is empty"));
    assert!(!db_path.exists());
    assert!(!project_dir.join(".rmu").exists());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn semantic_index_supports_scope_selection() {
    let project_dir = temp_dir("rmu-mcp-tests-scope-index");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::create_dir_all(project_dir.join("vendor")).expect("create vendor");
    fs::create_dir_all(project_dir.join("scripts")).expect("create scripts");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn wanted_symbol() {}\n",
    )
    .expect("write src");
    fs::write(
        project_dir.join("vendor/tmp.rs"),
        "pub fn ignored_symbol() {}\n",
    )
    .expect("write vendor");
    fs::write(project_dir.join("scripts/tool.py"), "print('ignored')\n").expect("write script");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let index_result = handle_tool_call(
        Some(json!({
            "name": "semantic_index",
            "arguments": {
                "include_paths": ["**/*.rs"],
                "exclude_paths": ["vendor/**"],
                "reindex": true
            }
        })),
        &mut state,
    )
    .expect("semantic_index should succeed");

    assert_eq!(index_result["isError"], json!(false));
    assert!(index_result["structuredContent"]["summary"]["indexed"].is_number());

    let status_result = handle_tool_call(
        Some(json!({
            "name": "index_status",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("index_status should succeed");

    assert_eq!(status_result["structuredContent"]["files"], json!(1));
    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn scope_preview_reports_scope_buckets_without_initializing_db() {
    let project_dir = temp_dir("rmu-mcp-tests-scope-preview");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::create_dir_all(project_dir.join("vendor")).expect("create vendor");
    fs::create_dir_all(project_dir.join("target")).expect("create target");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn preview_scope_symbol() {}\n",
    )
    .expect("write src");
    fs::write(
        project_dir.join("vendor/tmp.rs"),
        "pub fn preview_scope_excluded() {}\n",
    )
    .expect("write vendor");
    fs::write(
        project_dir.join("target/generated.rs"),
        "pub fn preview_scope_ignored() {}\n",
    )
    .expect("write target");

    let db_path = project_dir.join(".rmu/index.db");
    let mut state = state_for(project_dir.clone(), Some(db_path.clone()));

    let result = handle_tool_call(
        Some(json!({
            "name": "scope_preview",
            "arguments": {
                "include_paths": ["**/*.rs"],
                "exclude_paths": ["vendor/**"]
            }
        })),
        &mut state,
    )
    .expect("scope_preview should succeed");

    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert_eq!(structured["candidate_paths"], json!(["src/lib.rs"]));
    assert_eq!(
        structured["excluded_by_scope_paths"],
        json!(["vendor/tmp.rs"])
    );
    assert_eq!(structured["ignored_paths"], json!(["target/generated.rs"]));
    assert!(!db_path.exists());
    assert!(!project_dir.join(".rmu").exists());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn index_alias_matches_semantic_index_behavior() {
    let project_dir = temp_dir("rmu-mcp-tests-index-alias");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(project_dir.join("src/lib.rs"), "pub fn alias_symbol() {}\n").expect("write src");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let index_result = handle_tool_call(
        Some(json!({
            "name": "index",
            "arguments": {
                "reindex": true
            }
        })),
        &mut state,
    )
    .expect("index alias should succeed");

    assert_eq!(index_result["isError"], json!(false));
    assert!(index_result["structuredContent"]["summary"]["indexed"].is_number());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn semantic_index_supports_profile_selection_and_manual_narrowing() {
    let project_dir = temp_dir("rmu-mcp-tests-profile-index");
    fs::create_dir_all(project_dir.join("crates/core/src")).expect("create crates");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::create_dir_all(project_dir.join("docs")).expect("create docs");
    fs::write(
        project_dir.join("crates/core/src/lib.rs"),
        "pub fn wanted_profile_symbol() {}\n",
    )
    .expect("write crate");
    fs::write(
        project_dir.join("src/main.rs"),
        "fn root_profile_symbol() {}\n",
    )
    .expect("write src");
    fs::write(
        project_dir.join("docs/guide.md"),
        "profile_should_not_index\n",
    )
    .expect("write docs");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let index_result = handle_tool_call(
        Some(json!({
            "name": "semantic_index",
            "arguments": {
                "profile": "rust-monorepo",
                "include_paths": ["crates"],
                "reindex": true
            }
        })),
        &mut state,
    )
    .expect("semantic_index with profile should succeed");

    assert_eq!(
        index_result["structuredContent"]["summary"]["profile"],
        json!("rust-monorepo")
    );
    assert_eq!(
        index_result["structuredContent"]["summary"]["indexed"],
        json!(1)
    );

    let status_result = handle_tool_call(
        Some(json!({
            "name": "index_status",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("index_status should succeed");

    assert_eq!(status_result["structuredContent"]["files"], json!(1));
    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn scope_preview_hash_privacy_mode_sanitizes_path_lists() {
    let project_dir = temp_dir("rmu-mcp-tests-scope-preview-privacy");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn preview_privacy_symbol() {}\n",
    )
    .expect("write src");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "scope_preview",
            "arguments": {
                "privacy_mode": "hash"
            }
        })),
        &mut state,
    )
    .expect("scope_preview should succeed");

    assert_eq!(result["isError"], json!(false));
    let candidate = result["structuredContent"]["candidate_paths"][0]
        .as_str()
        .expect("candidate path should be string");
    assert!(candidate.starts_with("<hash:"));
    assert!(candidate.ends_with('>'));
    assert!(!candidate.contains("src/lib.rs"));

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn delete_index_removes_database_files() {
    let project_dir = temp_dir("rmu-mcp-tests-delete-index");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn delete_index_symbol() {}\n",
    )
    .expect("write file");

    let db_path = project_dir.join(".rmu/index.db");
    let mut state = state_for(project_dir.clone(), Some(db_path.clone()));

    let _ = handle_tool_call(
        Some(json!({
            "name": "semantic_index",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("semantic_index should succeed");

    assert!(db_path.exists());

    let result = handle_tool_call(
        Some(json!({
            "name": "delete_index",
            "arguments": {"confirm": true}
        })),
        &mut state,
    )
    .expect("delete_index should succeed");

    assert_eq!(result["isError"], json!(false));
    assert!(result["structuredContent"]["removed_count"].is_number());
    assert!(!db_path.exists());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn semantic_index_reports_changed_since_and_skip_count() {
    let project_dir = temp_dir("rmu-mcp-tests-changed-since");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/old.rs"),
        "pub fn old_mcp_symbol() {}\n",
    )
    .expect("write old");
    fs::write(
        project_dir.join("src/fresh.rs"),
        "pub fn fresh_mcp_symbol() {}\n",
    )
    .expect("write fresh");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let _ = handle_tool_call(
        Some(json!({
            "name": "semantic_index",
            "arguments": {
                "reindex": true
            }
        })),
        &mut state,
    )
    .expect("semantic_index should succeed");

    std::thread::sleep(std::time::Duration::from_millis(1200));
    let cutoff = OffsetDateTime::now_utc()
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .expect("format cutoff");
    std::thread::sleep(std::time::Duration::from_millis(1200));
    fs::write(
        project_dir.join("src/fresh.rs"),
        "pub fn fresh_mcp_symbol() { println!(\"updated\"); }\n",
    )
    .expect("rewrite fresh");

    let result = handle_tool_call(
        Some(json!({
            "name": "semantic_index",
            "arguments": {
                "changed_since": cutoff,
                "reindex": false
            }
        })),
        &mut state,
    )
    .expect("semantic_index with changed_since should succeed");

    assert_eq!(
        result["structuredContent"]["summary"]["changed_since"],
        json!(cutoff)
    );
    assert_eq!(
        result["structuredContent"]["summary"]["skipped_before_changed_since"],
        json!(1)
    );
    assert_eq!(result["structuredContent"]["summary"]["indexed"], json!(1));

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn semantic_index_reports_changed_since_commit_summary_fields() {
    let project_dir = temp_dir("rmu-mcp-tests-changed-since-commit");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn commit_index_symbol() {}\n",
    )
    .expect("write lib");

    run_git(&project_dir, &["init"]);
    run_git(&project_dir, &["config", "user.email", "codex@example.com"]);
    run_git(&project_dir, &["config", "user.name", "Codex"]);
    run_git(&project_dir, &["add", "."]);
    run_git(&project_dir, &["commit", "-m", "initial"]);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let _ = handle_tool_call(
        Some(json!({
            "name": "semantic_index",
            "arguments": {
                "reindex": true
            }
        })),
        &mut state,
    )
    .expect("semantic_index should succeed");

    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn commit_index_symbol() { println!(\"changed\"); }\n",
    )
    .expect("rewrite lib");

    let result = handle_tool_call(
        Some(json!({
            "name": "semantic_index",
            "arguments": {
                "changed_since_commit": "HEAD"
            }
        })),
        &mut state,
    )
    .expect("semantic_index with changed_since_commit should succeed");

    assert_eq!(
        result["structuredContent"]["summary"]["changed_since_commit"],
        json!("HEAD")
    );
    assert!(
        result["structuredContent"]["summary"]["resolved_merge_base_commit"]
            .as_str()
            .is_some()
    );
    assert_eq!(result["structuredContent"]["summary"]["indexed"], json!(1));

    let _ = fs::remove_dir_all(project_dir);
}

fn run_git(project_dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(project_dir)
        .args(args)
        .status()
        .expect("git should be available");
    assert!(status.success(), "git {:?} failed", args);
}
