use super::tools::tools_list;

#[test]
fn write_tools_publish_output_schema_and_annotations() {
    let tools = tools_list();
    let entries = tools["tools"].as_array().expect("tools array");
    for name in [
        "create_node",
        "add_observation",
        "link_nodes",
        "unlink_nodes",
        "update_node",
    ] {
        let tool = entries
            .iter()
            .find(|entry| entry["name"] == name)
            .expect("registered tool");
        assert!(
            tool.get("outputSchema").is_some(),
            "{name} missing outputSchema"
        );
        assert_eq!(tool["annotations"]["readOnlyHint"], false);
        assert_eq!(tool["annotations"]["openWorldHint"], false);
    }
    let destructive = entries
        .iter()
        .find(|entry| entry["name"] == "unlink_nodes")
        .expect("unlink_nodes");
    assert_eq!(destructive["annotations"]["destructiveHint"], true);
}

#[test]
fn bootstrap_read_tools_publish_output_schema_and_annotations() {
    let tools = tools_list();
    let entries = tools["tools"].as_array().expect("tools array");
    for name in [
        "recent_changes",
        "decision_log",
        "risk_hotspots",
        "context_pack",
    ] {
        let tool = entries
            .iter()
            .find(|entry| entry["name"] == name)
            .expect("registered bootstrap tool");
        assert!(
            tool.get("outputSchema").is_some(),
            "{name} missing outputSchema"
        );
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        assert_eq!(tool["annotations"]["destructiveHint"], false);
        assert_eq!(tool["annotations"]["idempotentHint"], true);
        assert_eq!(tool["annotations"]["openWorldHint"], false);
    }
}

#[test]
fn diagnostics_tools_publish_output_schema_and_annotations() {
    let tools = tools_list();
    let entries = tools["tools"].as_array().expect("tools array");
    for name in [
        "memory_status",
        "preflight",
        "index_status",
        "rebuild_index",
    ] {
        let tool = entries
            .iter()
            .find(|entry| entry["name"] == name)
            .expect("registered diagnostics tool");
        assert!(
            tool.get("outputSchema").is_some(),
            "{name} missing outputSchema"
        );
        assert_eq!(tool["annotations"]["openWorldHint"], false);
    }

    let read_only = entries
        .iter()
        .find(|entry| entry["name"] == "memory_status")
        .expect("memory_status");
    assert_eq!(read_only["annotations"]["readOnlyHint"], true);
    assert_eq!(read_only["annotations"]["destructiveHint"], false);
    assert_eq!(read_only["annotations"]["idempotentHint"], true);

    let rebuild = entries
        .iter()
        .find(|entry| entry["name"] == "rebuild_index")
        .expect("rebuild_index");
    assert_eq!(rebuild["annotations"]["readOnlyHint"], false);
    assert_eq!(rebuild["annotations"]["destructiveHint"], true);
    assert_eq!(rebuild["annotations"]["idempotentHint"], true);
}
