use serde_json::{Value, json};

use super::super::helpers::{json_schema_object, tool};
use super::super::schemas::{install_ignore_rules_schema, migration_mode_schema, preflight_schema};

pub(super) fn tools() -> Vec<Value> {
    vec![
        tool(
            "set_project_path",
            "Set active project path for subsequent queries",
            json_schema_object(
                &[(
                    "project_path",
                    json!({
                        "type": "string",
                        "minLength": 1,
                        "description": "Absolute or relative path to the repository root directory."
                    }),
                )],
                &["project_path"],
            ),
        ),
        tool(
            "install_ignore_rules",
            "Install RMU-managed ignore rules into .git/info/exclude or root .gitignore",
            install_ignore_rules_schema(),
        ),
        tool(
            "index_status",
            "Get current index statistics from local SQLite DB",
            json_schema_object(&[("migration_mode", migration_mode_schema())], &[]),
        ),
        tool(
            "workspace_brief",
            "Get immediate project snapshot for agent startup",
            json_schema_object(&[("migration_mode", migration_mode_schema())], &[]),
        ),
        tool(
            "usage_stats",
            "Inspect persisted MCP tool usage counters and recent call history",
            json_schema_object(
                &[(
                    "limit",
                    json!({
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Maximum number of recent calls to return. Defaults to 5 to keep MCP output compact."
                    }),
                )],
                &[],
            ),
        ),
        tool(
            "preflight",
            "Inspect binary/db/runtime compatibility and stale-process risks for the current project",
            preflight_schema(),
        ),
    ]
}
