use std::fs;

use serde_json::json;

use super::*;

#[test]
fn constraint_evidence_tool_returns_canonical_stage5_fields() {
    let project_dir = temp_dir("rmu-mcp-tests-constraint-stage5-fields");
    write_route_and_constraint_fixture(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "constraint_evidence",
            "arguments": {
                "seed": "resolve_lab",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("constraint_evidence should succeed");

    let item = &result["structuredContent"]["items"][0];
    assert!(item["constraint_kind"].is_string());
    assert!(item["source_kind"].is_string());
    assert!(item["path"].is_string());
    assert!(item["line_start"].is_number());
    assert!(item["line_end"].is_number());
    assert!(item["excerpt"].is_string());
    assert!(item["confidence"].is_number());
    assert!(item["normalized_key"].is_string());
    assert!(item["kind"].is_string());
    assert!(item["source_path"].is_string());

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn constraint_evidence_privacy_masks_canonical_stage5_fields() {
    let project_dir = temp_dir("rmu-mcp-tests-constraint-stage5-privacy");
    write_route_and_constraint_fixture(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "constraint_evidence",
            "arguments": {
                "seed": "resolve_lab",
                "seed_kind": "query",
                "limit": 5,
                "auto_index": true,
                "privacy_mode": "mask"
            }
        })),
        &mut state,
    )
    .expect("constraint_evidence should succeed");

    let item = &result["structuredContent"]["items"][0];
    assert!(
        item["path"]
            .as_str()
            .is_some_and(|value| value.starts_with("<masked:"))
    );
    assert_eq!(item["excerpt"], json!("<redacted-content>"));
    assert_eq!(item["normalized_key"], json!("<redacted-content>"));

    let _ = fs::remove_dir_all(project_dir);
}
