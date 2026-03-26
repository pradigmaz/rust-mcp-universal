use serde_json::{Value, json};

use super::*;

#[test]
fn tool_error_result_has_structured_content() {
    let payload = tool_error_result("boom".to_string());
    assert_eq!(payload["isError"], json!(true));
    assert_eq!(payload["structuredContent"]["error"], json!("boom"));
}

#[test]
fn delete_index_tool_schema_requires_confirm_true() {
    let tools = tools_list();
    let delete_tool = tools["tools"]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|tool| tool["name"].as_str() == Some("delete_index"))
        })
        .expect("delete_index tool should exist");
    let schema = &delete_tool["inputSchema"];

    assert_required_structure(
        &json!({"confirm": true}),
        schema,
        "delete_index.schema.true",
    );
    assert_schema_rejects(
        &json!({"confirm": false}),
        schema,
        "delete_index.schema.false",
    );
}

#[test]
fn scope_preview_tool_is_registered() {
    let tools = tools_list();
    let scope_preview_tool = tools["tools"]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|tool| tool["name"].as_str() == Some("scope_preview"))
        })
        .expect("scope_preview tool should exist");
    let schema = &scope_preview_tool["inputSchema"];
    assert_required_structure(
        &json!({"include_paths": ["src"], "privacy_mode": "off"}),
        schema,
        "scope_preview.schema.shape",
    );
}

#[test]
fn navigation_v2_tools_are_registered() {
    let tools = tools_list();
    let items = tools["tools"]
        .as_array()
        .expect("tools list should contain `tools` array");

    for (name, arg_name) in [
        ("symbol_lookup_v2", "name"),
        ("symbol_references_v2", "name"),
        ("related_files_v2", "path"),
    ] {
        let tool = items
            .iter()
            .find(|tool| tool["name"].as_str() == Some(name))
            .unwrap_or_else(|| panic!("{name} tool should exist"));
        assert_required_structure(
            &json!({arg_name: "probe", "limit": 5}),
            &tool["inputSchema"],
            &format!("{name}.schema.shape"),
        );
    }
}

#[test]
fn navigation_tool_descriptions_mark_v2_as_canonical_and_legacy_as_compatibility_only() {
    let tools = tools_list();
    let items = tools["tools"]
        .as_array()
        .expect("tools list should contain `tools` array");

    for (legacy_name, v2_name) in [
        ("symbol_lookup", "symbol_lookup_v2"),
        ("symbol_references", "symbol_references_v2"),
        ("related_files", "related_files_v2"),
    ] {
        let legacy_description = items
            .iter()
            .find(|tool| tool["name"].as_str() == Some(legacy_name))
            .and_then(|tool| tool["description"].as_str())
            .unwrap_or_else(|| panic!("{legacy_name} description should exist"));
        assert!(
            legacy_description.contains("Compatibility-only legacy"),
            "{legacy_name} should be marked compatibility-only, got: {legacy_description}"
        );

        let v2_description = items
            .iter()
            .find(|tool| tool["name"].as_str() == Some(v2_name))
            .and_then(|tool| tool["description"].as_str())
            .unwrap_or_else(|| panic!("{v2_name} description should exist"));
        assert!(
            v2_description.contains("Canonical navigation contract"),
            "{v2_name} should be marked canonical, got: {v2_description}"
        );
        assert!(
            v2_description.contains("result.structuredContent.hits"),
            "{v2_name} should point clients at result.structuredContent.hits, got: {v2_description}"
        );
    }
}

#[test]
fn tools_call_rejects_non_object_arguments_payload() {
    let project_dir = temp_dir("rmu-mcp-tests-invalid-arguments-root");
    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    for bad_arguments in [json!(null), json!([1, 2]), json!("bad")] {
        let err = handle_tool_call(
            Some(json!({
                "name": "agent_bootstrap",
                "arguments": bad_arguments
            })),
            &mut state,
        )
        .expect_err("non-object `arguments` must fail");

        assert!(
            err.to_string().contains("`arguments` must be object"),
            "unexpected error: {err}"
        );
    }
}

#[test]
fn all_tool_input_schemas_use_supported_keywords() {
    let tools = tools_list();
    let tool_entries = tools["tools"]
        .as_array()
        .expect("tools list should contain `tools` array");

    for tool in tool_entries {
        let name = tool["name"]
            .as_str()
            .expect("tool entry should have string `name`");
        let schema = &tool["inputSchema"];
        if let Err(err) =
            validate_schema_keyword_coverage(schema, &format!("tool `{name}` inputSchema"))
        {
            panic!("unsupported schema keyword for tool `{name}`: {err}");
        }
    }
}

#[test]
fn all_tool_parameters_have_human_readable_descriptions() {
    let tools = tools_list();
    let tool_entries = tools["tools"]
        .as_array()
        .expect("tools list should contain `tools` array");

    for tool in tool_entries {
        let name = tool["name"]
            .as_str()
            .expect("tool entry should have string `name`");
        let properties = tool["inputSchema"]["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("tool `{name}` inputSchema should define properties"));

        for (property_name, schema) in properties {
            let description = schema
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("");
            assert!(
                !description.trim().is_empty(),
                "tool `{name}` parameter `{property_name}` should have a description"
            );
        }
    }
}
