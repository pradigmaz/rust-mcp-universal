use serde_json::{Value, json};

use super::helpers::{json_schema_object, tool};
use super::schemas::{
    agent_bootstrap_schema, budget_query_schema, call_path_schema, context_pack_schema,
    db_maintenance_schema, delete_index_schema, index_schema, install_ignore_rules_schema,
    investigation_schema, mark_signal_memory_schema, migration_mode_schema, navigation_schema,
    preflight_schema, quality_facade_schema, quality_hotspots_schema, quality_snapshot_schema,
    query_benchmark_schema, query_schema, report_query_schema, rule_violations_schema,
    scope_preview_schema, sensitive_data_schema, signal_memory_schema,
};

pub(crate) mod names {
    pub(crate) const SET_PROJECT_PATH: &str = "set_project_path";
    pub(crate) const INSTALL_IGNORE_RULES: &str = "install_ignore_rules";
    pub(crate) const INDEX_STATUS: &str = "index_status";
    pub(crate) const WORKSPACE_BRIEF: &str = "workspace_brief";
    pub(crate) const USAGE_STATS: &str = "usage_stats";
    pub(crate) const AGENT_BOOTSTRAP: &str = "agent_bootstrap";
    pub(crate) const INDEX: &str = "index";
    pub(crate) const SEMANTIC_INDEX: &str = "semantic_index";
    pub(crate) const SCOPE_PREVIEW: &str = "scope_preview";
    pub(crate) const DELETE_INDEX: &str = "delete_index";
    pub(crate) const PREFLIGHT: &str = "preflight";
    pub(crate) const SYMBOL_LOOKUP: &str = "symbol_lookup";
    pub(crate) const SYMBOL_LOOKUP_V2: &str = "symbol_lookup_v2";
    pub(crate) const SYMBOL_REFERENCES: &str = "symbol_references";
    pub(crate) const SYMBOL_REFERENCES_V2: &str = "symbol_references_v2";
    pub(crate) const SYMBOL_BODY: &str = "symbol_body";
    pub(crate) const RELATED_FILES: &str = "related_files";
    pub(crate) const RELATED_FILES_V2: &str = "related_files_v2";
    pub(crate) const CALL_PATH: &str = "call_path";
    pub(crate) const ROUTE_TRACE: &str = "route_trace";
    pub(crate) const CONSTRAINT_EVIDENCE: &str = "constraint_evidence";
    pub(crate) const CONCEPT_CLUSTER: &str = "concept_cluster";
    pub(crate) const CONTRACT_TRACE: &str = "contract_trace";
    pub(crate) const DIVERGENCE_REPORT: &str = "divergence_report";
    pub(crate) const SEARCH_CANDIDATES: &str = "search_candidates";
    pub(crate) const SEMANTIC_SEARCH: &str = "semantic_search";
    pub(crate) const RULE_VIOLATIONS: &str = "rule_violations";
    pub(crate) const DEAD_CODE_REPORT: &str = "dead_code_report";
    pub(crate) const COMPLEXITY_REPORT: &str = "complexity_report";
    pub(crate) const API_SURFACE: &str = "api_surface";
    pub(crate) const QUALITY_HOTSPOTS: &str = "quality_hotspots";
    pub(crate) const QUALITY_SNAPSHOT: &str = "quality_snapshot";
    pub(crate) const SENSITIVE_DATA: &str = "sensitive_data";
    pub(crate) const SIGNAL_MEMORY: &str = "signal_memory";
    pub(crate) const MARK_SIGNAL_MEMORY: &str = "mark_signal_memory";
    pub(crate) const BUILD_CONTEXT_UNDER_BUDGET: &str = "build_context_under_budget";
    pub(crate) const CONTEXT_PACK: &str = "context_pack";
    pub(crate) const QUERY_REPORT: &str = "query_report";
    pub(crate) const QUERY_BENCHMARK: &str = "query_benchmark";
    pub(crate) const DB_MAINTENANCE: &str = "db_maintenance";
}

