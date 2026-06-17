mod support;

use serde_json::json;

use support::{ServerHarness, ensure_canonical_layout, temp_root, tool_success};

#[test]
fn line_json_bootstrap_and_diagnostics_flow_returns_compact_structures() {
    let root = temp_root("bootstrap");
    let mut server = ServerHarness::spawn();
    server.initialize_line();

    let set_project = server.call_line_tool(
        40,
        "set_project",
        json!({"project_path": root.display().to_string(), "storage_mode": "project"}),
    );
    assert_eq!(tool_success(&set_project)["status"], json!("bound"));
    let rebuilt = server.call_line_tool(41, "rebuild_index", json!({}));
    assert_eq!(tool_success(&rebuilt)["rebuilt"], json!(true));

    let project = server.call_line_tool(
        42,
        "create_node",
        json!({"type": "Project", "title": "Workspace", "slug": "_index", "summary": "Project summary."}),
    );
    assert_eq!(tool_success(&project)["node"]["slug"], json!("_index"));
    let decision = server.call_line_tool(
        43,
        "create_node",
        json!({"type": "Decision", "title": "Auth Strategy", "slug": "auth-strategy", "summary": "Use refresh rotation."}),
    );
    assert_eq!(
        tool_success(&decision)["node"]["slug"],
        json!("auth-strategy")
    );
    let risk = server.call_line_tool(
        44,
        "create_node",
        json!({"type": "Risk", "title": "Token Theft", "slug": "token-theft", "summary": "Stolen token impact."}),
    );
    assert_eq!(tool_success(&risk)["node"]["slug"], json!("token-theft"));
    let module = server.call_line_tool(
        45,
        "create_node",
        json!({"type": "Module", "title": "Auth Module", "slug": "auth-module", "summary": "Implements auth."}),
    );
    assert_eq!(tool_success(&module)["node"]["slug"], json!("auth-module"));
    let constraint = server.call_line_tool(
        46,
        "create_node",
        json!({"type": "Constraint", "title": "Ops Freeze", "slug": "ops-freeze", "summary": "Deployment freeze blocks changes."}),
    );
    assert_eq!(
        tool_success(&constraint)["node"]["slug"],
        json!("ops-freeze")
    );

    let warn_preflight = server.call_line_tool(47, "preflight", json!({}));
    let warn_preflight_payload = tool_success(&warn_preflight);
    assert_eq!(warn_preflight_payload["status"], json!("warning"));
    assert!(
        !warn_preflight_payload["missing_canonical_paths"]
            .as_array()
            .expect("paths")
            .is_empty()
    );

    let module_link = server.call_line_tool(
        48,
        "link_nodes",
        json!({"source": "auth-module", "target": "auth-strategy", "relation_kind": "implements"}),
    );
    assert_eq!(
        tool_success(&module_link)["relation_kind"],
        json!("implements")
    );
    let risk_link = server.call_line_tool(
        49,
        "link_nodes",
        json!({"source": "token-theft", "target": "auth-module", "relation_kind": "affects"}),
    );
    assert_eq!(tool_success(&risk_link)["relation_kind"], json!("affects"));
    let constraint_link = server.call_line_tool(
        50,
        "link_nodes",
        json!({"source": "ops-freeze", "target": "auth-module", "relation_kind": "blocks"}),
    );
    assert_eq!(
        tool_success(&constraint_link)["relation_kind"],
        json!("blocks")
    );
    let observation = server.call_line_tool(
        51,
        "add_observation",
        json!({"node": "token-theft", "content": "High priority risk"}),
    );
    assert_eq!(tool_success(&observation)["added"], json!(true));

    let brief = server.call_line_tool(52, "project_brief", json!({}));
    let brief_payload = tool_success(&brief);
    assert_eq!(brief_payload["summary"], json!("Project summary."));
    assert_eq!(
        brief_payload["top_decisions"][0]["slug"],
        json!("auth-strategy")
    );
    assert_eq!(brief_payload["top_risks"][0]["slug"], json!("token-theft"));
    assert_eq!(
        brief_payload["recent_changes"][0]["slug"],
        json!("token-theft")
    );

    let changes = server.call_line_tool(53, "recent_changes", json!({"limit": 5}));
    assert_eq!(
        tool_success(&changes)["changes"][0]["slug"],
        json!("token-theft")
    );
    let decisions = server.call_line_tool(54, "decision_log", json!({"limit": 5}));
    let decisions_payload = tool_success(&decisions);
    assert_eq!(
        decisions_payload["decisions"][0]["slug"],
        json!("auth-strategy")
    );
    assert_eq!(
        decisions_payload["decisions"]
            .as_array()
            .expect("decisions")
            .len(),
        1
    );
    let hotspots = server.call_line_tool(55, "risk_hotspots", json!({"limit": 5}));
    let hotspots_payload = tool_success(&hotspots);
    assert_eq!(
        hotspots_payload["risks"][0]["affects"],
        json!(["auth-module"])
    );
    assert_eq!(
        hotspots_payload["constraints"][0]["blocks"],
        json!(["auth-module"])
    );

    let pack = server.call_line_tool(
        56,
        "context_pack",
        json!({"seed": "auth", "limit": 5, "max_chars": 4000, "max_tokens": 1000}),
    );
    let pack_payload = tool_success(&pack);
    assert_eq!(pack_payload["seed"], json!("auth"));
    assert_eq!(
        pack_payload["brief"]["project"],
        json!(
            root.file_name()
                .and_then(|value| value.to_str())
                .expect("project name")
        )
    );
    assert_eq!(pack_payload["budget"]["truncated"], json!(false));
    assert!(
        pack_payload["budget"]["used_chars"]
            .as_u64()
            .expect("used chars")
            > 0
    );
    let included_slugs = pack_payload["included_nodes"]
        .as_array()
        .expect("included nodes")
        .iter()
        .filter_map(|node| node["slug"].as_str())
        .collect::<Vec<_>>();
    assert!(included_slugs.contains(&"auth-module"));
    assert!(included_slugs.contains(&"auth-strategy"));

    let memory = server.call_line_tool(57, "memory_status", json!({}));
    let memory_payload = tool_success(&memory);
    assert_eq!(memory_payload["health"], json!("ok"));
    assert_eq!(memory_payload["counts"]["notes"], json!(14));
    assert_eq!(memory_payload["counts"]["relations"], json!(33));
    assert_eq!(memory_payload["drift_detected"], json!(false));

    let index = server.call_line_tool(58, "index_status", json!({}));
    let index_payload = tool_success(&index);
    assert_eq!(index_payload["counts"]["notes"], json!(14));
    assert_eq!(index_payload["counts"]["relations"], json!(33));
    assert_eq!(index_payload["pending_markdown_files"], json!(0));
    assert_eq!(index_payload["drift_detected"], json!(false));

    let graph = server.call_line_tool(581, "read_graph", json!({"slugs": ["_index"]}));
    let graph_payload = tool_success(&graph);
    assert!(
        graph_payload["relations"]
            .as_array()
            .expect("relations")
            .iter()
            .any(|relation| relation["target_slug"] == "section-decisions")
    );
    assert!(
        graph_payload["relations"]
            .as_array()
            .expect("relations")
            .iter()
            .any(|relation| relation["target_slug"] == "section-risks")
    );

    ensure_canonical_layout(&root.join("memory"));
    let ok_preflight = server.call_line_tool(59, "preflight", json!({}));
    let ok_preflight_payload = tool_success(&ok_preflight);
    assert_eq!(ok_preflight_payload["status"], json!("ok"));
    assert_eq!(ok_preflight_payload["warnings"], json!([]));
    assert_eq!(ok_preflight_payload["errors"], json!([]));

    server.shutdown_line();
    let _ = std::fs::remove_dir_all(root);
}
