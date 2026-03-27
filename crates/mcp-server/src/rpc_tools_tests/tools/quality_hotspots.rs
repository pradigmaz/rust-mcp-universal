use std::fs;

use serde_json::json;

use super::*;

#[test]
fn quality_hotspots_returns_file_buckets_with_risk_scores() {
    let project_dir = temp_dir("rmu-mcp-tests-quality-hotspots-file");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn noisy(flag: bool, level: i32) -> i32 {\n  if flag {\n    if level > 0 {\n      return 1;\n    }\n  }\n  if level < 0 {\n    return -1;\n  }\n  return 0;\n}\n",
    )
    .expect("write rust source");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "quality_hotspots",
            "arguments": {
                "aggregation": "file",
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("quality_hotspots should succeed");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["summary"]["aggregation"],
        json!("file")
    );
    assert!(
        result["structuredContent"]["buckets"][0]["risk_score"]["score"]
            .as_f64()
            .is_some()
    );
    assert!(
        result["structuredContent"]["buckets"][0]["risk_score"]["components"]["complexity"]
            .as_f64()
            .unwrap_or_default()
            > 0.0
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn quality_hotspots_module_mode_falls_back_to_directory_without_zones() {
    let project_dir = temp_dir("rmu-mcp-tests-quality-hotspots-module-fallback");
    fs::create_dir_all(project_dir.join("src/a")).expect("create src/a");
    fs::create_dir_all(project_dir.join("src/b")).expect("create src/b");
    fs::write(project_dir.join("src/a/lib.rs"), "pub fn a() {}\n").expect("write a");
    fs::write(project_dir.join("src/b/lib.rs"), "pub fn b() {}\n").expect("write b");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let directory = handle_tool_call(
        Some(json!({
            "name": "quality_hotspots",
            "arguments": {
                "aggregation": "directory",
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("directory hotspots should succeed");
    let module = handle_tool_call(
        Some(json!({
            "name": "quality_hotspots",
            "arguments": {
                "aggregation": "module"
            }
        })),
        &mut state,
    )
    .expect("module hotspots should succeed");

    let directory_ids = directory["structuredContent"]["buckets"]
        .as_array()
        .expect("directory buckets")
        .iter()
        .map(|bucket| bucket["bucket_id"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    let module_ids = module["structuredContent"]["buckets"]
        .as_array()
        .expect("module buckets")
        .iter()
        .map(|bucket| bucket["bucket_id"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();

    assert_eq!(directory_ids, module_ids);

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn quality_hotspots_returns_degraded_status_for_invalid_quality_policy() {
    let project_dir = temp_dir("rmu-mcp-tests-quality-hotspots-degraded");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn noisy() {\n  let _value = \"this line is intentionally very very very very very very very very very very very very very very very very very very very long\";\n}\n",
    )
    .expect("write rust source");
    fs::write(
        project_dir.join("rmu-quality-policy.json"),
        "{not-valid-json",
    )
    .expect("write invalid policy");

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
            "name": "quality_hotspots",
            "arguments": {
                "aggregation": "file"
            }
        })),
        &mut state,
    )
    .expect("quality_hotspots should degrade instead of failing");

    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["summary"]["status"],
        json!("degraded")
    );

    let _ = fs::remove_dir_all(project_dir);
}
