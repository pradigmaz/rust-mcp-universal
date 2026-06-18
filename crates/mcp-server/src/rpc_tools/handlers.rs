use rmu_core::Engine;

use anyhow::Result;

#[path = "handlers/agent_bootstrap.rs"]
mod agent_bootstrap;
#[path = "handlers/benchmark/mod.rs"]
mod benchmark;
#[path = "handlers/benchmark.rs"]
mod benchmark_impl;
#[path = "handlers/build_context_under_budget.rs"]
mod build_context_under_budget;
#[path = "handlers/call_path.rs"]
mod call_path;
#[path = "handlers/context_pack.rs"]
mod context_pack;
#[path = "handlers/investigation.rs"]
mod investigation;
#[path = "handlers/maintenance/mod.rs"]
mod maintenance;
#[path = "handlers/maintenance.rs"]
mod maintenance_impl;
#[path = "handlers/modes.rs"]
mod modes;
#[path = "handlers/navigation/mod.rs"]
mod navigation;
#[path = "handlers/quality/mod.rs"]
mod quality;
#[path = "handlers/quality_hotspots.rs"]
mod quality_hotspots;
#[path = "handlers/quality_snapshot.rs"]
mod quality_snapshot;
#[path = "handlers/query_report.rs"]
mod query_report;
#[path = "handlers/related_files.rs"]
mod related_files;
#[path = "handlers/rule_violations.rs"]
mod rule_violations;
#[path = "handlers/search/mod.rs"]
mod search;
#[path = "handlers/search_candidates.rs"]
mod search_candidates;
#[path = "handlers/semantic_search.rs"]
mod semantic_search;
#[path = "handlers/sensitive_data.rs"]
mod sensitive_data;
#[path = "handlers/signal_memory.rs"]
mod signal_memory;
#[path = "handlers/symbol_lookup.rs"]
mod symbol_lookup;
#[path = "handlers/symbol_references.rs"]
mod symbol_references;

pub(super) use benchmark::query_benchmark;
pub(super) use maintenance::{db_maintenance, preflight};
use modes::{
    parse_optional_agent_intent_mode, parse_optional_bootstrap_profile,
    parse_optional_context_mode, parse_optional_migration_mode, parse_optional_privacy_mode,
    parse_optional_rollout_phase, parse_optional_semantic_fail_mode,
};
pub(super) use navigation::{
    call_path, concept_cluster, constraint_evidence, contract_trace, divergence_report,
    related_files, related_files_v2, route_trace, symbol_body, symbol_lookup, symbol_lookup_v2,
    symbol_references, symbol_references_v2,
};
pub(super) use quality::{
    api_surface, complexity_report, dead_code_report, mark_signal_memory, quality_hotspots,
    quality_snapshot, rule_violations, sensitive_data, signal_memory,
};
pub(super) use search::{
    agent_bootstrap, build_context_under_budget, context_pack, query_report, search_candidates,
    semantic_search,
};

fn ensure_query_index_ready(engine: &Engine, auto_index: bool) -> Result<()> {
    let _ = engine.ensure_index_ready_with_policy(auto_index)?;
    Ok(())
}
