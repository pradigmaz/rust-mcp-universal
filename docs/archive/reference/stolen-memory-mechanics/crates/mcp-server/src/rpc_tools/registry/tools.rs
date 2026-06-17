use serde_json::{Value, json};

use super::bootstrap_tools::bootstrap_tools;
use super::diagnostics_tools::diagnostics_tools;
use super::helpers::{json_schema_object, tool, tool_with_output};

pub(super) fn tools_list() -> Value {
    let privacy_mode = json!({
        "type": "string",
        "description": "Privacy mode for returned paths and content.",
        "oneOf": [{"const": "off"}, {"const": "mask"}, {"const": "hash"}]
    });
    let auto_index = json!({
        "type": "boolean",
        "description": "Automatically rebuild the derived index if it does not exist."
    });
    let string_list = json!({
        "type": "array",
        "items": {"type": "string", "minLength": 1}
    });
    let storage_mode = json!({
        "type": "string",
        "description": "Where canonical memory should be stored for this project.",
        "oneOf": [{"const": "codex"}, {"const": "project"}]
    });
    let node_ref = json_schema_object(
        &[
            ("id", json!({"type": "string"})),
            ("slug", json!({"type": "string"})),
            ("title", json!({"type": "string"})),
            ("node_type", json!({"type": "string"})),
            ("file_path", json!({"type": "string"})),
        ],
        &["id", "slug", "title", "node_type", "file_path"],
    );

    let mut tools = vec![
        tool(
            "set_project",
            "Set active project path for subsequent memory queries",
            json_schema_object(
                &[
                    (
                        "project_path",
                        json!({
                            "type": "string",
                            "minLength": 1,
                            "description": "Absolute path, relative path, or file:// URI for the Obsidian vault root."
                        }),
                    ),
                    ("storage_mode", storage_mode.clone()),
                ],
                &["project_path"],
            ),
            false,
            false,
            true,
        ),
        tool(
            "project_brief",
            "Get compact current-state project memory summary for agent bootstrap",
            json_schema_object(
                &[
                    ("auto_index", auto_index.clone()),
                    ("privacy_mode", privacy_mode.clone()),
                ],
                &[],
            ),
            true,
            false,
            true,
        ),
        tool(
            "search_memory",
            "Search indexed Markdown memory using SQLite FTS",
            json_schema_object(
                &[
                    (
                        "query",
                        json!({
                            "type": "string",
                            "minLength": 1,
                            "description": "Search query."
                        }),
                    ),
                    (
                        "limit",
                        json!({
                            "type": "integer",
                            "minimum": 1,
                            "description": "Maximum number of hits."
                        }),
                    ),
                    ("auto_index", auto_index.clone()),
                    ("privacy_mode", privacy_mode.clone()),
                ],
                &["query"],
            ),
            true,
            false,
            true,
        ),
        tool(
            "open_nodes",
            "Open specific nodes by id, slug, title, or alias",
            json_schema_object(
                &[
                    (
                        "slugs",
                        json!({
                            "type": "array",
                            "minItems": 1,
                            "items": {"type": "string", "minLength": 1},
                            "description": "Node references to open; accepts id, slug, title, or alias."
                        }),
                    ),
                    ("auto_index", auto_index.clone()),
                    ("privacy_mode", privacy_mode.clone()),
                ],
                &["slugs"],
            ),
            true,
            false,
            true,
        ),
        tool(
            "read_graph",
            "Read direct graph neighborhood for the given node references",
            json_schema_object(
                &[
                    (
                        "slugs",
                        json!({
                            "type": "array",
                            "minItems": 1,
                            "items": {"type": "string", "minLength": 1},
                            "description": "Seed node references; accepts id, slug, title, or alias."
                        }),
                    ),
                    ("auto_index", auto_index.clone()),
                    ("privacy_mode", privacy_mode.clone()),
                ],
                &["slugs"],
            ),
            true,
            false,
            true,
        ),
        tool_with_output(
            "create_node",
            "Create a canonical Markdown memory node, inject required project/section graph links, and refresh derived state; duplicate current nodes and system-managed section hubs are rejected",
            json_schema_object(
                &[
                    ("type", json!({"type": "string", "minLength": 1})),
                    ("title", json!({"type": "string", "minLength": 1})),
                    ("slug", json!({"type": "string", "minLength": 1})),
                    ("status", json!({"type": "string", "minLength": 1})),
                    ("summary", json!({"type": "string", "minLength": 1})),
                    ("tags", string_list.clone()),
                    ("aliases", string_list.clone()),
                ],
                &["type", "title"],
            ),
            json_schema_object(
                &[
                    ("node", node_ref.clone()),
                    ("sync_status", json!({"type": "string"})),
                ],
                &["node", "sync_status"],
            ),
            false,
            false,
            false,
        ),
        tool_with_output(
            "add_observation",
            "Append a canonical observation bullet to an existing non-system node",
            json_schema_object(
                &[
                    ("node", json!({"type": "string", "minLength": 1})),
                    ("content", json!({"type": "string", "minLength": 1})),
                ],
                &["node", "content"],
            ),
            json_schema_object(
                &[
                    ("node", node_ref.clone()),
                    ("added", json!({"type": "boolean"})),
                    ("sync_status", json!({"type": "string"})),
                ],
                &["node", "added", "sync_status"],
            ),
            false,
            false,
            false,
        ),
        tool_with_output(
            "link_nodes",
            "Create one directed canonical relation between two non-system nodes",
            json_schema_object(
                &[
                    ("source", json!({"type": "string", "minLength": 1})),
                    ("target", json!({"type": "string", "minLength": 1})),
                    ("relation_kind", json!({"type": "string", "minLength": 1})),
                ],
                &["source", "target", "relation_kind"],
            ),
            json_schema_object(
                &[
                    ("source", node_ref.clone()),
                    ("target", node_ref.clone()),
                    ("relation_kind", json!({"type": "string"})),
                    ("changed", json!({"type": "boolean"})),
                    ("sync_status", json!({"type": "string"})),
                ],
                &[
                    "source",
                    "target",
                    "relation_kind",
                    "changed",
                    "sync_status",
                ],
            ),
            false,
            false,
            true,
        ),
        tool_with_output(
            "unlink_nodes",
            "Remove one directed canonical relation between two non-system nodes; required project/section graph links are protected",
            json_schema_object(
                &[
                    ("source", json!({"type": "string", "minLength": 1})),
                    ("target", json!({"type": "string", "minLength": 1})),
                    ("relation_kind", json!({"type": "string", "minLength": 1})),
                ],
                &["source", "target", "relation_kind"],
            ),
            json_schema_object(
                &[
                    ("source", node_ref.clone()),
                    ("target", node_ref.clone()),
                    ("relation_kind", json!({"type": "string"})),
                    ("changed", json!({"type": "boolean"})),
                    ("sync_status", json!({"type": "string"})),
                ],
                &[
                    "source",
                    "target",
                    "relation_kind",
                    "changed",
                    "sync_status",
                ],
            ),
            false,
            true,
            true,
        ),
        tool_with_output(
            "update_node",
            "Update supported canonical fields on an existing non-system node",
            json_schema_object(
                &[
                    ("node", json!({"type": "string", "minLength": 1})),
                    ("title", json!({"type": "string", "minLength": 1})),
                    ("status", json!({"type": "string", "minLength": 1})),
                    ("summary", json!({"type": "string", "minLength": 1})),
                    ("tags", string_list),
                    (
                        "aliases",
                        json!({
                            "type": "array",
                            "items": {"type": "string", "minLength": 1}
                        }),
                    ),
                ],
                &["node"],
            ),
            json_schema_object(
                &[
                    ("node", node_ref),
                    ("changed", json!({"type": "boolean"})),
                    ("sync_status", json!({"type": "string"})),
                ],
                &["node", "changed", "sync_status"],
            ),
            false,
            false,
            false,
        ),
    ];
    tools.extend(diagnostics_tools(&auto_index, &privacy_mode));
    tools.extend(bootstrap_tools(&auto_index, &privacy_mode));
    json!({
        "tools": tools
    })
}
