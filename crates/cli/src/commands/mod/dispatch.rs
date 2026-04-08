use anyhow::Result;

use crate::args::Command;

use super::modes::{
    parse_agent_intent_mode, parse_bootstrap_profile, parse_context_mode,
    parse_ignore_install_target, parse_seed_kind, parse_semantic_fail_mode,
};
use super::preflight_helpers::PreparedRun;
use super::{
    indexing, maintenance, quality_hotspots, quality_matrix, quality_snapshot, query,
    sensitive_data, signal_memory,
};

pub(super) fn run(prepared: PreparedRun) -> Result<()> {
    let PreparedRun {
        engine,
        project_path,
        json,
        privacy_mode,
        vector_layer_enabled,
        rollout_phase,
        migration_mode,
        command,
    } = prepared;
    let engine = engine.as_ref();

    match command {
        Command::Index(index_args) => {
            indexing::run_index(required_engine(engine)?, json, index_args)
        }
        Command::SemanticIndex(index_args) => {
            indexing::run_semantic_index(required_engine(engine)?, json, index_args)
        }
        Command::ScopePreview(index_args) => {
            indexing::run_scope_preview(required_engine(engine)?, json, privacy_mode, index_args)
        }
        Command::InstallIgnoreRules { target } => maintenance::run_install_ignore_rules(
            &project_path,
            json,
            privacy_mode,
            parse_ignore_install_target(&target)?,
        ),
        Command::DeleteIndex { yes } => {
            maintenance::run_delete_index(required_engine(engine)?, json, yes, privacy_mode)
        }
        Command::DbMaintenance {
            integrity_check,
            checkpoint,
            vacuum,
            analyze,
            stats,
            prune,
        } => maintenance::run_db_maintenance(
            required_engine(engine)?,
            json,
            privacy_mode,
            maintenance::DbMaintenanceArgs {
                integrity_check,
                checkpoint,
                vacuum,
                analyze,
                stats,
                prune,
            },
        ),
        Command::Preflight => {
            maintenance::run_preflight(required_engine(engine)?, json, privacy_mode)
        }
        Command::Status => maintenance::run_status(required_engine(engine)?, json, privacy_mode),
        Command::Search {
            query,
            limit,
            detailed,
            semantic,
            auto_index,
            semantic_fail_mode,
        } => query::run_search(
            required_engine(engine)?,
            json,
            query::SearchArgs {
                query,
                limit,
                detailed,
                semantic,
                auto_index,
                semantic_fail_mode: parse_semantic_fail_mode(&semantic_fail_mode)?,
                privacy_mode,
                vector_layer_enabled,
                rollout_phase,
            },
        ),
        Command::SemanticSearch {
            query,
            limit,
            auto_index,
            semantic_fail_mode,
        } => query::run_semantic_search(
            required_engine(engine)?,
            json,
            query::SemanticSearchArgs {
                query,
                limit,
                auto_index,
                semantic_fail_mode: parse_semantic_fail_mode(&semantic_fail_mode)?,
                privacy_mode,
                vector_layer_enabled,
                rollout_phase,
            },
        ),
        Command::SymbolLookup {
            name,
            limit,
            auto_index,
        } => query::run_symbol_lookup(
            required_engine(engine)?,
            json,
            name,
            limit,
            auto_index,
            privacy_mode,
        ),
        Command::SymbolReferences {
            name,
            limit,
            auto_index,
        } => query::run_symbol_references(
            required_engine(engine)?,
            json,
            name,
            limit,
            auto_index,
            privacy_mode,
        ),
        Command::SymbolBody {
            seed,
            seed_kind,
            limit,
            auto_index,
        } => query::run_symbol_body(
            required_engine(engine)?,
            json,
            query::InvestigationArgs {
                seed,
                seed_kind: parse_seed_kind(&seed_kind)?,
                limit,
                auto_index,
                privacy_mode,
            },
        ),
        Command::RelatedFiles {
            path,
            limit,
            auto_index,
        } => query::run_related_files(
            required_engine(engine)?,
            json,
            path,
            limit,
            auto_index,
            privacy_mode,
        ),
        Command::CallPath {
            from,
            to,
            max_hops,
            auto_index,
        } => query::run_call_path(
            required_engine(engine)?,
            json,
            from,
            to,
            max_hops,
            auto_index,
            privacy_mode,
        ),
        Command::RouteTrace {
            seed,
            seed_kind,
            limit,
            auto_index,
        } => query::run_route_trace(
            required_engine(engine)?,
            json,
            query::InvestigationArgs {
                seed,
                seed_kind: parse_seed_kind(&seed_kind)?,
                limit,
                auto_index,
                privacy_mode,
            },
        ),
        Command::ConstraintEvidence {
            seed,
            seed_kind,
            limit,
            auto_index,
        } => query::run_constraint_evidence(
            required_engine(engine)?,
            json,
            query::InvestigationArgs {
                seed,
                seed_kind: parse_seed_kind(&seed_kind)?,
                limit,
                auto_index,
                privacy_mode,
            },
        ),
        Command::ConceptCluster {
            seed,
            seed_kind,
            limit,
            auto_index,
        } => query::run_concept_cluster(
            required_engine(engine)?,
            json,
            query::InvestigationArgs {
                seed,
                seed_kind: parse_seed_kind(&seed_kind)?,
                limit,
                auto_index,
                privacy_mode,
            },
        ),
        Command::ContractTrace {
            seed,
            seed_kind,
            limit,
            auto_index,
        } => query::run_contract_trace(
            required_engine(engine)?,
            json,
            query::InvestigationArgs {
                seed,
                seed_kind: parse_seed_kind(&seed_kind)?,
                limit,
                auto_index,
                privacy_mode,
            },
        ),
        Command::DivergenceReport {
            seed,
            seed_kind,
            limit,
            auto_index,
        } => query::run_divergence_report(
            required_engine(engine)?,
            json,
            query::InvestigationArgs {
                seed,
                seed_kind: parse_seed_kind(&seed_kind)?,
                limit,
                auto_index,
                privacy_mode,
            },
        ),
        Command::InvestigationBenchmark {
            dataset,
            limit,
            auto_index,
            baseline_report,
            thresholds,
            enforce_gates,
        } => query::run_investigation_benchmark(
            required_engine(engine)?,
            json,
            query::InvestigationBenchmarkArgs {
                dataset,
                limit,
                auto_index,
                privacy_mode,
                baseline_report,
                thresholds,
                enforce_gates,
            },
        ),
        Command::Context {
            query,
            limit,
            semantic,
            auto_index,
            semantic_fail_mode,
            max_chars,
            max_tokens,
        } => query::run_context(
            required_engine(engine)?,
            json,
            query::ContextArgs {
                query,
                limit,
                semantic,
                auto_index,
                semantic_fail_mode: parse_semantic_fail_mode(&semantic_fail_mode)?,
                privacy_mode,
                vector_layer_enabled,
                rollout_phase,
                max_chars,
                max_tokens,
            },
        ),
        Command::ContextPack {
            query,
            mode,
            limit,
            semantic,
            auto_index,
            semantic_fail_mode,
            max_chars,
            max_tokens,
        } => query::run_context_pack(
            required_engine(engine)?,
            json,
            query::ContextPackArgs {
                query,
                mode: parse_context_mode(&mode)?,
                limit,
                semantic,
                auto_index,
                semantic_fail_mode: parse_semantic_fail_mode(&semantic_fail_mode)?,
                privacy_mode,
                vector_layer_enabled,
                rollout_phase,
                max_chars,
                max_tokens,
            },
        ),
        Command::Report {
            query,
            mode,
            limit,
            semantic,
            auto_index,
            semantic_fail_mode,
            max_chars,
            max_tokens,
        } => query::run_report(
            required_engine(engine)?,
            json,
            query::ReportArgs {
                query,
                mode: mode.as_deref().map(parse_agent_intent_mode).transpose()?,
                limit,
                semantic,
                auto_index,
                semantic_fail_mode: parse_semantic_fail_mode(&semantic_fail_mode)?,
                privacy_mode,
                vector_layer_enabled,
                rollout_phase,
                max_chars,
                max_tokens,
            },
        ),
        Command::QueryBenchmark {
            dataset,
            k,
            limit,
            semantic,
            auto_index,
            semantic_fail_mode,
            max_chars,
            max_tokens,
            baseline,
            thresholds,
            runs,
            enforce_gates,
        } => query::run_query_benchmark(
            required_engine(engine)?,
            json,
            query::QueryBenchmarkArgs {
                dataset,
                k,
                limit,
                semantic,
                auto_index,
                semantic_fail_mode: parse_semantic_fail_mode(&semantic_fail_mode)?,
                privacy_mode,
                vector_layer_enabled,
                rollout_phase,
                migration_mode,
                max_chars,
                max_tokens,
                baseline,
                thresholds,
                runs,
                enforce_gates,
            },
        ),
        Command::QualityMatrix {
            manifest,
            override_path,
            output_root,
            repo_ids,
        } => quality_matrix::run(
            &project_path,
            json,
            privacy_mode,
            migration_mode,
            quality_matrix::QualityMatrixArgs {
                manifest,
                override_path,
                output_root,
                repo_ids,
            },
        ),
        Command::QualityHotspots {
            aggregation,
            limit,
            path_prefix,
            language,
            rule_ids,
            sort_by,
            auto_index,
        } => quality_hotspots::run(
            required_engine(engine)?,
            json,
            privacy_mode,
            quality_hotspots::QualityHotspotsArgs {
                aggregation,
                limit,
                path_prefix,
                language,
                rule_ids,
                sort_by,
                auto_index,
            },
        ),
        Command::QualitySnapshot(args) => quality_snapshot::run(
            required_engine(engine)?,
            json,
            privacy_mode,
            quality_snapshot::QualitySnapshotArgs {
                snapshot_kind: args.snapshot_kind,
                wave_id: args.wave_id,
                output_root: args.output_root,
                compare_against: args.compare_against,
                auto_index: args.auto_index,
                persist_artifacts: args.persist_artifacts,
                promote_self_baseline: args.promote_self_baseline,
                fail_on_regression: args.fail_on_regression,
            },
        ),
        Command::SensitiveData {
            path_prefix,
            limit,
            include_low_confidence,
        } => sensitive_data::run(
            required_engine(engine)?,
            json,
            privacy_mode,
            sensitive_data::SensitiveDataArgs {
                path_prefix,
                limit,
                include_low_confidence,
            },
        ),
        Command::SignalMemory {
            limit,
            finding_family,
            decision,
        } => signal_memory::inspect(
            required_engine(engine)?,
            json,
            privacy_mode,
            signal_memory::SignalMemoryArgs {
                limit,
                finding_family,
                decision,
            },
        ),
        Command::MarkSignalMemory {
            signal_key,
            finding_family,
            decision,
            reason,
            source,
            scope,
        } => signal_memory::mark(
            required_engine(engine)?,
            json,
            privacy_mode,
            signal_memory::MarkSignalMemoryArgs {
                signal_key,
                finding_family,
                decision,
                reason,
                source,
                scope,
            },
        ),
        Command::Brief => maintenance::run_brief(required_engine(engine)?, json, privacy_mode),
        Command::Agent {
            query,
            mode,
            profile,
            limit,
            semantic,
            auto_index,
            semantic_fail_mode,
            max_chars,
            max_tokens,
        } => query::run_agent(
            required_engine(engine)?,
            json,
            query::AgentArgs {
                query,
                mode: mode.as_deref().map(parse_agent_intent_mode).transpose()?,
                profile: profile
                    .as_deref()
                    .map(parse_bootstrap_profile)
                    .transpose()?,
                limit,
                semantic,
                auto_index,
                semantic_fail_mode: parse_semantic_fail_mode(&semantic_fail_mode)?,
                privacy_mode,
                vector_layer_enabled,
                rollout_phase,
                max_chars,
                max_tokens,
            },
        ),
    }
}

fn required_engine(engine: Option<&rmu_core::Engine>) -> Result<&rmu_core::Engine> {
    engine.ok_or_else(|| anyhow::anyhow!("internal error: prepared run missing engine"))
}
