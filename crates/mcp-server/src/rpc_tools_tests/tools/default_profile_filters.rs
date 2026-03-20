use std::fs;

use serde_json::json;

use super::*;

#[test]
fn semantic_index_defaults_to_mixed_and_excludes_docs() {
    let project_dir = temp_dir("rmu-mcp-tests-default-mixed-index");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::create_dir_all(project_dir.join("docs")).expect("create docs");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn default_mixed_symbol() {}\n",
    )
    .expect("write src");
    fs::write(
        project_dir.join("docs/guide.md"),
        "default_mixed_docs_marker\n",
    )
    .expect("write docs");

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
    assert_eq!(
        index_result["structuredContent"]["summary"]["profile"],
        json!("mixed")
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
fn scope_preview_defaults_to_mixed_and_filters_docs() {
    let project_dir = temp_dir("rmu-mcp-tests-default-mixed-scope-preview");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::create_dir_all(project_dir.join("docs")).expect("create docs");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn default_scope_preview_symbol() {}\n",
    )
    .expect("write src");
    fs::write(
        project_dir.join("docs/guide.md"),
        "default_scope_preview_docs_marker\n",
    )
    .expect("write docs");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "scope_preview",
            "arguments": {}
        })),
        &mut state,
    )
    .expect("scope_preview should succeed");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["candidate_paths"],
        json!(["src/lib.rs"])
    );
    assert_eq!(
        result["structuredContent"]["excluded_by_scope_paths"],
        json!(["docs/guide.md"])
    );

    let _ = fs::remove_dir_all(project_dir);
}
