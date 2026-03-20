use serde_json::{Value, json};

use super::helpers::{json_schema_object, tool};
use super::schemas::{
    budget_query_schema, call_path_schema, context_pack_schema, db_maintenance_schema,
    index_schema, migration_mode_schema, navigation_schema, privacy_mode_schema,
    query_benchmark_schema, query_schema, rollout_phase_schema, scope_preview_schema,
};

pub(super) fn tools_list() -> Value {
    json!({
        "tools": [
            tool(
                "set_project_path",
                "Set active project path for subsequent queries",
                json_schema_object(
                    &[("project_path", json!({"type": "string", "minLength": 1}))],
                    &["project_path"]
                )
            ),
            tool(
                "index_status",
                "Get current index statistics from local SQLite DB",
                json_schema_object(&[("migration_mode", migration_mode_schema())], &[])
            ),
            tool(
                "workspace_brief",
                "Get immediate project snapshot for agent startup",
                json_schema_object(&[("migration_mode", migration_mode_schema())], &[])
            ),
            tool(
                "agent_bootstrap",
                "One-shot bootstrap payload for autonomous agents",
                json_schema_object(
                    &[
                        ("query", json!({"type": "string", "minLength": 1})),
                        ("limit", json!({"type": "integer", "minimum": 1})),
                        ("semantic", json!({"type": "boolean"})),
                        ("auto_index", json!({"type": "boolean"})),
                        (
                            "semantic_fail_mode",
                            json!({
                                "type": "string",
                                "oneOf": [
                                    {"const": "fail_open"},
                                    {"const": "fail_closed"}
                                ]
                            }),
                        ),
                        ("privacy_mode", privacy_mode_schema()),
                        ("vector_layer_enabled", json!({"type": "boolean"})),
                        ("rollout_phase", rollout_phase_schema()),
                        ("migration_mode", migration_mode_schema()),
                        ("max_chars", json!({"type": "integer", "minimum": 256})),
                        ("max_tokens", json!({"type": "integer", "minimum": 64}))
                    ],
                    &[]
                )
            ),
            tool(
                "index",
                "Rebuild file index including semantic vectors",
                index_schema()
            ),
            tool(
                "semantic_index",
                "Alias for semantic index rebuild",
                index_schema()
            ),
            tool(
                "scope_preview",
                "Preview effective scope and candidate paths before indexing",
                scope_preview_schema()
            ),
            tool(
                "delete_index",
                "Delete index storage files for current project",
                json_schema_object(
                    &[
                        ("confirm", json!({"type": "boolean", "const": true})),
                        ("migration_mode", migration_mode_schema()),
                    ],
                    &["confirm"]
                )
            ),
            tool(
                "db_maintenance",
                "Run SQLite maintenance tasks (integrity/checkpoint/vacuum/analyze/stats/prune)",
                db_maintenance_schema()
            ),
            tool(
                "symbol_lookup",
                "Compatibility-only legacy navigation tool: find symbol definitions by exact or partial name",
                navigation_schema("name")
            ),
            tool(
                "symbol_lookup_v2",
                "Canonical navigation contract: find symbol definitions in result.structuredContent.hits",
                navigation_schema("name")
            ),
            tool(
                "symbol_references",
                "Compatibility-only legacy navigation tool: find files that reference a symbol",
                navigation_schema("name")
            ),
            tool(
                "symbol_references_v2",
                "Canonical navigation contract: find symbol references in result.structuredContent.hits",
                navigation_schema("name")
            ),
            tool(
                "related_files",
                "Compatibility-only legacy navigation tool: show files adjacent by dependency and call relationships",
                navigation_schema("path")
            ),
            tool(
                "related_files_v2",
                "Canonical navigation contract: show related files in result.structuredContent.hits",
                navigation_schema("path")
            ),
            tool(
                "call_path",
                "Find a bounded heuristic path over refs/deps between two endpoints",
                call_path_schema()
            ),
            tool(
                "search_candidates",
                "Search indexed candidates by query",
                query_schema(true)
            ),
            tool(
                "semantic_search",
                "Search indexed candidates with semantic rerank enabled",
                query_schema(false)
            ),
            tool(
                "build_context_under_budget",
                "Build context constrained by char/token budgets",
                budget_query_schema()
            ),
            tool(
                "context_pack",
                "Build mode-aware context pack for code, design, or bugfix work",
                context_pack_schema()
            ),
            tool(
                "query_report",
                "Generate retrieval report for a query",
                budget_query_schema()
            ),
            tool(
                "query_benchmark",
                "Run query benchmark (legacy metrics or baseline-vs-candidate compare mode)",
                query_benchmark_schema()
            )
        ]
    })
}
