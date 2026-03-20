use anyhow::Result;

use crate::args::Command;

use super::modes::{parse_context_mode, parse_semantic_fail_mode};
use super::preflight_helpers::PreparedRun;
use super::{indexing, maintenance, query};

pub(super) fn run(prepared: PreparedRun) -> Result<()> {
    let PreparedRun {
        engine,
        json,
        privacy_mode,
        vector_layer_enabled,
        rollout_phase,
        migration_mode,
        command,
    } = prepared;

    match command {
        Command::Index(index_args) => indexing::run_index(&engine, json, index_args),
        Command::SemanticIndex(index_args) => {
            indexing::run_semantic_index(&engine, json, index_args)
        }
        Command::ScopePreview(index_args) => {
            indexing::run_scope_preview(&engine, json, privacy_mode, index_args)
        }
        Command::DeleteIndex { yes } => {
            maintenance::run_delete_index(&engine, json, yes, privacy_mode)
        }
        Command::DbMaintenance {
            integrity_check,
            checkpoint,
            vacuum,
            analyze,
            stats,
            prune,
        } => maintenance::run_db_maintenance(
            &engine,
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
        Command::Status => maintenance::run_status(&engine, json, privacy_mode),
        Command::Search {
            query,
            limit,
            detailed,
            semantic,
            auto_index,
            semantic_fail_mode,
        } => query::run_search(
            &engine,
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
            &engine,
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
        } => query::run_symbol_lookup(&engine, json, name, limit, auto_index, privacy_mode),
        Command::SymbolReferences {
            name,
            limit,
            auto_index,
        } => query::run_symbol_references(&engine, json, name, limit, auto_index, privacy_mode),
        Command::RelatedFiles {
            path,
            limit,
            auto_index,
        } => query::run_related_files(&engine, json, path, limit, auto_index, privacy_mode),
        Command::CallPath {
            from,
            to,
            max_hops,
            auto_index,
        } => query::run_call_path(&engine, json, from, to, max_hops, auto_index, privacy_mode),
        Command::Context {
            query,
            limit,
            semantic,
            auto_index,
            semantic_fail_mode,
            max_chars,
            max_tokens,
        } => query::run_context(
            &engine,
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
            &engine,
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
            &engine,
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
            &engine,
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
        Command::Brief => maintenance::run_brief(&engine, json, privacy_mode),
        Command::Agent {
            query,
            limit,
            semantic,
            auto_index,
            semantic_fail_mode,
            max_chars,
            max_tokens,
        } => query::run_agent(
            &engine,
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
