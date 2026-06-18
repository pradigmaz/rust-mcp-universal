mod indexing;
mod parsing;
mod project;
mod usage;

use anyhow::Result;
use rmu_core::{Engine, MigrationMode};
use serde_json::{Value, json};

use crate::ServerState;

use super::errors::{invalid_params_error, is_invalid_params_error, tool_domain_error};
use super::registry::{ToolHandler, metadata, names};
use super::result::{tool_compatibility_error_result, tool_state_error_result};
use crate::rpc_tools::handlers::{
    agent_bootstrap, api_surface, build_context_under_budget, call_path, complexity_report,
    concept_cluster, constraint_evidence, context_pack, contract_trace, db_maintenance,
    dead_code_report, divergence_report, mark_signal_memory, preflight, quality_hotspots,
    quality_snapshot, query_benchmark, query_report, related_files, related_files_v2, route_trace,
    rule_violations, search_candidates, semantic_search, sensitive_data, signal_memory,
    symbol_body, symbol_lookup, symbol_lookup_v2, symbol_references, symbol_references_v2,
};

pub(super) fn handle_tool_call(params: Option<Value>, state: &mut ServerState) -> Result<Value> {
    let params = params.ok_or_else(|| invalid_params_error("tools/call params are required"))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params_error("tools/call requires string field `name`"))?;
    let args = match params.get("arguments") {
        Some(value) if value.is_object() => value.clone(),
        Some(value) => {
            return Err(invalid_params_error(format!(
                "tools/call `arguments` must be object, got {}",
                value
            )));
        }
        None => json!({}),
    };
    let metadata =
        metadata(name).ok_or_else(|| invalid_params_error(format!("unknown tool: {name}")))?;

    if metadata.requires_bound_project {
        if let Some(binding_failure) = state.binding_failure() {
            return Ok(tool_state_error_result(
                binding_failure.code,
                binding_failure.message,
                binding_failure.details,
            ));
        }
    }

    if name != names::PREFLIGHT && metadata.requires_bound_project {
        if let Some(compatibility_error) = runtime_compatibility_guard(state)? {
            return Ok(compatibility_error);
        }
    }

    let result = dispatch_registered_tool(metadata.handler, name, &args, state);
    usage::record_tool_usage(state, name, &result);
    result
}

fn dispatch_registered_tool(
    handler: ToolHandler,
    name: &str,
    args: &Value,
    state: &mut ServerState,
) -> Result<Value> {
    match handler {
        ToolHandler::AgentBootstrap => agent_bootstrap(args, state).map_err(into_tool_error),
        ToolHandler::ApiSurface => api_surface(args, state).map_err(into_tool_error),
        ToolHandler::BuildContextUnderBudget => {
            build_context_under_budget(args, state).map_err(into_tool_error)
        }
        ToolHandler::CallPath => call_path(args, state).map_err(into_tool_error),
        ToolHandler::ComplexityReport => complexity_report(args, state).map_err(into_tool_error),
        ToolHandler::ConceptCluster => concept_cluster(args, state).map_err(into_tool_error),
        ToolHandler::ConstraintEvidence => {
            constraint_evidence(args, state).map_err(into_tool_error)
        }
        ToolHandler::ContextPack => context_pack(args, state).map_err(into_tool_error),
        ToolHandler::ContractTrace => contract_trace(args, state).map_err(into_tool_error),
        ToolHandler::DbMaintenance => db_maintenance(args, state).map_err(into_tool_error),
        ToolHandler::DeadCodeReport => dead_code_report(args, state).map_err(into_tool_error),
        ToolHandler::DeleteIndex => indexing::delete_index(args, state),
        ToolHandler::DivergenceReport => divergence_report(args, state).map_err(into_tool_error),
        ToolHandler::Index => indexing::index(args, name, state),
        ToolHandler::IndexStatus => project::index_status(args, state),
        ToolHandler::InstallIgnoreRules => project::install_ignore_rules_tool(args, state),
        ToolHandler::MarkSignalMemory => mark_signal_memory(args, state).map_err(into_tool_error),
        ToolHandler::Preflight => preflight(args, state).map_err(into_tool_error),
        ToolHandler::QualityHotspots => quality_hotspots(args, state).map_err(into_tool_error),
        ToolHandler::QualitySnapshot => quality_snapshot(args, state).map_err(into_tool_error),
        ToolHandler::QueryBenchmark => query_benchmark(args, state).map_err(into_tool_error),
        ToolHandler::QueryReport => query_report(args, state).map_err(into_tool_error),
        ToolHandler::RelatedFiles => related_files(args, state).map_err(into_tool_error),
        ToolHandler::RelatedFilesV2 => related_files_v2(args, state).map_err(into_tool_error),
        ToolHandler::RouteTrace => route_trace(args, state).map_err(into_tool_error),
        ToolHandler::RuleViolations => rule_violations(args, state).map_err(into_tool_error),
        ToolHandler::ScopePreview => indexing::scope_preview(args, state),
        ToolHandler::SearchCandidates => search_candidates(args, state).map_err(into_tool_error),
        ToolHandler::SemanticSearch => semantic_search(args, state).map_err(into_tool_error),
        ToolHandler::SensitiveData => sensitive_data(args, state).map_err(into_tool_error),
        ToolHandler::SetProjectPath => project::set_project_path(args, state),
        ToolHandler::SignalMemory => signal_memory(args, state).map_err(into_tool_error),
        ToolHandler::SymbolBody => symbol_body(args, state).map_err(into_tool_error),
        ToolHandler::SymbolLookup => symbol_lookup(args, state).map_err(into_tool_error),
        ToolHandler::SymbolLookupV2 => symbol_lookup_v2(args, state).map_err(into_tool_error),
        ToolHandler::SymbolReferences => symbol_references(args, state).map_err(into_tool_error),
        ToolHandler::SymbolReferencesV2 => {
            symbol_references_v2(args, state).map_err(into_tool_error)
        }
        ToolHandler::UsageStats => usage::usage_stats(args, state),
        ToolHandler::WorkspaceBrief => project::workspace_brief(args, state),
    }
}

fn runtime_compatibility_guard(state: &ServerState) -> Result<Option<Value>> {
    let engine = Engine::new_read_only_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        MigrationMode::Auto,
    )?;
    let status = engine.preflight_status()?;
    if status.running_binary_stale || !status.errors.is_empty() {
        let message = status
            .errors
            .first()
            .cloned()
            .unwrap_or_else(|| "compatibility check failed before tool execution".to_string());
        return Ok(Some(tool_compatibility_error_result(
            message,
            Some(&status),
        )));
    }
    Ok(None)
}

fn into_tool_error(err: anyhow::Error) -> anyhow::Error {
    if is_invalid_params_error(&err) {
        err
    } else {
        tool_domain_error(err.to_string())
    }
}
