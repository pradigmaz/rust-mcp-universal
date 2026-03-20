mod artifact_fingerprint;
mod benchmark_compare;
mod db_store;
mod embedding_backend;
pub mod engine;
mod engine_brief;
mod engine_status;
mod graph;
mod index_scope;
pub mod model;
mod privacy;
mod query_profile;
mod rebuild_lock;
mod report;
mod rollout;
mod search_db;
mod text_utils;
mod utils;
mod vector_rank;

pub use benchmark_compare::{
    BenchmarkMetrics, GateEvaluation, ThresholdConfig, build_benchmark_diff_payload,
    build_metrics_diff, load_baseline_metrics, load_thresholds, median_metrics_from_runs,
};
pub use engine::{Engine, IndexSummary};
pub use model::{
    AgentBootstrap, AgentQueryBundle, BudgetInfo, CallPathEndpoint, CallPathExplain,
    CallPathResult, CallPathStep, ConfidenceInfo, ConfidenceSignals, ContextFile, ContextMode,
    ContextPackResult, ContextSelection, DbCheckpointResult, DbMaintenanceOptions,
    DbMaintenanceResult, DbMaintenanceStats, DbPruneResult, DeleteIndexResult, IndexProfile,
    IndexStatus, IndexTelemetry, IndexingOptions, MigrationMode, PrivacyMode, QueryBenchmarkCase,
    QueryBenchmarkDataset, QueryBenchmarkOptions, QueryBenchmarkReport, QueryOptions, QueryQrel,
    QueryReport, RankExplainBreakdown, RelatedFileHit, RetrievalStage, RolloutPhase,
    ScopePreviewResult, SearchHit, SelectedContextItem, SemanticFailMode, SymbolMatch,
    SymbolReferenceHit, WorkspaceBrief, WorkspaceLanguageStat, WorkspaceTopSymbol,
};
pub use privacy::{
    sanitize_error_message, sanitize_path_text, sanitize_query_text, sanitize_value_for_privacy,
};
pub use rollout::{
    RollbackLevel, RollbackRecommendation, RollbackSignals, RolloutDecision,
    decide_semantic_rollout, recommend_rollback, stable_cycles_observed,
};
