use std::fs;

use serde_json::json;

use super::*;

#[test]
fn symbol_lookup_v2_finds_decorated_python_async_function() {
    let project_dir = temp_dir("rmu-mcp-tests-symbol-lookup-v2-python-async");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/advanced.py"),
        r#"def traced(fn):
    return fn


@traced
async def decorated_worker():
    return 1
"#,
    )
    .expect("write fixture");

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let result = handle_tool_call(
        Some(json!({
            "name": "symbol_lookup_v2",
            "arguments": {
                "name": "decorated_worker",
                "limit": 5,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("symbol_lookup_v2 should succeed");

    assert_eq!(result["isError"], json!(false));
    let hits = result["structuredContent"]["hits"]
        .as_array()
        .expect("structuredContent.hits should be array");
    assert!(
        hits.iter()
            .any(|hit| hit["path"] == "src/advanced.py" && hit["name"] == "decorated_worker")
    );

    let _ = fs::remove_dir_all(project_dir);
}
