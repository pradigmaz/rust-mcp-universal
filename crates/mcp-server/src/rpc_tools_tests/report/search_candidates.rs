use std::fs;

use serde_json::json;

use super::*;

#[test]
fn search_candidates_unicode_fallback_matches_canonical_forms() {
    let project_dir = temp_dir("rmu-mcp-tests-unicode-fallback");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/uni.rs"),
        "pub fn unicode_probe() { let s = \"Cafe\\u{301}\"; println!(\"{s}\"); }\n",
    )
    .expect("write file");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "search_candidates",
            "arguments": {
                "query": "\u{00C9}",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("search_candidates should succeed");

    assert_eq!(result["isError"], json!(false));
    let hits = result["structuredContent"]["hits"]
        .as_array()
        .expect("hits should be array");
    assert!(
        hits.iter()
            .filter_map(|hit| hit["path"].as_str())
            .any(|path| path.ends_with("src/uni.rs") || path == "src/uni.rs")
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn search_candidates_respects_semantic_fail_mode_on_corrupted_vectors() {
    let project_dir = temp_dir("rmu-mcp-tests-semantic-fail-mode");
    fs::create_dir_all(project_dir.join("src")).expect("create temp dir");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn semantic_fail_mode_probe() -> i32 { 7 }\n",
    )
    .expect("write file");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let _indexed = handle_tool_call(
        Some(json!({
            "name": "index",
            "arguments": { "reindex": true }
        })),
        &mut state,
    )
    .expect("index should succeed");

    let conn = rusqlite::Connection::open(project_dir.join(".rmu/index.db"))
        .expect("open sqlite for corruption");
    conn.execute(
        "UPDATE semantic_vectors
         SET vector_json = '[1,2,3]'
         WHERE rowid = (SELECT rowid FROM semantic_vectors LIMIT 1)",
        [],
    )
    .expect("corrupt semantic vector payload");

    let fail_open = handle_tool_call(
        Some(json!({
            "name": "search_candidates",
            "arguments": {
                "query": "semantic_fail_mode_probe",
                "limit": 5,
                "semantic": true,
                "semantic_fail_mode": "fail_open"
            }
        })),
        &mut state,
    )
    .expect("fail_open query should degrade to lexical");
    assert_eq!(fail_open["isError"], json!(false));
    assert!(
        fail_open["structuredContent"]["hits"]
            .as_array()
            .is_some_and(|hits| !hits.is_empty())
    );

    let err = handle_tool_call(
        Some(json!({
            "name": "search_candidates",
            "arguments": {
                "query": "semantic_fail_mode_probe",
                "limit": 5,
                "semantic": true,
                "semantic_fail_mode": "fail_closed"
            }
        })),
        &mut state,
    )
    .expect_err("fail_closed should propagate semantic runtime error");
    assert!(err.to_string().contains("invalid semantic vector"));

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn search_candidates_auto_index_uses_mixed_profile_and_skips_docs() {
    let project_dir = temp_dir("rmu-mcp-tests-search-auto-index-mixed");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::create_dir_all(project_dir.join("docs")).expect("create docs");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn code_only_auto_index_symbol() {}\n",
    )
    .expect("write src");
    fs::write(
        project_dir.join("docs/design.md"),
        "docs_only_auto_index_marker\n",
    )
    .expect("write docs");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "search_candidates",
            "arguments": {
                "query": "docs_only_auto_index_marker",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("search_candidates should succeed");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(result["structuredContent"]["hits"], json!([]));

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
