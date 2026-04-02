use std::fs;

use serde_json::json;

use super::*;

fn write_hot_file(project_dir: &std::path::Path) {
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        (0..320)
            .map(|idx| format!("pub const LINE_{idx}: &str = \"value_{idx}\";\n"))
            .collect::<String>(),
    )
    .expect("write source");
}

#[test]
fn quality_snapshot_returns_structured_snapshot_and_artifacts() {
    let project_dir = temp_dir("rmu-mcp-tests-quality-snapshot");
    write_hot_file(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "quality_snapshot",
            "arguments": {
                "snapshot_kind": "before",
                "wave_id": "wave-1",
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("quality_snapshot should succeed");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["snapshot"]["snapshot_kind"],
        json!("before")
    );
    assert_eq!(
        result["structuredContent"]["snapshot"]["quality_status_after_refresh"],
        json!("ready")
    );
    assert!(
        result["structuredContent"]["artifacts"]["snapshot_root"]
            .as_str()
            .is_some_and(|path| path.contains(".codex"))
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn quality_snapshot_fail_on_regression_blocks_new_violations() {
    let project_dir = temp_dir("rmu-mcp-tests-quality-snapshot-regression");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(project_dir.join("src/lib.rs"), "pub fn ok() {}\n").expect("write initial source");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    handle_tool_call(
        Some(json!({
            "name": "quality_snapshot",
            "arguments": {
                "snapshot_kind": "before",
                "wave_id": "wave-2",
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("before snapshot should succeed");

    fs::write(
        project_dir.join("src/lib.rs"),
        (0..320)
            .map(|idx| format!("pub const LINE_{idx}: &str = \"value_{idx}\";\n"))
            .collect::<String>(),
    )
    .expect("rewrite source with violation");

    let err = handle_tool_call(
        Some(json!({
            "name": "quality_snapshot",
            "arguments": {
                "snapshot_kind": "after",
                "wave_id": "wave-2",
                "compare_against": "wave_before",
                "auto_index": true,
                "fail_on_regression": true
            }
        })),
        &mut state,
    )
    .expect_err("regression gate should fail");

    assert!(err.to_string().contains("regression gate failed"));

    let _ = fs::remove_dir_all(project_dir);
}
