use serde_json::{Value, json};

use super::helpers::{json_schema_object, tool};
use super::schemas::{
    agent_intent_mode_schema, bootstrap_profile_schema, budget_query_schema, call_path_schema,
    context_pack_schema, db_maintenance_schema, delete_index_schema, index_schema,
    install_ignore_rules_schema, investigation_schema, migration_mode_schema, navigation_schema,
    preflight_schema, privacy_mode_schema, quality_hotspots_schema, quality_snapshot_schema,
    query_benchmark_schema, query_schema, report_query_schema, rollout_phase_schema,
    rule_violations_schema, scope_preview_schema,
};

pub(super) fn tools_list() -> Value {
    json!({
        "tools": [
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
                    &["project_path"]
                )
            ),
            tool(
                "install_ignore_rules",
                "Install RMU-managed ignore rules into .git/info/exclude or root .gitignore",
                install_ignore_rules_schema()
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
                        (
                            "query",
                            json!({
                                "type": "string",
                                "minLength": 1,
                                "description": "Task or question the bootstrap payload should support."
                            }),
                        ),
                        (
                            "limit",
                            json!({
                                "type": "integer",
                                "minimum": 1,
                                "description": "Maximum number of candidates to consider."
                            }),
                        ),
                        (
                            "semantic",
                            json!({
                                "type": "boolean",
                                "description": "Enable semantic reranking for candidate selection."
                            }),
                        ),
                        (
                            "auto_index",
                            json!({
                                "type": "boolean",
                                "description": "Automatically build or refresh the index if needed."
                            }),
                        ),
                        (
                            "semantic_fail_mode",
                            json!({
                                "type": "string",
                                "description": "How to behave if semantic search is unavailable.",
                                "oneOf": [
                                    {"const": "fail_open"},
                                    {"const": "fail_closed"}
                                ]
                            }),
                        ),
                        ("privacy_mode", privacy_mode_schema()),
                        (
                            "vector_layer_enabled",
                            json!({
                                "type": "boolean",
                                "description": "Allow vector-layer retrieval when available."
                            }),
                        ),
                        ("rollout_phase", rollout_phase_schema()),
                        ("migration_mode", migration_mode_schema()),
                        (
                            "max_chars",
                            json!({
                                "type": "integer",
                                "minimum": 256,
                                "description": "Maximum number of characters allowed in the assembled payload."
                            }),
                        ),
                        (
                            "max_tokens",
                            json!({
                                "type": "integer",
                                "minimum": 64,
                                "description": "Maximum number of tokens allowed in the assembled payload."
                            }),
                        ),
                        (
                            "mode",
                            agent_intent_mode_schema(
                                "Optional agent-facing intent mode. When omitted, RMU resolves one heuristically.",
                            ),
                        ),
                        (
                            "profile",
                            bootstrap_profile_schema(),
                        ),
                        (
                            "include_report",
                            json!({
                                "type": "boolean",
                                "description": "Include the expensive query report payload in the bootstrap response."
                            }),
                        ),
                        (
                            "include_investigation_summary",
                            json!({
                                "type": "boolean",
                                "description": "Include the expensive investigation summary payload in the bootstrap response."
                            }),
                        )
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
                delete_index_schema()
            ),
            tool(
                "db_maintenance",
                "Run SQLite maintenance tasks (integrity/checkpoint/vacuum/analyze/stats/prune)",
                db_maintenance_schema()
            ),
            tool(
                "preflight",
                "Inspect binary/db/runtime compatibility and stale-process risks for the current project",
                preflight_schema()
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
                "symbol_body",
                "Resolve symbol/path/query seed into body snippets with typed anchors",
                investigation_schema()
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
                "route_trace",
                "Build implementation variants with typed route segments for a conceptual seed",
                investigation_schema()
            ),
            tool(
                "constraint_evidence",
                "Collect normalized schema/model/migration constraint evidence for a conceptual seed",
                investigation_schema()
            ),
            tool(
                "concept_cluster",
                "Cluster multiple implementation variants around a conceptual seed",
                investigation_schema()
            ),
            tool(
                "divergence_report",
                "Compare implementation variants and surface divergence axes for a conceptual seed",
                investigation_schema()
            ),
            tool(
                "search_candidates",
                "Search indexed candidates by query with canonical privacy_mode values `off`, `mask`, or `hash`",
                query_schema(true)
            ),
            tool(
                "semantic_search",
                "Search indexed candidates with semantic rerank enabled",
                query_schema(false)
            ),
            tool(
                "rule_violations",
                "Report persisted file-level quality violations from the quality index; use `path_prefix` to scope paths",
                rule_violations_schema()
            ),
            tool(
                "quality_hotspots",
                "Report aggregated quality hotspots across file, directory, or module buckets",
                quality_hotspots_schema()
            ),
            tool(
                "quality_snapshot",
                "Capture a fresh project quality snapshot, persist debt-wave artifacts, and optionally compare against baseline or wave_before",
                quality_snapshot_schema()
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
                report_query_schema()
            ),
            tool(
                "query_benchmark",
                "Run query benchmark (legacy metrics or baseline-vs-candidate compare mode)",
                query_benchmark_schema()
            )
        ]
    })
}
