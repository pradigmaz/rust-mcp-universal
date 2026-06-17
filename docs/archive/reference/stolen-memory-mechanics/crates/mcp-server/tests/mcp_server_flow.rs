mod support;

use serde_json::json;

use support::{ServerHarness, read_note, temp_root, tool_error, tool_success, write_note};

#[test]
fn line_json_flow_returns_structured_content_for_tools() {
    let root = temp_root("line");
    write_note(
        &root.join("memory"),
        "_index.md",
        "Workspace",
        "Project",
        "Project summary.",
    );
    write_note(
        &root.join("memory"),
        "decisions/auth.md",
        "Auth Decision",
        "Decision",
        "Auth summary.",
    );

    let mut server = ServerHarness::spawn();
    server.initialize_line();

    let listed = server.list_tools_line(2);
    assert!(
        listed["result"]["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .any(|tool| tool["name"] == "search_memory")
    );

    let set_project = server.call_line_tool(
        3,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "project"}),
    );
    assert_eq!(tool_success(&set_project)["status"], json!("bound"));

    let rebuilt = server.call_line_tool(4, "rebuild_index", json!({}));
    assert_eq!(tool_success(&rebuilt)["rebuilt"], json!(true));

    let searched = server.call_line_tool(5, "search_memory", json!({"query": "Auth", "limit": 5}));
    assert_eq!(tool_success(&searched)["hits"][0]["slug"], json!("auth"));

    let opened = server.call_line_tool(6, "open_nodes", json!({"slugs": ["auth"]}));
    assert_eq!(
        tool_success(&opened)["nodes"][0]["title"],
        json!("Auth Decision")
    );

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn framed_flow_distinguishes_protocol_errors_from_tool_errors() {
    let root = temp_root("framed");
    write_note(
        &root.join("memory"),
        "_index.md",
        "Workspace",
        "Project",
        "Project summary.",
    );

    let mut server = ServerHarness::spawn();
    server.initialize_framed();

    server.send_framed(&json!({
        "jsonrpc": "2.0",
        "id": 7
    }));
    let protocol_error = server.read_framed();
    assert_eq!(protocol_error["error"]["code"], json!(-32600));

    server.send_framed(&json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "tools/call",
        "params": {
            "name": "set_project",
            "arguments": {"project_path": root.display().to_string(), "storage_mode": "project"}
        }
    }));
    let set_project = server.read_framed();
    assert_eq!(set_project["result"]["isError"], json!(false));

    server.send_framed(&json!({
        "jsonrpc": "2.0",
        "id": 9,
        "method": "tools/call",
        "params": {
            "name": "search_memory",
            "arguments": {"query": "Workspace"}
        }
    }));
    let missing_index = server.read_framed();
    assert_eq!(missing_index["result"]["isError"], json!(true));
    assert_eq!(
        missing_index["result"]["structuredContent"]["code"],
        json!("E_REBUILD_REQUIRED")
    );

    server.send_framed(&json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "memory_status",
            "arguments": {"unexpected": true}
        }
    }));
    let invalid_input = server.read_framed();
    assert_eq!(invalid_input["result"]["isError"], json!(true));
    assert_eq!(
        invalid_input["result"]["structuredContent"]["code"],
        json!("E_INVALID_INPUT")
    );

    server.shutdown_framed();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_write_read_flow_updates_markdown_and_graph() {
    let root = temp_root("write");
    let mut server = ServerHarness::spawn();
    server.initialize_line();

    let listed = server.list_tools_line(20);
    let tool_names = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    for required in [
        "create_node",
        "update_node",
        "add_observation",
        "link_nodes",
        "unlink_nodes",
        "read_graph",
    ] {
        assert!(tool_names.contains(&required), "missing tool {required}");
    }

    let set_project = server.call_line_tool(
        21,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "project"}),
    );
    assert!(
        tool_success(&set_project)["project_root"]
            .as_str()
            .is_some_and(|value| value.ends_with(&root.display().to_string()))
    );
    assert!(
        tool_success(&set_project)["memory_root"]
            .as_str()
            .is_some_and(|value| value.ends_with(&root.join("memory").display().to_string()))
    );
    let rebuilt = server.call_line_tool(22, "rebuild_index", json!({}));
    assert_eq!(tool_success(&rebuilt)["indexed_files"], json!(0));

    let project = server.call_line_tool(
        23,
        "create_node",
        json!({"type": "Project", "title": "Workspace", "slug": "_index", "summary": "Live MCP smoke project."}),
    );
    assert_eq!(tool_success(&project)["sync_status"], json!("synced"));
    let module = server.call_line_tool(
        24,
        "create_node",
        json!({"type": "Module", "title": "Auth Module", "slug": "auth-module", "summary": "Handles auth."}),
    );
    assert_eq!(tool_success(&module)["node"]["slug"], json!("auth-module"));
    let risk = server.call_line_tool(
        25,
        "create_node",
        json!({"type": "Risk", "title": "Token Risk", "slug": "token-risk", "summary": "Token theft risk."}),
    );
    assert_eq!(
        tool_success(&risk)["node"]["file_path"],
        json!("risks/Token Risk.md")
    );

    let observation = server.call_line_tool(
        26,
        "add_observation",
        json!({"node": "token-risk", "content": "Refresh token rotation required"}),
    );
    assert_eq!(tool_success(&observation)["added"], json!(true));
    let link = server.call_line_tool(
        27,
        "link_nodes",
        json!({"source": "token-risk", "target": "auth-module", "relation_kind": "affects"}),
    );
    assert_eq!(tool_success(&link)["changed"], json!(true));
    let updated = server.call_line_tool(
        28,
        "update_node",
        json!({"node": "auth-module", "summary": "Handles auth and sessions.", "tags": ["auth", "session"]}),
    );
    assert_eq!(tool_success(&updated)["changed"], json!(true));

    let searched = server.call_line_tool(
        29,
        "search_memory",
        json!({"query": "sessions", "limit": 5}),
    );
    assert_eq!(
        tool_success(&searched)["hits"][0]["slug"],
        json!("auth-module")
    );
    let opened = server.call_line_tool(
        30,
        "open_nodes",
        json!({"slugs": ["token-risk", "auth-module"]}),
    );
    let nodes = &tool_success(&opened)["nodes"];
    assert_eq!(nodes[0]["slug"], json!("token-risk"));
    assert_eq!(
        nodes[0]["observations"][0],
        json!("Refresh token rotation required")
    );
    assert_eq!(nodes[1]["summary"], json!("Handles auth and sessions."));
    assert_eq!(nodes[1]["tags"], json!(["auth", "session"]));

    let graph = server.call_line_tool(31, "read_graph", json!({"slugs": ["token-risk"]}));
    let graph_payload = tool_success(&graph);
    assert!(
        graph_payload["relations"]
            .as_array()
            .expect("relations")
            .iter()
            .any(|relation| relation["relation_kind"] == "affects"
                && relation["target_slug"] == "auth-module")
    );

    let risk_note = read_note(&root, "memory/risks/Token Risk.md");
    assert!(risk_note.contains("- Refresh token rotation required\n"));
    assert!(risk_note.contains("- affects [[auth-module|Auth Module]]\n"));
    assert!(risk_note.contains("- documents [[_index|Workspace]]\n"));
    let module_note = read_note(&root, "memory/modules/Auth Module.md");
    assert!(module_note.contains("Handles auth and sessions.\n"));
    assert!(module_note.contains("tags:\n  - auth\n  - session\n"));
    assert!(module_note.contains("- documents [[_index|Workspace]]\n"));

    let unlinked = server.call_line_tool(
        32,
        "unlink_nodes",
        json!({"source": "token-risk", "target": "auth-module", "relation_kind": "affects"}),
    );
    assert_eq!(tool_success(&unlinked)["changed"], json!(true));
    let graph_after = server.call_line_tool(33, "read_graph", json!({"slugs": ["token-risk"]}));
    let graph_after_payload = tool_success(&graph_after);
    assert!(
        graph_after_payload["nodes"][0]["relations"]
            .as_array()
            .expect("node relations")
            .iter()
            .all(|relation| relation["relation_kind"] != "affects")
    );
    assert!(
        graph_after_payload["relations"]
            .as_array()
            .expect("graph relations")
            .iter()
            .all(|relation| relation["relation_kind"] != "affects")
    );
    assert!(
        !read_note(&root, "memory/risks/Token Risk.md")
            .contains("- affects [[auth-module|Auth Module]]")
    );

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_set_project_defaults_to_codex_memory_root() {
    let root = temp_root("codex-default");
    let mut server = ServerHarness::spawn();
    server.initialize_line();

    let set_project = server.call_line_tool(
        34,
        "set_project",
        json!({"project_path": root.display().to_string()}),
    );
    let payload = tool_success(&set_project);
    assert_eq!(payload["storage_mode"], json!("codex"));
    let memory_root = payload["memory_root"].as_str().expect("memory root");
    assert!(!memory_root.starts_with(&root.display().to_string()));
    assert!(memory_root.contains("memory"));
    assert!(!memory_root.contains("memory/projects"));
    assert!(!memory_root.contains("memory\\projects"));
    assert!(!root.join("_index.md").exists());
    assert!(!root.join("memory").exists());

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_set_project_persists_repo_binding_marker() {
    let root = temp_root("binding-marker");
    let mut server = ServerHarness::spawn();
    server.initialize_line();

    let set_project = server.call_line_tool(
        35,
        "set_project",
        json!({"project_path": root.display().to_string()}),
    );
    let payload = tool_success(&set_project);
    let memory_root = payload["memory_root"].as_str().expect("memory root");
    let marker_path = root.join(".codex").join("project-memory.json");
    let marker = std::fs::read_to_string(&marker_path).expect("binding marker");

    assert!(marker.contains("\"project_key\""));
    assert!(marker.contains(memory_root.rsplit(['\\', '/']).next().expect("project key")));

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_set_project_prefers_existing_repo_binding_marker() {
    let root = temp_root("binding-marker-existing");
    std::fs::create_dir_all(root.join(".codex")).expect("marker dir");
    std::fs::write(
        root.join(".codex").join("project-memory.json"),
        r#"{
  "schema_version": 1,
  "project_slug": "obsidian-mcp-memory",
  "project_id": "0638380514",
  "project_key": "obsidian-mcp-memory--0638380514"
}"#,
    )
    .expect("marker");

    let mut server = ServerHarness::spawn();
    server.initialize_line();

    let set_project = server.call_line_tool(
        36,
        "set_project",
        json!({"project_path": root.display().to_string()}),
    );
    let payload = tool_success(&set_project);
    let expected = server
        .codex_home()
        .join("memory")
        .join("obsidian-mcp-memory--0638380514")
        .display()
        .to_string();

    assert_eq!(payload["memory_root"], json!(expected));

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_migrate_memory_root_moves_legacy_notes_into_project_mode() {
    let root = temp_root("migrate");
    write_note(
        &root,
        "_index.md",
        "Workspace",
        "Project",
        "Project summary.",
    );
    write_note(
        &root,
        "decisions/auth.md",
        "Auth Decision",
        "Decision",
        "Auth summary.",
    );

    let mut server = ServerHarness::spawn();
    server.initialize_line();

    let set_project = server.call_line_tool(
        37,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "codex"}),
    );
    assert_eq!(tool_success(&set_project)["storage_mode"], json!("codex"));

    let preflight = server.call_line_tool(38, "preflight", json!({}));
    assert_eq!(
        tool_success(&preflight)["legacy_root_layout_detected"],
        json!(true)
    );

    let dry_run = server.call_line_tool(
        39,
        "migrate_memory_root",
        json!({"target_storage_mode": "project", "dry_run": true}),
    );
    let dry_run_payload = tool_success(&dry_run);
    assert_eq!(dry_run_payload["migrated"], json!(false));
    assert_eq!(
        dry_run_payload["canonical_paths"],
        json!(["_index.md", "decisions/auth.md"])
    );

    let migrated = server.call_line_tool(
        40,
        "migrate_memory_root",
        json!({"target_storage_mode": "project", "dry_run": false}),
    );
    let migrated_payload = tool_success(&migrated);
    assert_eq!(migrated_payload["migrated"], json!(true));
    assert!(root.join("memory").join("Workspace.md").exists());
    assert!(root.join("memory").join("Workspace Decisions.md").exists());
    assert!(!root.join("_index.md").exists());

    let rebuilt = server.call_line_tool(41, "rebuild_index", json!({}));
    assert_eq!(tool_success(&rebuilt)["indexed_files"], json!(11));
    let searched = server.call_line_tool(42, "search_memory", json!({"query": "Auth"}));
    assert_eq!(tool_success(&searched)["hits"][0]["slug"], json!("auth"));

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_current_memory_flow_blocks_duplicates_and_tracks_supersedes() {
    let root = temp_root("current-memory-flow");
    let mut server = ServerHarness::spawn();
    server.initialize_line();

    let set_project = server.call_line_tool(
        41,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "project"}),
    );
    assert_eq!(tool_success(&set_project)["status"], json!("bound"));
    let rebuilt = server.call_line_tool(42, "rebuild_index", json!({}));
    assert_eq!(tool_success(&rebuilt)["indexed_files"], json!(0));

    let project = server.call_line_tool(
        43,
        "create_node",
        json!({"type": "Project", "title": "Workspace", "summary": "Current memory project."}),
    );
    assert_eq!(tool_success(&project)["node"]["slug"], json!("_index"));
    let original = server.call_line_tool(
        44,
        "create_node",
        json!({"type": "Task", "title": "Shared Task", "slug": "shared-task", "summary": "Current task."}),
    );
    assert_eq!(
        tool_success(&original)["node"]["slug"],
        json!("shared-task")
    );

    let duplicate = server.call_line_tool(
        45,
        "create_node",
        json!({"type": "Task", "title": "Shared   Task", "slug": "shared-task-v2"}),
    );
    let duplicate_payload = tool_error(&duplicate);
    assert_eq!(duplicate_payload["code"], json!("E_DUPLICATE_CANDIDATE"));

    let superseded = server.call_line_tool(
        46,
        "update_node",
        json!({"node": "shared-task", "status": "superseded"}),
    );
    assert_eq!(tool_success(&superseded)["changed"], json!(true));

    let replacement = server.call_line_tool(
        47,
        "create_node",
        json!({"type": "Task", "title": "Shared Task", "slug": "shared-task-v2", "summary": "Replacement task."}),
    );
    assert_eq!(
        tool_success(&replacement)["node"]["slug"],
        json!("shared-task-v2")
    );

    let linked = server.call_line_tool(
        48,
        "link_nodes",
        json!({"source": "shared-task-v2", "target": "shared-task", "relation_kind": "supersedes"}),
    );
    assert_eq!(tool_success(&linked)["changed"], json!(true));

    let brief = server.call_line_tool(49, "project_brief", json!({}));
    let brief_payload = tool_success(&brief);
    assert!(
        brief_payload["recent_changes"]
            .as_array()
            .expect("recent changes")
            .iter()
            .all(|item| item["slug"] != "shared-task")
    );

    let graph = server.call_line_tool(50, "read_graph", json!({"slugs": ["shared-task-v2"]}));
    assert!(
        tool_success(&graph)["relations"]
            .as_array()
            .expect("relations")
            .iter()
            .any(|item| item["relation_kind"] == "supersedes"
                && item["target_slug"] == "shared-task")
    );

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}