#[derive(Clone, Copy)]
pub(crate) enum ToolHandler {
    AgentBootstrap,
    ApiSurface,
    BuildContextUnderBudget,
    CallPath,
    ComplexityReport,
    ConceptCluster,
    ConstraintEvidence,
    ContextPack,
    ContractTrace,
    DbMaintenance,
    DeadCodeReport,
    DeleteIndex,
    DivergenceReport,
    Index,
    IndexStatus,
    InstallIgnoreRules,
    MarkSignalMemory,
    Preflight,
    QualityHotspots,
    QualitySnapshot,
    QueryBenchmark,
    QueryReport,
    RelatedFiles,
    RelatedFilesV2,
    RouteTrace,
    RuleViolations,
    ScopePreview,
    SearchCandidates,
    SemanticSearch,
    SensitiveData,
    SetProjectPath,
    SignalMemory,
    SymbolBody,
    SymbolLookup,
    SymbolLookupV2,
    SymbolReferences,
    SymbolReferencesV2,
    UsageStats,
    WorkspaceBrief,
}

pub(crate) struct ToolMetadata {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) input_schema: fn() -> Value,
    pub(crate) handler: ToolHandler,
    pub(crate) requires_bound_project: bool,
}

const TOOL_METADATA: &[ToolMetadata] = &[
    unbound_tool(
        names::SET_PROJECT_PATH,
        "Set active project path for subsequent queries",
        set_project_path_schema,
        ToolHandler::SetProjectPath,
    ),
    bound_tool(
        names::INSTALL_IGNORE_RULES,
        "Install RMU-managed ignore rules into .git/info/exclude or root .gitignore",
        install_ignore_rules_schema,
        ToolHandler::InstallIgnoreRules,
    ),
    bound_tool(
        names::INDEX_STATUS,
        "Get current index statistics from local SQLite DB",
        migration_only_schema,
        ToolHandler::IndexStatus,
    ),
    bound_tool(
        names::WORKSPACE_BRIEF,
        "Get immediate project snapshot for agent startup",
        migration_only_schema,
        ToolHandler::WorkspaceBrief,
    ),
    unbound_tool(
        names::USAGE_STATS,
        "Inspect persisted MCP tool usage counters and recent call history",
        usage_stats_schema,
        ToolHandler::UsageStats,
    ),
    bound_tool(
        names::AGENT_BOOTSTRAP,
        "Primary explore path for agents: one-shot bootstrap payload before narrower follow-up tools",
        agent_bootstrap_schema,
        ToolHandler::AgentBootstrap,
    ),
    bound_tool(
        names::INDEX,
        "Rebuild file index including semantic vectors",
        index_schema,
        ToolHandler::Index,
    ),
    bound_tool(
        names::SEMANTIC_INDEX,
        "Alias for semantic index rebuild",
        index_schema,
        ToolHandler::Index,
    ),
    bound_tool(
        names::SCOPE_PREVIEW,
        "Preview effective scope and candidate paths before indexing",
        scope_preview_schema,
        ToolHandler::ScopePreview,
    ),
    bound_tool(
        names::DELETE_INDEX,
        "Delete index storage files for current project",
        delete_index_schema,
        ToolHandler::DeleteIndex,
    ),
    unbound_tool(
        names::PREFLIGHT,
        "Inspect binary/db/runtime compatibility and stale-process risks for the current project",
        preflight_schema,
        ToolHandler::Preflight,
    ),
    bound_tool(
        names::SYMBOL_LOOKUP,
        "Compatibility-only legacy navigation tool: find symbol definitions by exact or partial name",
        navigation_name_schema,
        ToolHandler::SymbolLookup,
    ),
    bound_tool(
        names::SYMBOL_LOOKUP_V2,
        "Canonical navigation contract: find symbol definitions in result.structuredContent.hits with reason_codes",
        navigation_name_schema,
        ToolHandler::SymbolLookupV2,
    ),
    bound_tool(
        names::SYMBOL_REFERENCES,
        "Compatibility-only legacy navigation tool: find files that reference a symbol",
        navigation_name_schema,
        ToolHandler::SymbolReferences,
    ),
    bound_tool(
        names::SYMBOL_REFERENCES_V2,
        "Canonical navigation contract: find symbol references in result.structuredContent.hits with reason_codes",
        navigation_name_schema,
        ToolHandler::SymbolReferencesV2,
    ),
    bound_tool(
        names::SYMBOL_BODY,
        "Resolve symbol/path/query seed into body snippets with typed anchors",
        investigation_schema,
        ToolHandler::SymbolBody,
    ),
    bound_tool(
        names::RELATED_FILES,
        "Compatibility-only legacy navigation tool: show files adjacent by dependency and call relationships",
        navigation_path_schema,
        ToolHandler::RelatedFiles,
    ),
    bound_tool(
        names::RELATED_FILES_V2,
        "Canonical navigation contract: show related files in result.structuredContent.hits with reason_codes",
        navigation_path_schema,
        ToolHandler::RelatedFilesV2,
    ),
    bound_tool(
        names::CALL_PATH,
        "Find a bounded heuristic path over refs/deps between two endpoints",
        call_path_schema,
        ToolHandler::CallPath,
    ),
    bound_tool(
        names::ROUTE_TRACE,
        "Build implementation variants with typed route segments for a conceptual seed",
        investigation_schema,
        ToolHandler::RouteTrace,
    ),
    bound_tool(
        names::CONSTRAINT_EVIDENCE,
        "Collect normalized schema/model/migration constraint evidence for a conceptual seed",
        investigation_schema,
        ToolHandler::ConstraintEvidence,
    ),
    bound_tool(
        names::CONCEPT_CLUSTER,
        "Cluster multiple implementation variants around a conceptual seed",
        investigation_schema,
        ToolHandler::ConceptCluster,
    ),
    bound_tool(
        names::CONTRACT_TRACE,
        "Trace cross-layer contract roots, generated lineage, and actionable next steps for a conceptual seed",
        investigation_schema,
        ToolHandler::ContractTrace,
    ),
    bound_tool(
        names::DIVERGENCE_REPORT,
        "Compare implementation variants and surface divergence axes for a conceptual seed",
        investigation_schema,
        ToolHandler::DivergenceReport,
    ),
    bound_tool(
        names::SEARCH_CANDIDATES,
        "Search indexed candidates by query with canonical privacy_mode values `off`, `mask`, or `hash`",
        search_candidates_schema,
        ToolHandler::SearchCandidates,
    ),
    bound_tool(
        names::SEMANTIC_SEARCH,
        "Search indexed candidates with semantic rerank enabled",
        semantic_search_schema,
        ToolHandler::SemanticSearch,
    ),
    bound_tool(
        names::RULE_VIOLATIONS,
        "Report compact persisted file-level quality violations from the quality index; pass details=true for full fields",
        rule_violations_schema,
        ToolHandler::RuleViolations,
    ),
    bound_tool(
        names::DEAD_CODE_REPORT,
        "Facade over quality violations for dead-code candidate signals",
        quality_facade_schema,
        ToolHandler::DeadCodeReport,
    ),
    bound_tool(
        names::COMPLEXITY_REPORT,
        "Facade over quality metrics for cyclomatic, cognitive, branch, and early-return complexity",
        quality_facade_schema,
        ToolHandler::ComplexityReport,
    ),
    bound_tool(
        names::API_SURFACE,
        "Facade over quality metrics for public API exports, re-exports, and hub signals",
        quality_facade_schema,
        ToolHandler::ApiSurface,
    ),
    bound_tool(
        names::QUALITY_HOTSPOTS,
        "Report compact aggregated quality hotspots across file, directory, or module buckets; pass details=true for full fields",
        quality_hotspots_schema,
        ToolHandler::QualityHotspots,
    ),
    bound_tool(
        names::QUALITY_SNAPSHOT,
        "Capture a fresh project quality snapshot, persist debt-wave artifacts, and optionally compare against baseline or wave_before",
        quality_snapshot_schema,
        ToolHandler::QualitySnapshot,
    ),
    bound_tool(
        names::SENSITIVE_DATA,
        "Run dedicated security-sensitive data scan over repo files without mixing results into ordinary quality scoring",
        sensitive_data_schema,
        ToolHandler::SensitiveData,
    ),
    bound_tool(
        names::SIGNAL_MEMORY,
        "Inspect repo-local remembered useful/noisy signal decisions",
        signal_memory_schema,
        ToolHandler::SignalMemory,
    ),
    bound_tool(
        names::MARK_SIGNAL_MEMORY,
        "Persist a repo-local useful/noisy decision for a specific signal key",
        mark_signal_memory_schema,
        ToolHandler::MarkSignalMemory,
    ),
    bound_tool(
        names::BUILD_CONTEXT_UNDER_BUDGET,
        "Build context constrained by char/token budgets",
        budget_query_schema,
        ToolHandler::BuildContextUnderBudget,
    ),
    bound_tool(
        names::CONTEXT_PACK,
        "Build mode-aware context pack for code, design, or bugfix work",
        context_pack_schema,
        ToolHandler::ContextPack,
    ),
    bound_tool(
        names::QUERY_REPORT,
        "Generate retrieval report for a query",
        report_query_schema,
        ToolHandler::QueryReport,
    ),
    bound_tool(
        names::QUERY_BENCHMARK,
        "Run query benchmark (legacy metrics or baseline-vs-candidate compare mode)",
        query_benchmark_schema,
        ToolHandler::QueryBenchmark,
    ),
    bound_tool(
        names::DB_MAINTENANCE,
        "Run SQLite maintenance tasks (integrity/checkpoint/vacuum/analyze/stats/prune)",
        db_maintenance_schema,
        ToolHandler::DbMaintenance,
    ),
];

