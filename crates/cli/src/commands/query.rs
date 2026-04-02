use anyhow::{Result, anyhow};
use rmu_core::{
    AgentIntentMode, BootstrapProfile, ConceptSeedKind, ContextMode, Engine, IndexProfile,
    IndexingOptions, MigrationMode, PrivacyMode, QueryBenchmarkOptions, QueryOptions,
    RolloutPhase, SemanticFailMode, decide_semantic_rollout, sanitize_path_text,
    sanitize_query_text, sanitize_value_for_privacy,
};
use std::path::PathBuf;

use crate::error::{CODE_INDEX_NOT_READY, cli_error};
use crate::output::{print_json, print_line};
use crate::validation::require_min;

#[path = "query/agent.rs"]
mod agent;
#[path = "query/benchmark.rs"]
mod benchmark;
#[path = "query/context.rs"]
mod context;
#[path = "query/investigation.rs"]
mod investigation;
#[path = "query/investigation_benchmark.rs"]
mod investigation_benchmark;
#[path = "query/investigation_benchmark_cluster_metrics.rs"]
mod investigation_benchmark_cluster_metrics;
#[path = "query/investigation_benchmark_compare.rs"]
mod investigation_benchmark_compare;
#[path = "query/investigation_benchmark_constraint_metrics.rs"]
mod investigation_benchmark_constraint_metrics;
#[path = "query/investigation_benchmark_eval.rs"]
mod investigation_benchmark_eval;
#[path = "query/investigation_benchmark_metrics.rs"]
mod investigation_benchmark_metrics;
#[path = "query/investigation_benchmark_route_metrics.rs"]
mod investigation_benchmark_route_metrics;
#[path = "query/navigation.rs"]
mod navigation;
#[path = "query/report.rs"]
mod report;
#[path = "query/search.rs"]
mod search;
#[path = "query/semantic_search.rs"]
mod semantic_search;

pub(crate) use agent::run_agent;
pub(crate) use benchmark::run_query_benchmark;
pub(crate) use context::{run_context, run_context_pack};
pub(crate) use investigation::{
    run_concept_cluster, run_constraint_evidence, run_divergence_report, run_route_trace,
    run_symbol_body,
};
pub(crate) use investigation_benchmark::run_investigation_benchmark;
pub(crate) use navigation::{
    run_call_path, run_related_files, run_symbol_lookup, run_symbol_references,
};
pub(crate) use report::run_report;
pub(crate) use search::run_search;
pub(crate) use semantic_search::run_semantic_search;

pub(crate) struct AgentArgs {
    pub(crate) query: Option<String>,
    pub(crate) mode: Option<AgentIntentMode>,
    pub(crate) profile: Option<BootstrapProfile>,
    pub(crate) limit: usize,
    pub(crate) semantic: bool,
    pub(crate) auto_index: bool,
    pub(crate) semantic_fail_mode: SemanticFailMode,
    pub(crate) privacy_mode: PrivacyMode,
    pub(crate) vector_layer_enabled: bool,
    pub(crate) rollout_phase: RolloutPhase,
    pub(crate) max_chars: usize,
    pub(crate) max_tokens: usize,
}

pub(crate) struct QueryBenchmarkArgs {
    pub(crate) dataset: PathBuf,
    pub(crate) k: usize,
    pub(crate) limit: usize,
    pub(crate) semantic: bool,
    pub(crate) auto_index: bool,
    pub(crate) semantic_fail_mode: SemanticFailMode,
    pub(crate) privacy_mode: PrivacyMode,
    pub(crate) vector_layer_enabled: bool,
    pub(crate) rollout_phase: RolloutPhase,
    pub(crate) migration_mode: MigrationMode,
    pub(crate) max_chars: usize,
    pub(crate) max_tokens: usize,
    pub(crate) baseline: Option<PathBuf>,
    pub(crate) thresholds: Option<PathBuf>,
    pub(crate) runs: usize,
    pub(crate) enforce_gates: bool,
}

pub(crate) struct InvestigationArgs {
    pub(crate) seed: String,
    pub(crate) seed_kind: ConceptSeedKind,
    pub(crate) limit: usize,
    pub(crate) auto_index: bool,
    pub(crate) privacy_mode: PrivacyMode,
}

pub(crate) struct InvestigationBenchmarkArgs {
    pub(crate) dataset: PathBuf,
    pub(crate) limit: usize,
    pub(crate) auto_index: bool,
    pub(crate) privacy_mode: PrivacyMode,
    pub(crate) baseline_report: Option<PathBuf>,
    pub(crate) thresholds: Option<PathBuf>,
    pub(crate) enforce_gates: bool,
}

pub(crate) struct ContextArgs {
    pub(crate) query: String,
    pub(crate) limit: usize,
    pub(crate) semantic: bool,
    pub(crate) auto_index: bool,
    pub(crate) semantic_fail_mode: SemanticFailMode,
    pub(crate) privacy_mode: PrivacyMode,
    pub(crate) vector_layer_enabled: bool,
    pub(crate) rollout_phase: RolloutPhase,
    pub(crate) max_chars: usize,
    pub(crate) max_tokens: usize,
}

