use anyhow::Result;

use crate::args::Command;

use super::modes::{parse_context_mode, parse_ignore_install_target, parse_semantic_fail_mode};
use super::preflight_helpers::PreparedRun;
use super::{indexing, maintenance, query};

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
        Command::Brief => maintenance::run_brief(required_engine(engine)?, json, privacy_mode),
        Command::Agent {
            query,
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