const fn bound_tool(
    name: &'static str,
    description: &'static str,
    input_schema: fn() -> Value,
    handler: ToolHandler,
) -> ToolMetadata {
    ToolMetadata {
        name,
        description,
        input_schema,
        handler,
        requires_bound_project: true,
    }
}

const fn unbound_tool(
    name: &'static str,
    description: &'static str,
    input_schema: fn() -> Value,
    handler: ToolHandler,
) -> ToolMetadata {
    ToolMetadata {
        name,
        description,
        input_schema,
        handler,
        requires_bound_project: false,
    }
}

pub(crate) fn metadata(name: &str) -> Option<&'static ToolMetadata> {
    TOOL_METADATA.iter().find(|metadata| metadata.name == name)
}

pub(super) fn tools_list() -> Value {
    json!({
        "tools": TOOL_METADATA
            .iter()
            .map(|metadata| tool(metadata.name, metadata.description, (metadata.input_schema)()))
            .collect::<Vec<_>>()
    })
}

fn migration_only_schema() -> Value {
    json_schema_object(&[("migration_mode", migration_mode_schema())], &[])
}

fn navigation_name_schema() -> Value {
    navigation_schema("name")
}

fn navigation_path_schema() -> Value {
    navigation_schema("path")
}

fn search_candidates_schema() -> Value {
    query_schema(true)
}

fn semantic_search_schema() -> Value {
    query_schema(false)
}

fn set_project_path_schema() -> Value {
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
    )
}

fn usage_stats_schema() -> Value {
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
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn tools_list_matches_metadata_registry() {
        let listed = tools_list()["tools"]
            .as_array()
            .expect("tools/list payload has tools array")
            .iter()
            .map(|tool| {
                tool["name"]
                    .as_str()
                    .expect("listed tool has string name")
                    .to_string()
            })
            .collect::<BTreeSet<_>>();
        let registered = TOOL_METADATA
            .iter()
            .map(|metadata| metadata.name.to_string())
            .collect::<BTreeSet<_>>();

        assert_eq!(listed, registered);
    }
}
