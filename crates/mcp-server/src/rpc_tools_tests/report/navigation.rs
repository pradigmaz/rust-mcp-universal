use std::fs;

use serde_json::{Value, json};

use super::*;

fn expect_navigation_v2_hits(result: &Value) -> &[Value] {
    result["structuredContent"]["hits"]
        .as_array()
        .expect("structuredContent.hits should be array")
}

#[test]
fn symbol_lookup_returns_matches_with_auto_index() {
    let project_dir = temp_dir("rmu-mcp-tests-symbol-lookup");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/main.rs"),
        "fn lookup_symbol_target() {}\n",
    )
    .expect("write fixture");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "symbol_lookup",
            "arguments": {
                "name": "lookup_symbol_target",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("symbol_lookup should succeed");

    assert_eq!(result["isError"], json!(false));
    let hits = result["structuredContent"]
        .as_array()
        .expect("structuredContent should be array");
    assert!(!hits.is_empty());
    assert!(hits[0]["path"].is_string());
    assert!(hits[0]["line"].is_number());
    assert!(hits[0]["column"].is_number());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn symbol_lookup_v2_returns_hits_object_with_auto_index() {
    let project_dir = temp_dir("rmu-mcp-tests-symbol-lookup-v2");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/main.rs"),
        "fn lookup_symbol_target_v2() {}\n",
    )
    .expect("write fixture");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "symbol_lookup_v2",
            "arguments": {
                "name": "lookup_symbol_target_v2",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("symbol_lookup_v2 should succeed");

    assert_eq!(result["isError"], json!(false));
    let hits = expect_navigation_v2_hits(&result);
    assert!(!hits.is_empty());
    assert!(hits[0]["path"].is_string());
    assert!(hits[0]["line"].is_number());
    assert!(hits[0]["column"].is_number());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn symbol_references_returns_grouped_hits() {
    let project_dir = temp_dir("rmu-mcp-tests-symbol-references");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        r#"
fn reference_target() {}
mod caller {
    fn call() {
        crate::reference_target();
    }
}
"#,
    )
    .expect("write fixture");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "symbol_references",
            "arguments": {
                "name": "reference_target",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("symbol_references should succeed");

    assert_eq!(result["isError"], json!(false));
    let hits = result["structuredContent"]
        .as_array()
        .expect("structuredContent should be array");
    assert!(!hits.is_empty());
    assert!(hits[0]["ref_count"].is_number());
    assert!(hits[0]["line"].is_number());
    assert!(hits[0]["column"].is_number());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn symbol_references_v2_returns_grouped_hits_object() {
    let project_dir = temp_dir("rmu-mcp-tests-symbol-references-v2");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        r#"
fn reference_target_v2() {}
mod caller {
    fn call() {
        crate::reference_target_v2();
    }
}
"#,
    )
    .expect("write fixture");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "symbol_references_v2",
            "arguments": {
                "name": "reference_target_v2",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("symbol_references_v2 should succeed");

    assert_eq!(result["isError"], json!(false));
    let hits = expect_navigation_v2_hits(&result);
    assert!(!hits.is_empty());
    assert!(hits[0]["ref_count"].is_number());
    assert!(hits[0]["line"].is_number());
    assert!(hits[0]["column"].is_number());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn symbol_references_returns_type_and_struct_literal_usages() {
    let project_dir = temp_dir("rmu-mcp-tests-symbol-references-types");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        r#"
pub struct GraphRef {
    value: usize,
}

pub struct Holder {
    inner: GraphRef,
}

impl GraphRef {
    pub fn from_value(value: usize) -> Self {
        GraphRef { value }
    }
}

fn mirror(input: &GraphRef) -> GraphRef {
    GraphRef { value: input.value }
}
"#,
    )
    .expect("write fixture");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "symbol_references",
            "arguments": {
                "name": "GraphRef",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("symbol_references should succeed");

    assert_eq!(result["isError"], json!(false));
    let hits = result["structuredContent"]
        .as_array()
        .expect("structuredContent should be array");
    assert!(hits.iter().any(|hit| {
        hit["path"] == "src/lib.rs"
            && hit["exact"] == json!(true)
            && hit["ref_count"].as_u64().is_some_and(|count| count >= 5)
            && hit["line"].is_number()
            && hit["column"].is_number()
    }));

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn related_files_returns_connected_neighbors() {
    let project_dir = temp_dir("rmu-mcp-tests-related-files");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/main.rs"),
        r#"
use crate::shared::helper;
fn root() {
    helper();
}
"#,
    )
    .expect("write main");
    fs::write(
        project_dir.join("src/shared.rs"),
        r#"
fn helper() {}
"#,
    )
    .expect("write shared");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "related_files",
            "arguments": {
                "path": "src/main.rs",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("related_files should succeed");

    assert_eq!(result["isError"], json!(false));
    let hits = result["structuredContent"]
        .as_array()
        .expect("structuredContent should be array");
    assert!(hits.iter().any(|hit| hit["path"] == "src/shared.rs"));

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn related_files_v2_returns_hits_object() {
    let project_dir = temp_dir("rmu-mcp-tests-related-files-v2");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/main.rs"),
        r#"
use crate::shared::helper;
fn root_v2() {
    helper();
}
"#,
    )
    .expect("write main");
    fs::write(
        project_dir.join("src/shared.rs"),
        r#"
fn helper() {}
"#,
    )
    .expect("write shared");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "related_files_v2",
            "arguments": {
                "path": "src/main.rs",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("related_files_v2 should succeed");

    assert_eq!(result["isError"], json!(false));
    let hits = expect_navigation_v2_hits(&result);
    assert!(hits.iter().any(|hit| hit["path"] == "src/shared.rs"));

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn call_path_returns_path_with_evidence() {
    let project_dir = temp_dir("rmu-mcp-tests-call-path");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/main.rs"),
        "mod shared;\nmod util;\nfn main() { shared::helper(); }\n",
    )
    .expect("write main");
    fs::write(
        project_dir.join("src/shared.rs"),
        "pub fn helper() { crate::util::support(); }\n",
    )
    .expect("write shared");
    fs::write(project_dir.join("src/util.rs"), "pub fn support() {}\n").expect("write util");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "call_path",
            "arguments": {
                "from": "src/main.rs",
                "to": "support",
                "max_hops": 6,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("call_path should succeed");

    assert_eq!(result["isError"], json!(false));
    let payload = &result["structuredContent"];
    assert_eq!(payload["found"], json!(true));
    assert_eq!(payload["hops"], json!(2));
    assert_eq!(payload["path"][0], "src/main.rs");
    assert_eq!(payload["path"][2], "src/util.rs");
    assert_eq!(payload["steps"][0]["edge_kind"], json!("ref_tail_unique"));
    assert!(
        payload["steps"][0]["evidence"]
            .as_str()
            .is_some_and(|value| value.contains("helper"))
    );
    assert!(
        payload["steps"][1]["evidence"]
            .as_str()
            .is_some_and(|value| value.contains("support"))
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn context_pack_design_mode_returns_docs_first() {
    let project_dir = temp_dir("rmu-mcp-tests-context-pack");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::create_dir_all(project_dir.join("docs")).expect("create docs");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn architecture_runtime() {}\n",
    )
    .expect("write source");
    fs::write(
        project_dir.join("docs/design.md"),
        "Architecture overview and design decisions.\n",
    )
    .expect("write docs");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let indexed = handle_tool_call(
        Some(json!({
            "name": "semantic_index",
            "arguments": {
                "profile": "docs-heavy",
                "reindex": true
            }
        })),
        &mut state,
    )
    .expect("semantic_index should succeed");
    assert_eq!(indexed["isError"], json!(false));

    let result = handle_tool_call(
        Some(json!({
            "name": "context_pack",
            "arguments": {
                "query": "architecture",
                "mode": "design",
                "limit": 10,
                "auto_index": false
            }
        })),
        &mut state,
    )
    .expect("context_pack should succeed");

    assert_eq!(result["isError"], json!(false));
    let payload = &result["structuredContent"];
    assert_eq!(payload["mode"], json!("design"));
    assert_eq!(
        payload["context"]["files"][0]["path"],
        json!("docs/design.md")
    );

    let _ = fs::remove_dir_all(project_dir);
}
