use serde_json::json;

use super::bootstrap_broad_shared::*;
use super::*;

#[test]
fn agent_bootstrap_broad_query_surfaces_followups_in_mcp_payload() {
    let project_dir = temp_dir("rmu-mcp-tests-bootstrap-broad");
    write_bootstrap_broad_fixture(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "agent_bootstrap",
            "arguments": {
                "query": "frontend backend auth entrypoints tests architecture",
                "limit": 5,
                "semantic": true,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("agent_bootstrap broad query should succeed");

    assert_eq!(result["isError"], json!(false));
    let bundle = &result["structuredContent"]["query_bundle"];
    let followups = bundle["followups"]
        .as_array()
        .expect("followups should be present for broad query");
    assert!(!followups.is_empty());

    let top_paths = bundle["hits"]
        .as_array()
        .expect("hits should be array")
        .iter()
        .take(4)
        .filter_map(|hit| hit["path"].as_str())
        .collect::<Vec<_>>();
    assert!(top_paths.iter().any(|path| path.starts_with("backend/")));
    assert!(top_paths.iter().any(|path| path.starts_with("frontend/")));

    let _ = std::fs::remove_dir_all(project_dir);
}

#[test]
fn agent_bootstrap_auth_tests_query_keeps_test_surface_visible_in_mcp_payload() {
    let project_dir = temp_dir("rmu-mcp-tests-bootstrap-auth-tests");
    write_bootstrap_broad_fixture(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "agent_bootstrap",
            "arguments": {
                "query": "auth boundary tests nearby backend frontend",
                "limit": 6,
                "semantic": true,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("agent_bootstrap auth/tests query should succeed");

    assert_eq!(result["isError"], json!(false));
    let top_paths = result["structuredContent"]["query_bundle"]["hits"]
        .as_array()
        .expect("hits should be array")
        .iter()
        .take(5)
        .filter_map(|hit| hit["path"].as_str())
        .collect::<Vec<_>>();
    assert!(top_paths.iter().any(|path| path.starts_with("tests/")));

    let _ = std::fs::remove_dir_all(project_dir);
}

#[test]
fn agent_bootstrap_service_query_keeps_api_and_domain_service_layers_visible_in_mcp_payload() {
    let project_dir = temp_dir("rmu-mcp-tests-bootstrap-service-broad");
    write_bootstrap_broad_fixture(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "agent_bootstrap",
            "arguments": {
                "query": "orchestration domain rules api service layer",
                "limit": 6,
                "semantic": true,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("agent_bootstrap service query should succeed");

    assert_eq!(result["isError"], json!(false));
    let top_paths = result["structuredContent"]["query_bundle"]["hits"]
        .as_array()
        .expect("hits should be array")
        .iter()
        .take(5)
        .filter_map(|hit| hit["path"].as_str())
        .collect::<Vec<_>>();
    assert!(top_paths.iter().any(|path| path.starts_with("api/")));
    assert!(top_paths.iter().any(|path| {
        path.starts_with("domain/")
            || path.starts_with("services/")
            || path.starts_with("orchestration/")
    }));

    let _ = std::fs::remove_dir_all(project_dir);
}

#[test]
fn agent_bootstrap_natural_language_query_surfaces_followups_in_mcp_payload() {
    let project_dir = temp_dir("rmu-mcp-tests-bootstrap-broad-nl");
    write_bootstrap_broad_fixture(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "agent_bootstrap",
            "arguments": {
                "query": "How does the login flow move through the app and which code paths and tests matter most?",
                "limit": 5,
                "semantic": true,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("agent_bootstrap broad natural-language query should succeed");

    assert_eq!(result["isError"], json!(false));
    let bundle = &result["structuredContent"]["query_bundle"];
    let top_paths = bundle["hits"]
        .as_array()
        .expect("hits should be array")
        .iter()
        .take(4)
        .filter_map(|hit| hit["path"].as_str())
        .collect::<Vec<_>>();
    assert!(top_paths.iter().any(|path| path.starts_with("backend/")));
    assert!(
        top_paths
            .iter()
            .any(|path| path.starts_with("frontend/") || path.starts_with("tests/"))
    );
    assert!(
        bundle["followups"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let _ = std::fs::remove_dir_all(project_dir);
}
