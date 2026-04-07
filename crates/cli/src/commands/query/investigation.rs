use anyhow::{Result, anyhow};
use serde::Serialize;
use std::path::Path;

use super::*;

pub(crate) fn run_symbol_body(engine: &Engine, json: bool, args: InvestigationArgs) -> Result<()> {
    run_investigation(engine, json, args, |engine, args| {
        serde_json::to_value(engine.symbol_body(&args.seed, args.seed_kind, args.limit)?)
            .map_err(Into::into)
    })
}

pub(crate) fn run_route_trace(engine: &Engine, json: bool, args: InvestigationArgs) -> Result<()> {
    run_investigation(engine, json, args, |engine, args| {
        serde_json::to_value(engine.route_trace(&args.seed, args.seed_kind, args.limit)?)
            .map_err(Into::into)
    })
}

pub(crate) fn run_constraint_evidence(
    engine: &Engine,
    json: bool,
    args: InvestigationArgs,
) -> Result<()> {
    run_investigation(engine, json, args, |engine, args| {
        serde_json::to_value(engine.constraint_evidence(&args.seed, args.seed_kind, args.limit)?)
            .map_err(Into::into)
    })
}

pub(crate) fn run_concept_cluster(
    engine: &Engine,
    json: bool,
    args: InvestigationArgs,
) -> Result<()> {
    run_investigation(engine, json, args, |engine, args| {
        serde_json::to_value(engine.concept_cluster(&args.seed, args.seed_kind, args.limit)?)
            .map_err(Into::into)
    })
}

pub(crate) fn run_divergence_report(
    engine: &Engine,
    json: bool,
    args: InvestigationArgs,
) -> Result<()> {
    run_investigation(engine, json, args, |engine, args| {
        serde_json::to_value(engine.divergence_report(&args.seed, args.seed_kind, args.limit)?)
            .map_err(Into::into)
    })
}

pub(crate) fn run_contract_trace(
    engine: &Engine,
    json: bool,
    args: InvestigationArgs,
) -> Result<()> {
    run_investigation(engine, json, args, |engine, args| {
        serde_json::to_value(engine.contract_trace(&args.seed, args.seed_kind, args.limit)?)
            .map_err(Into::into)
    })
}

fn run_investigation(
    engine: &Engine,
    json: bool,
    args: InvestigationArgs,
    produce: impl FnOnce(&Engine, &InvestigationArgs) -> Result<serde_json::Value>,
) -> Result<()> {
    let limit = require_min("limit", args.limit, 1)?;
    if args.seed.trim().is_empty() {
        return Err(anyhow!("`seed` must be non-empty"));
    }
    let required_paths = seed_paths(&args.seed, args.seed_kind);
    let _ = engine.ensure_mixed_index_ready_for_paths(args.auto_index, &required_paths)?;
    let args = InvestigationArgs { limit, ..args };
    let mut payload = produce(engine, &args)?;
    sanitize_value_for_privacy(args.privacy_mode, &mut payload);
    if json {
        print_json(serde_json::to_string_pretty(&payload))?;
    } else {
        print_line(render_text_summary(&payload)?);
    }
    Ok(())
}

fn render_text_summary(payload: &serde_json::Value) -> Result<String> {
    #[derive(Serialize)]
    struct Summary<'a> {
        capability_status: &'a serde_json::Value,
        confidence: Option<&'a serde_json::Value>,
        variants: Option<usize>,
        items: Option<usize>,
    }

    let summary = Summary {
        capability_status: &payload["capability_status"],
        confidence: payload
            .get("confidence")
            .or_else(|| payload.get("overall_confidence")),
        variants: payload
            .get("variants")
            .and_then(|value| value.as_array())
            .map(Vec::len),
        items: payload
            .get("items")
            .and_then(|value| value.as_array())
            .map(Vec::len),
    };
    Ok(serde_json::to_string(&summary)?)
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
