mod support;

use serde_json::json;

use support::{ServerHarness, temp_root, tool_error, tool_success, write_note};

#[cfg(not(windows))]
use support::{temp_root_on_workspace_mount, windows_file_uri, windows_style_path};

#[cfg(not(windows))]
#[test]
fn line_json_set_project_translates_windows_paths_on_unix() {
    let root = temp_root_on_workspace_mount("translated-set-project");
    let mut server = ServerHarness::spawn();
    server.initialize_line();

    let response = server.call_line_tool(
        60,
        "set_project",
        json!({"project_path": windows_style_path(&root), "storage_mode": "project"}),
    );

    assert_eq!(tool_success(&response)["status"], json!("bound"));
    assert_eq!(
        tool_success(&response)["project_root"],
        json!(
            std::fs::canonicalize(&root)
                .expect("canonical root")
                .display()
                .to_string()
        )
    );

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(not(windows))]
#[test]
fn line_json_initialize_binds_root_uri_from_windows_file_uri_on_unix() {
    let root = temp_root_on_workspace_mount("translated-initialize");
    let mut server = ServerHarness::spawn();
    server.initialize_line_with_params(json!({
        "rootUri": windows_file_uri(&root),
        "storageMode": "project"
    }));

    let created = server.call_line_tool(
        61,
        "create_node",
        json!({
            "type": "Project",
            "title": "Workspace",
            "slug": "_index",
            "summary": "Bound through initialize."
        }),
    );
    assert_eq!(tool_success(&created)["node"]["slug"], json!("_index"));
    assert!(root.join("memory").join("_index.md").exists());

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(not(windows))]
#[test]
fn line_json_set_project_rejects_missing_translated_windows_path_with_hint() {
    let root = temp_root_on_workspace_mount("translated-missing");
    let missing = format!(r"{}\missing-vault", windows_style_path(&root));
    let mut server = ServerHarness::spawn();
    server.initialize_line();

    let response = server.call_line_tool(62, "set_project", json!({"project_path": missing}));
    let payload = tool_error(&response);

    assert_eq!(payload["code"], json!("E_INVALID_INPUT"));
    assert!(
        payload["error"]
            .as_str()
            .expect("error")
            .contains("/mnt/<drive>/...")
    );

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_rebuild_and_index_status_surface_parse_failures() {
    let root = temp_root("parse-failure");
    let malformed = root.join("memory").join("decisions").join("bad.md");
    std::fs::create_dir_all(malformed.parent().expect("parent")).expect("create dir");
    std::fs::write(
        &malformed,
        "# Not canonical\n\nThis file is inside a canonical directory but is malformed.",
    )
    .expect("write malformed note");

    let mut server = ServerHarness::spawn();
    server.initialize_line();
    let set_project = server.call_line_tool(
        63,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "project"}),
    );
    assert_eq!(tool_success(&set_project)["status"], json!("bound"));

    let rebuilt = server.call_line_tool(64, "rebuild_index", json!({}));
    let rebuilt_payload = tool_success(&rebuilt);
    assert_eq!(rebuilt_payload["rebuilt"], json!(true));
    assert!(
        !rebuilt_payload["errors"]
            .as_array()
            .expect("errors")
            .is_empty()
    );

    let status = server.call_line_tool(65, "index_status", json!({}));
    let status_payload = tool_success(&status);
    assert_eq!(status_payload["indexed"], json!(true));
    assert_eq!(status_payload["drift_detected"], json!(true));
    assert!(
        !status_payload["inconsistencies"]["parse_failures"]
            .as_array()
            .expect("parse failures")
            .is_empty()
    );
    assert!(
        status_payload["failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(|item| item
                .as_str()
                .is_some_and(|value| value.contains("parse error")))
    );

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_read_tools_report_stale_index_with_recovery_hint() {
    let root = temp_root("stale-index");
    write_note(
        &root.join("memory"),
        "decisions/auth.md",
        "Auth Decision",
        "Decision",
        "Original summary.",
    );

    let mut server = ServerHarness::spawn();
    server.initialize_line();
    let set_project = server.call_line_tool(
        66,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "project"}),
    );
    assert_eq!(tool_success(&set_project)["status"], json!("bound"));
    let rebuilt = server.call_line_tool(67, "rebuild_index", json!({}));
    assert_eq!(tool_success(&rebuilt)["rebuilt"], json!(true));

    let note_path = root.join("memory").join("decisions").join("auth.md");
    let mut content = std::fs::read_to_string(&note_path).expect("read note");
    content.push_str("\n<!-- drift -->\n");
    std::fs::write(&note_path, content).expect("write drift");

    let response = server.call_line_tool(68, "open_nodes", json!({"slugs": ["auth"]}));
    let payload = tool_error(&response);
    assert_eq!(payload["code"], json!("E_STALE_INDEX"));
    assert_eq!(payload["details"]["kind"], json!("derived_state"));
    assert!(
        payload["details"]["safe_recovery_hint"]
            .as_str()
            .expect("recovery hint")
            .contains("rebuild_index")
    );

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_write_tools_surface_missing_targets_as_domain_errors() {
    let root = temp_root("write-target-failures");
    let mut server = ServerHarness::spawn();
    server.initialize_line();
    let set_project = server.call_line_tool(
        69,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "project"}),
    );
    assert_eq!(tool_success(&set_project)["status"], json!("bound"));
    let rebuilt = server.call_line_tool(70, "rebuild_index", json!({}));
    assert_eq!(tool_success(&rebuilt)["rebuilt"], json!(true));
    let created = server.call_line_tool(
        71,
        "create_node",
        json!({"type": "Task", "title": "Shared Task", "slug": "shared-task"}),
    );
    assert_eq!(tool_success(&created)["node"]["slug"], json!("shared-task"));

    let missing_update = server.call_line_tool(
        72,
        "update_node",
        json!({"node": "missing-task", "summary": "Should fail"}),
    );
    let missing_update_payload = tool_error(&missing_update);
    assert_eq!(missing_update_payload["code"], json!("E_NOT_FOUND"));
    assert_eq!(
        missing_update_payload["details"]["node"],
        json!("missing-task")
    );

    let missing_link = server.call_line_tool(
        73,
        "link_nodes",
        json!({
            "source": "shared-task",
            "target": "missing-task",
            "relation_kind": "depends_on"
        }),
    );
    let missing_link_payload = tool_error(&missing_link);
    assert_eq!(missing_link_payload["code"], json!("E_NOT_FOUND"));
    assert_eq!(
        missing_link_payload["details"]["node"],
        json!("missing-task")
    );

    let missing_observation = server.call_line_tool(
        74,
        "add_observation",
        json!({"node": "missing-task", "content": "Should fail"}),
    );
    let missing_observation_payload = tool_error(&missing_observation);
    assert_eq!(missing_observation_payload["code"], json!("E_NOT_FOUND"));
    assert_eq!(
        missing_observation_payload["details"]["node"],
        json!("missing-task")
    );

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_create_node_surfaces_duplicate_candidates() {
    let root = temp_root("duplicate-candidates");
    let mut server = ServerHarness::spawn();
    server.initialize_line();
    let set_project = server.call_line_tool(
        75,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "project"}),
    );
    assert_eq!(tool_success(&set_project)["status"], json!("bound"));
    let rebuilt = server.call_line_tool(76, "rebuild_index", json!({}));
    assert_eq!(tool_success(&rebuilt)["rebuilt"], json!(true));

    let created = server.call_line_tool(
        77,
        "create_node",
        json!({
            "type": "Task",
            "title": "Shared Task",
            "slug": "shared-task",
            "aliases": ["Canonical Shared Task"]
        }),
    );
    assert_eq!(tool_success(&created)["node"]["slug"], json!("shared-task"));

    let duplicate = server.call_line_tool(
        78,
        "create_node",
        json!({
            "type": "Task",
            "title": "Shared   Task",
            "slug": "shared-task-v2"
        }),
    );
    let payload = tool_error(&duplicate);
    assert_eq!(payload["code"], json!("E_DUPLICATE_CANDIDATE"));
    assert_eq!(
        payload["details"]["candidates"][0]["slug"],
        json!("shared-task")
    );
    assert!(
        payload["details"]["candidates"][0]["why_matched"]
            .as_str()
            .is_some_and(|value| value.contains("normalized_title"))
    );

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_read_tools_surface_ambiguous_targets_with_stable_code() {
    let root = temp_root("ambiguous-read-target");
    let mut server = ServerHarness::spawn();
    server.initialize_line();
    let set_project = server.call_line_tool(
        93,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "project"}),
    );
    assert_eq!(tool_success(&set_project)["status"], json!("bound"));
    let rebuilt = server.call_line_tool(94, "rebuild_index", json!({}));
    assert_eq!(tool_success(&rebuilt)["rebuilt"], json!(true));

    let task = server.call_line_tool(
        95,
        "create_node",
        json!({"type": "Task", "title": "Shared Read", "slug": "shared-read-task"}),
    );
    assert_eq!(
        tool_success(&task)["node"]["slug"],
        json!("shared-read-task")
    );
    let module = server.call_line_tool(
        96,
        "create_node",
        json!({"type": "Module", "title": "Shared Read", "slug": "shared-read-module"}),
    );
    assert_eq!(
        tool_success(&module)["node"]["slug"],
        json!("shared-read-module")
    );

    let opened = server.call_line_tool(97, "open_nodes", json!({"slugs": ["Shared Read"]}));
    let payload = tool_error(&opened);
    assert_eq!(payload["code"], json!("E_AMBIGUOUS_TARGET"));
    assert_eq!(payload["details"]["node"], json!("Shared Read"));
    assert_eq!(payload["details"]["column"], json!("title_or_alias"));

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_migrate_memory_root_requires_explicit_source_root_when_ambiguous() {
    let root = temp_root("ambiguous-migration");
    write_note(
        &root,
        "_index.md",
        "Workspace",
        "Project",
        "Project summary.",
    );
    write_note(
        &root.join(".memory"),
        "tasks/eval.md",
        "Eval",
        "Task",
        "Task summary.",
    );

    let mut server = ServerHarness::spawn();
    server.initialize_line();
    let set_project = server.call_line_tool(
        79,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "project"}),
    );
    assert_eq!(tool_success(&set_project)["status"], json!("bound"));

    let dry_run = server.call_line_tool(
        80,
        "migrate_memory_root",
        json!({"target_storage_mode": "project", "dry_run": true}),
    );
    let dry_run_payload = tool_success(&dry_run);
    assert_eq!(
        dry_run_payload["candidate_sources"]
            .as_array()
            .expect("candidate sources")
            .len(),
        2
    );

    let ambiguous = server.call_line_tool(
        81,
        "migrate_memory_root",
        json!({"target_storage_mode": "project", "dry_run": false}),
    );
    let ambiguous_payload = tool_error(&ambiguous);
    assert_eq!(ambiguous_payload["code"], json!("E_AMBIGUOUS_SOURCE_ROOT"));

    let explicit = server.call_line_tool(
        82,
        "migrate_memory_root",
        json!({
            "target_storage_mode": "project",
            "dry_run": false,
            "source_root": root.join(".memory").display().to_string()
        }),
    );
    assert_eq!(tool_success(&explicit)["migrated"], json!(true));

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_system_section_hubs_reject_direct_updates() {
    let root = temp_root("system-section-hubs");
    let mut server = ServerHarness::spawn();
    server.initialize_line();

    let set_project = server.call_line_tool(
        83,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "project"}),
    );
    assert_eq!(tool_success(&set_project)["status"], json!("bound"));
    let rebuilt = server.call_line_tool(84, "rebuild_index", json!({}));
    assert_eq!(tool_success(&rebuilt)["rebuilt"], json!(true));
    let created = server.call_line_tool(
        85,
        "create_node",
        json!({"type": "Task", "title": "Field Check", "slug": "field-check"}),
    );
    assert_eq!(tool_success(&created)["node"]["slug"], json!("field-check"));

    let update = server.call_line_tool(
        86,
        "update_node",
        json!({"node": "section-tasks", "title": "Manual Tasks"}),
    );
    assert_eq!(tool_error(&update)["code"], json!("E_SYSTEM_NODE_TYPE"));

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn line_json_representative_tools_reject_unknown_fields() {
    let root = temp_root("unknown-fields");
    let mut server = ServerHarness::spawn();
    server.initialize_line();

    let bad_set_project = server.call_line_tool(
        87,
        "set_project",
        json!({"project_path": root.display().to_string(), "unexpected": true}),
    );
    assert_eq!(
        tool_error(&bad_set_project)["code"],
        json!("E_INVALID_INPUT")
    );

    let set_project = server.call_line_tool(
        88,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "project"}),
    );
    assert_eq!(tool_success(&set_project)["status"], json!("bound"));
    let rebuilt = server.call_line_tool(89, "rebuild_index", json!({}));
    assert_eq!(tool_success(&rebuilt)["rebuilt"], json!(true));
    let created = server.call_line_tool(
        90,
        "create_node",
        json!({"type": "Task", "title": "Field Check", "slug": "field-check"}),
    );
    assert_eq!(tool_success(&created)["node"]["slug"], json!("field-check"));

    let bad_search = server.call_line_tool(
        91,
        "search_memory",
        json!({"query": "Field", "unexpected": true}),
    );
    assert_eq!(tool_error(&bad_search)["code"], json!("E_INVALID_INPUT"));

    let bad_update = server.call_line_tool(
        92,
        "update_node",
        json!({"node": "field-check", "summary": "Updated", "unexpected": true}),
    );
    assert_eq!(tool_error(&bad_update)["code"], json!("E_INVALID_INPUT"));

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}
