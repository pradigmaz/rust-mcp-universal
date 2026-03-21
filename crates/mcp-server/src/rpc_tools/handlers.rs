use anyhow::Result;
use serde_json::Value;

use rmu_core::{Engine, IndexProfile, IndexingOptions};

use crate::ServerState;

#[path = "handlers/agent_bootstrap.rs"]
mod agent_bootstrap;
#[path = "handlers/benchmark.rs"]
mod benchmark;
#[path = "handlers/build_context_under_budget.rs"]
mod build_context_under_budget;
#[path = "handlers/call_path.rs"]
mod call_path;
#[path = "handlers/context_pack.rs"]
mod context_pack;
#[path = "handlers/maintenance.rs"]
mod maintenance;
#[path = "handlers/modes.rs"]
mod modes;
#[path = "handlers/query_report.rs"]
mod query_report;
#[path = "handlers/related_files.rs"]
mod related_files;
#[path = "handlers/rule_violations.rs"]
mod rule_violations;
#[path = "handlers/search_candidates.rs"]
mod search_candidates;
#[path = "handlers/semantic_search.rs"]
mod semantic_search;
#[path = "handlers/symbol_lookup.rs"]
mod symbol_lookup;
#[path = "handlers/symbol_references.rs"]
mod symbol_references;

use modes::{
    parse_optional_context_mode, parse_optional_migration_mode, parse_optional_privacy_mode,
    parse_optional_rollout_phase, parse_optional_semantic_fail_mode,
};

pub(super) fn query_benchmark(args: &Value, state: &mut ServerState) -> Result<Value> {
    benchmark::query_benchmark(args, state)
}

pub(super) fn db_maintenance(args: &Value, state: &mut ServerState) -> Result<Value> {
    maintenance::db_maintenance(args, state)
}

pub(super) fn search_candidates(args: &Value, state: &mut ServerState) -> Result<Value> {
    search_candidates::search_candidates(args, state)
}

pub(super) fn semantic_search(args: &Value, state: &mut ServerState) -> Result<Value> {
    semantic_search::semantic_search(args, state)
}

pub(super) fn build_context_under_budget(args: &Value, state: &mut ServerState) -> Result<Value> {
    build_context_under_budget::build_context_under_budget(args, state)
}

pub(super) fn context_pack(args: &Value, state: &mut ServerState) -> Result<Value> {
    context_pack::context_pack(args, state)
}

pub(super) fn query_report(args: &Value, state: &mut ServerState) -> Result<Value> {
    query_report::query_report(args, state)
}

pub(super) fn symbol_lookup(args: &Value, state: &mut ServerState) -> Result<Value> {
    symbol_lookup::symbol_lookup(args, state)
}

pub(super) fn symbol_lookup_v2(args: &Value, state: &mut ServerState) -> Result<Value> {
    symbol_lookup::symbol_lookup_v2(args, state)
}

pub(super) fn symbol_references(args: &Value, state: &mut ServerState) -> Result<Value> {
    symbol_references::symbol_references(args, state)
}

pub(super) fn symbol_references_v2(args: &Value, state: &mut ServerState) -> Result<Value> {
    symbol_references::symbol_references_v2(args, state)
}

pub(super) fn related_files(args: &Value, state: &mut ServerState) -> Result<Value> {
    related_files::related_files(args, state)
}

pub(super) fn related_files_v2(args: &Value, state: &mut ServerState) -> Result<Value> {
    related_files::related_files_v2(args, state)
}

pub(super) fn rule_violations(args: &Value, state: &mut ServerState) -> Result<Value> {
    rule_violations::rule_violations(args, state)
}

pub(super) fn call_path(args: &Value, state: &mut ServerState) -> Result<Value> {
    call_path::call_path(args, state)
}

pub(super) fn agent_bootstrap(args: &Value, state: &mut ServerState) -> Result<Value> {
    agent_bootstrap::agent_bootstrap(args, state)
}

fn ensure_query_index_ready(engine: &Engine, auto_index: bool) -> Result<()> {
    if !auto_index {
        let _ = engine.ensure_index_ready_with_policy(false)?;
        return Ok(());
    }

    if engine.resolve_default_index_profile(None).is_some() {
        let _ = engine.ensure_index_ready_with_policy(true)?;
        return Ok(());
    }

    match engine.ensure_index_ready_with_policy(false) {
        Ok(_) => {
            if index_contains_non_code_languages(engine)? {
                reindex_with_mixed_profile(engine)?;
            }
        }
        Err(_) => reindex_with_mixed_profile(engine)?,
    }
    Ok(())
}

fn reindex_with_mixed_profile(engine: &Engine) -> Result<()> {
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: Some(IndexProfile::Mixed),
        reindex: true,
        ..IndexingOptions::default()
    })?;
    Ok(())
}

fn index_contains_non_code_languages(engine: &Engine) -> Result<bool> {
    let brief = engine.workspace_brief_with_policy(false)?;
    Ok(brief.languages.iter().any(|stat| {
        matches!(
            stat.language.as_str(),
            "markdown" | "json" | "toml" | "text"
        )
    }))
}