pub(crate) struct ContextPackArgs {
    pub(crate) query: String,
    pub(crate) mode: rmu_core::ContextMode,
    pub(crate) limit: usize,
    pub(crate) semantic: bool,
    pub(crate) auto_index: bool,
    pub(crate) semantic_fail_mode: SemanticFailMode,
    pub(crate) privacy_mode: PrivacyMode,
    pub(crate) vector_layer_enabled: bool,
    pub(crate) rollout_phase: RolloutPhase,
    pub(crate) max_chars: usize,
    pub(crate) max_tokens: usize,
}

pub(crate) struct ReportArgs {
    pub(crate) query: String,
    pub(crate) mode: Option<AgentIntentMode>,
    pub(crate) limit: usize,
    pub(crate) semantic: bool,
    pub(crate) auto_index: bool,
    pub(crate) semantic_fail_mode: SemanticFailMode,
    pub(crate) privacy_mode: PrivacyMode,
    pub(crate) vector_layer_enabled: bool,
    pub(crate) rollout_phase: RolloutPhase,
    pub(crate) max_chars: usize,
    pub(crate) max_tokens: usize,
}

pub(crate) struct SearchArgs {
    pub(crate) query: String,
    pub(crate) limit: usize,
    pub(crate) detailed: bool,
    pub(crate) semantic: bool,
    pub(crate) auto_index: bool,
    pub(crate) semantic_fail_mode: SemanticFailMode,
    pub(crate) privacy_mode: PrivacyMode,
    pub(crate) vector_layer_enabled: bool,
    pub(crate) rollout_phase: RolloutPhase,
}

pub(crate) struct SemanticSearchArgs {
    pub(crate) query: String,
    pub(crate) limit: usize,
    pub(crate) auto_index: bool,
    pub(crate) semantic_fail_mode: SemanticFailMode,
    pub(crate) privacy_mode: PrivacyMode,
    pub(crate) vector_layer_enabled: bool,
    pub(crate) rollout_phase: RolloutPhase,
}

fn ensure_query_index_ready(engine: &Engine, auto_index: bool) -> Result<()> {
    if auto_index {
        let _ = engine.ensure_index_ready_with_policy(true)?;
        return Ok(());
    }
    let status = engine.index_status()?;
    if status.files == 0 {
        return Err(cli_error(
            CODE_INDEX_NOT_READY,
            "index is empty; run `rmu index` or `rmu semantic-index` first",
        ));
    }
    let _ = engine.ensure_index_ready_with_policy(false)?;
    Ok(())
}

fn ensure_context_pack_index_ready(
    engine: &Engine,
    mode: ContextMode,
    auto_index: bool,
) -> Result<()> {
    if auto_index && matches!(mode, ContextMode::Design) && engine.index_status()?.files == 0 {
        let _ = engine.index_path_with_options(&IndexingOptions {
            profile: Some(IndexProfile::DocsHeavy),
            reindex: true,
            ..IndexingOptions::default()
        })?;
        return Ok(());
    }

    ensure_query_index_ready(engine, auto_index)
}

#[cfg(test)]
mod tests {
    use super::ensure_query_index_ready;
    use rmu_core::Engine;
    use std::error::Error;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    #[test]
    fn ensure_query_index_ready_respects_auto_index_flag() -> Result<(), Box<dyn Error>> {
        let project_dir = temp_project_dir("rmu-cli-query-auto-index");
        fs::create_dir_all(project_dir.join("src"))?;
        fs::write(
            project_dir.join("src/main.rs"),
            "fn cli_auto_index_smoke() { println!(\"ok\"); }\n",
        )?;

        let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
        let err =
            ensure_query_index_ready(&engine, false).expect_err("auto_index=false must reject");
        assert!(err.to_string().contains("index is empty"));

        ensure_query_index_ready(&engine, true)?;
        let status = engine.index_status()?;
        assert!(status.files >= 1);

        let _ = fs::remove_dir_all(project_dir);
        Ok(())
    }

    #[test]
    fn ensure_query_index_ready_auto_indexes_rust_workspace_with_rust_monorepo()
    -> Result<(), Box<dyn Error>> {
        let project_dir = temp_project_dir("rmu-cli-query-rust-default-profile");
        fs::create_dir_all(project_dir.join("src"))?;
        fs::create_dir_all(project_dir.join("docs"))?;
        fs::write(
            project_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )?;
        fs::write(
            project_dir.join("src/lib.rs"),
            "pub fn cli_rust_default_profile() {}\n",
        )?;
        fs::write(
            project_dir.join("docs/guide.md"),
            "cli docs should stay out of the default rust scope\n",
        )?;

        let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
        ensure_query_index_ready(&engine, true)?;

        let status = engine.index_status()?;
        assert_eq!(status.files, 2);

        let _ = fs::remove_dir_all(project_dir);
        Ok(())
    }
}
