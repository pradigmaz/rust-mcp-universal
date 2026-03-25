use anyhow::{Result, bail};

use crate::args::Command;
use crate::error::{CODE_CONFIRM_REQUIRED, cli_error};
use crate::validation::{require_max, require_min};

use super::modes::{
    parse_changed_since, parse_changed_since_commit, parse_context_mode,
    parse_ignore_install_target, parse_index_profile, parse_seed_kind, parse_semantic_fail_mode,
};

pub(super) fn preflight_validate(command: &Command) -> Result<()> {
    let limit_max = usize::try_from(i64::MAX).unwrap_or(usize::MAX);
    match command {
        Command::DeleteIndex { yes } => {
            if !yes {
                return Err(cli_error(
                    CODE_CONFIRM_REQUIRED,
                    "delete-index requires --yes",
                ));
            }
        }
        Command::Search {
            limit,
            semantic_fail_mode,
            ..
        } => {
            let _ = require_min("limit", *limit, 1)?;
            let _ = require_max("limit", *limit, limit_max)?;
            let _ = parse_semantic_fail_mode(semantic_fail_mode)?;
        }
        Command::SemanticSearch {
            limit,
            semantic_fail_mode,
            ..
        } => {
            let _ = require_min("limit", *limit, 1)?;
            let _ = require_max("limit", *limit, limit_max)?;
            let _ = parse_semantic_fail_mode(semantic_fail_mode)?;
        }
        Command::SymbolLookup { name, limit, .. }
        | Command::SymbolReferences { name, limit, .. } => {
            let _ = require_min("limit", *limit, 1)?;
            let _ = require_max("limit", *limit, limit_max)?;
            if name.trim().is_empty() {
                bail!("`name` must be non-empty");
            }
        }
        Command::SymbolBody {
            seed,
            seed_kind,
            limit,
            ..
        }
        | Command::RouteTrace {
            seed,
            seed_kind,
            limit,
            ..
        }
        | Command::ConstraintEvidence {
            seed,
            seed_kind,
            limit,
            ..
        }
        | Command::ConceptCluster {
            seed,
            seed_kind,
            limit,
            ..
        }
        | Command::DivergenceReport {
            seed,
            seed_kind,
            limit,
            ..
        } => {
            let _ = require_min("limit", *limit, 1)?;
            let _ = require_max("limit", *limit, limit_max)?;
            if seed.trim().is_empty() {
                bail!("`seed` must be non-empty");
            }
            let _ = parse_seed_kind(seed_kind)?;
        }
        Command::InvestigationBenchmark {
            limit,
            thresholds,
            enforce_gates,
            ..
        } => {
            let _ = require_min("limit", *limit, 1)?;
            let _ = require_max("limit", *limit, limit_max)?;
            if *enforce_gates && thresholds.is_none() {
                bail!("`investigation-benchmark` --enforce-gates requires --thresholds");
            }
        }
        Command::RelatedFiles { path, limit, .. } => {
            let _ = require_min("limit", *limit, 1)?;
            let _ = require_max("limit", *limit, limit_max)?;
            if path.trim().is_empty() {
                bail!("`path` must be non-empty");
            }
        }
        Command::CallPath {
            from, to, max_hops, ..
        } => {
            let _ = require_min("max_hops", *max_hops, 1)?;
            if from.trim().is_empty() {
                bail!("`from` must be non-empty");
            }
            if to.trim().is_empty() {
                bail!("`to` must be non-empty");
            }
        }
        Command::Context {
            limit,
            semantic_fail_mode,
            max_chars,
            max_tokens,
            ..
        }
        | Command::Report {
            limit,
            semantic_fail_mode,
            max_chars,
            max_tokens,
            ..
        } => {
            let _ = require_min("limit", *limit, 1)?;
            let _ = require_max("limit", *limit, limit_max)?;
            let _ = require_min("max_chars", *max_chars, 256)?;
            let _ = require_min("max_tokens", *max_tokens, 64)?;
            let _ = parse_semantic_fail_mode(semantic_fail_mode)?;
        }
        Command::ContextPack {
            mode,
            limit,
            semantic_fail_mode,
            max_chars,
            max_tokens,
            ..
        } => {
            let _ = require_min("limit", *limit, 1)?;
            let _ = require_max("limit", *limit, limit_max)?;
            let _ = require_min("max_chars", *max_chars, 256)?;
            let _ = require_min("max_tokens", *max_tokens, 64)?;
            let _ = parse_semantic_fail_mode(semantic_fail_mode)?;
            let _ = parse_context_mode(mode)?;
        }
        Command::QueryBenchmark {
            k,
            limit,
            semantic_fail_mode,
            max_chars,
            max_tokens,
            baseline,
            thresholds,
            runs,
            enforce_gates,
            ..
        } => {
            let _ = require_min("k", *k, 1)?;
            let _ = require_min("limit", *limit, 1)?;
            let _ = require_max("limit", *limit, limit_max)?;
            let _ = require_min("max_chars", *max_chars, 256)?;
            let _ = require_min("max_tokens", *max_tokens, 64)?;
            let _ = require_min("runs", *runs, 1)?;
            let _ = parse_semantic_fail_mode(semantic_fail_mode)?;
            let baseline_mode_requested =
                baseline.is_some() || thresholds.is_some() || *runs > 1 || *enforce_gates;
            if baseline_mode_requested && baseline.is_none() {
                bail!("`query-benchmark` baseline-vs-candidate mode requires --baseline");
            }
            if *enforce_gates && thresholds.is_none() {
                bail!("`--enforce-gates` requires --thresholds");
            }
        }
        Command::QualityMatrix {
            manifest,
            output_root,
            repo_ids,
            ..
        } => {
            if manifest.as_os_str().is_empty() {
                bail!("`manifest` must be non-empty");
            }
            if let Some(root) = output_root
                && root.as_os_str().is_empty()
            {
                bail!("`output_root` must be non-empty when provided");
            }
            if repo_ids.iter().any(|repo_id| repo_id.trim().is_empty()) {
                bail!("`repo` values must be non-empty");
            }
        }
        Command::QualityHotspots {
            aggregation,
            limit,
            path_prefix,
            language,
            rule_ids,
            sort_by,
            ..
        } => {
            let _ = require_min("limit", *limit, 1)?;
            let _ = require_max("limit", *limit, limit_max)?;
            if path_prefix
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
            {
                bail!("`path_prefix` must be non-empty when provided");
            }
            if language
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
            {
                bail!("`language` must be non-empty when provided");
            }
            if rule_ids.iter().any(|rule_id| rule_id.trim().is_empty()) {
                bail!("`rule-id` values must be non-empty");
            }
            if rmu_core::QualityHotspotAggregation::parse(aggregation).is_none() {
                bail!("`aggregation` must be one of: file, directory, module");
            }
            if rmu_core::QualityHotspotsSortBy::parse(sort_by).is_none() {
                bail!("`sort_by` must be one of: hotspot_score, risk_score_delta, new_violations");
            }
        }
        Command::Agent {
            query,
            limit,
            semantic_fail_mode,
            max_chars,
            max_tokens,
            ..
        } => {
            let _ = require_min("limit", *limit, 1)?;
            let _ = require_max("limit", *limit, limit_max)?;
            let _ = require_min("max_chars", *max_chars, 256)?;
            let _ = require_min("max_tokens", *max_tokens, 64)?;
            let _ = parse_semantic_fail_mode(semantic_fail_mode)?;
            if let Some(raw_query) = query {
                if raw_query.trim().is_empty() {
                    bail!("`query` must be non-empty when provided");
                }
            }
        }
        Command::Index(index_args)
        | Command::SemanticIndex(index_args)
        | Command::ScopePreview(index_args) => {
            if let Some(raw_profile) = &index_args.profile {
                let _ = parse_index_profile(raw_profile)?;
            }
            if let Some(raw_changed_since) = &index_args.changed_since {
                let _ = parse_changed_since(raw_changed_since)?;
            }
            if let Some(raw_changed_since_commit) = &index_args.changed_since_commit {
                let _ = parse_changed_since_commit(raw_changed_since_commit)?;
            }
            if index_args.changed_since.is_some() && index_args.changed_since_commit.is_some() {
                bail!("`changed_since` and `changed_since_commit` are mutually exclusive");
            }
        }
        Command::InstallIgnoreRules { target } => {
            let _ = parse_ignore_install_target(target)?;
        }
        Command::Status | Command::Brief | Command::DbMaintenance { .. } | Command::Preflight => {}
    }
    Ok(())
}
