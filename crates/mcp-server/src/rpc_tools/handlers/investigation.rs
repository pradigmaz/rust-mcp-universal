use anyhow::Result;
use serde_json::Value;
use std::path::Path;

use rmu_core::{ConceptSeedKind, Engine, MigrationMode, PrivacyMode, sanitize_value_for_privacy};

use crate::ServerState;
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_usize_with_min, parse_required_non_empty_string,
    reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{parse_optional_migration_mode, parse_optional_privacy_mode};

pub(super) fn symbol_body(args: &Value, state: &mut ServerState) -> Result<Value> {
    run_investigation(
        args,
        state,
        "symbol_body",
        |engine, seed, seed_kind, limit| {
            serde_json::to_value(engine.symbol_body(seed, seed_kind, limit)?).map_err(Into::into)
        },
    )
}

pub(super) fn route_trace(args: &Value, state: &mut ServerState) -> Result<Value> {
    run_investigation(
        args,
        state,
        "route_trace",
        |engine, seed, seed_kind, limit| {
            serde_json::to_value(engine.route_trace(seed, seed_kind, limit)?).map_err(Into::into)
        },
    )
}

pub(super) fn constraint_evidence(args: &Value, state: &mut ServerState) -> Result<Value> {
    run_investigation(
        args,
        state,
        "constraint_evidence",
        |engine, seed, seed_kind, limit| {
            serde_json::to_value(engine.constraint_evidence(seed, seed_kind, limit)?)
                .map_err(Into::into)
        },
    )
}

pub(super) fn concept_cluster(args: &Value, state: &mut ServerState) -> Result<Value> {
    run_investigation(
        args,
        state,
        "concept_cluster",
        |engine, seed, seed_kind, limit| {
            serde_json::to_value(engine.concept_cluster(seed, seed_kind, limit)?)
                .map_err(Into::into)
        },
    )
}

pub(super) fn divergence_report(args: &Value, state: &mut ServerState) -> Result<Value> {
    run_investigation(
        args,
        state,
        "divergence_report",
        |engine, seed, seed_kind, limit| {
            serde_json::to_value(engine.divergence_report(seed, seed_kind, limit)?)
                .map_err(Into::into)
        },
    )
}

pub(super) fn contract_trace(args: &Value, state: &mut ServerState) -> Result<Value> {
    run_investigation(
        args,
        state,
        "contract_trace",
        |engine, seed, seed_kind, limit| {
            serde_json::to_value(engine.contract_trace(seed, seed_kind, limit)?).map_err(Into::into)
        },
    )
}

fn run_investigation(
    args: &Value,
    state: &mut ServerState,
    tool_name: &str,
    produce: impl FnOnce(&Engine, &str, ConceptSeedKind, usize) -> Result<Value>,
) -> Result<Value> {
    reject_unknown_fields(
        args,
        tool_name,
        &[
            "seed",
            "seed_kind",
            "limit",
            "auto_index",
            "privacy_mode",
            "migration_mode",
        ],
    )?;
    let seed = parse_required_non_empty_string(args, tool_name, "seed")?;
    let seed_kind = parse_seed_kind(args, tool_name)?;
    let limit = parse_optional_usize_with_min(args, tool_name, "limit", 1, 20)?;
    let auto_index = parse_optional_bool(args, tool_name, "auto_index")?.unwrap_or(false);
    let privacy_mode =
        parse_optional_privacy_mode(args, tool_name, "privacy_mode")?.unwrap_or(PrivacyMode::Off);
    let migration_mode = parse_optional_migration_mode(args, tool_name, "migration_mode")?
        .unwrap_or(MigrationMode::Auto);

    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )?;
    let required_paths = seed_paths(&seed, seed_kind);
    let _ = engine.ensure_mixed_index_ready_for_paths(auto_index, &required_paths)?;
    let mut payload = produce(&engine, &seed, seed_kind, limit)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

fn parse_seed_kind(args: &Value, tool_name: &str) -> Result<ConceptSeedKind> {
    let raw = parse_required_non_empty_string(args, tool_name, "seed_kind")?;
    ConceptSeedKind::parse(&raw).ok_or_else(|| {
        crate::rpc_tools::errors::invalid_params_error(format!(
            "{tool_name} `seed_kind` must be one of: query, symbol, path, path_line"
        ))
    })
}

fn seed_paths(seed: &str, seed_kind: ConceptSeedKind) -> Vec<String> {
    match seed_kind {
        ConceptSeedKind::Path => vec![seed.trim().to_string()],
        ConceptSeedKind::PathLine => seed
            .trim()
            .rsplit_once(':')
            .map(|(path, line)| (path.trim(), line.trim()))
            .filter(|(path, line)| !path.is_empty() && line.parse::<usize>().is_ok())
            .map(|(path, _)| path.to_string())
            .into_iter()
            .collect(),
        ConceptSeedKind::Query | ConceptSeedKind::Symbol => {
            if Path::new(seed.trim()).extension().is_some() {
                vec![seed.trim().to_string()]
            } else {
                Vec::new()
            }
        }
    }
}
