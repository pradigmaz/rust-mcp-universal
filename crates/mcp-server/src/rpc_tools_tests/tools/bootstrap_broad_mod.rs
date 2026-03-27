use serde_json::json;

use super::bootstrap_broad_shared::*;
use super::*;

fn run_mod_query(semantic: bool) {
    let project_dir = temp_dir("rmu-mcp-tests-bootstrap-mod-roots");
    write_bootstrap_broad_fixture(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    let result = handle_tool_call(
        Some(json!({
            "name": "agent_bootstrap",
            "arguments": {
                "query": "mod entrypoint mixins runtime hooks config network",
                "limit": 6,
                "semantic": semantic,
                "auto_index": true
            }
        })),
        &mut state,
    )
    .expect("agent_bootstrap generic mod query should succeed");

    assert_eq!(result["isError"], json!(false));
    let top_paths = result["structuredContent"]["query_bundle"]["hits"]
        .as_array()
        .expect("hits should be array")
        .iter()
        .take(6)
        .filter_map(|hit| hit["path"].as_str())
        .collect::<Vec<_>>();
    assert!(
        top_paths
            .iter()
            .any(|path| path.starts_with("mods/alpha_mod/"))
    );
    assert!(
        top_paths
            .iter()
            .any(|path| path.starts_with("mods/beta_mod/"))
    );
    assert!(
        top_paths.iter().any(|path| {
            path.ends_with("BetaClient.java")
                || path.ends_with("BetaClientMixin.java")
                || path.ends_with("BetaMixinConfig.java")
                || path.ends_with("ModuleVision.java")
                || path.ends_with("AlphaMod.java")
        }),
        "generic mod query should surface at least one foundational module file: {top_paths:?}"
    );
    assert!(
        top_paths
            .iter()
            .take(4)
            .filter(|path| path.starts_with("mods/beta_mod/"))
            .count()
            >= 1,
        "generic mod query should keep an alternate module root in the top window: {top_paths:?}"
    );

    let followups = result["structuredContent"]["query_bundle"]["followups"]
        .as_array()
        .expect("followups should be array for mod query")
        .iter()
        .filter_map(|item| item.as_str())
        .collect::<Vec<_>>();
    assert!(
        followups.iter().any(|item| item.contains("module roots")),
        "generic mod query should emit module-oriented followups: {followups:?}"
    );

    let _ = std::fs::remove_dir_all(project_dir);
}

#[test]
fn agent_bootstrap_generic_mod_query_spans_multiple_module_roots_in_mcp_payload() {
    run_mod_query(true);
}

#[test]
fn agent_bootstrap_generic_mod_query_rebalances_lexical_results_in_mcp_payload() {
    run_mod_query(false);
}
