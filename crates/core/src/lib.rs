mod artifact_fingerprint;
mod benchmark_compare;
mod db_store;
mod default_index_profile;
mod embedding_backend;
pub mod engine;
mod engine_brief;
mod engine_quality;
mod engine_status;
mod graph;
mod index_scope;
mod index_scope_meta;
pub mod model;
mod privacy;
mod quality;
mod query_profile;
mod rebuild_lock;
mod report;
mod rollout;
mod search_db;
mod security;
mod signal_memory;
mod text_utils;
mod utils;
mod vector_rank;

pub use benchmark_compare::{
    BenchmarkMetrics, GateEvaluation, ThresholdConfig, build_benchmark_diff_payload,
    build_metrics_diff, load_baseline_metrics, load_thresholds, median_metrics_from_runs,
};
pub use engine::{
    Engine, IndexSummary, ThreadRunningBinaryTimestampsOverrideGuard,
    set_thread_running_binary_timestamps_override_for_tests,
};
pub use model::{
    AgentBootstrap, AgentBootstrapIncludeOptions, AgentBootstrapTimings, AgentIntentMode,
    AgentQueryBundle, AxisObservation, BootstrapProfile, BudgetInfo, CallPathEndpoint,
    CallPathExplain, CallPathResult, CallPathStep, CanonicalBasis, CanonicalFreshness,
    CanonicalProvenance, CanonicalStrength, ConceptClusterResult, ConceptClusterSummary,
    ConceptSeed, ConceptSeedKind, ConfidenceInfo, ConfidenceSignals, ConstraintEvidence,
    ConstraintEvidenceResult, ContextFile, ContextMode, ContextPackResult, ContextSelection,
    DbCheckpointResult, DbMaintenanceOptions, DbMaintenanceResult, DbMaintenanceStats,
    DbPruneResult, DegradationReason, DeleteIndexResult, DivergenceAxis, DivergenceReport,
    DivergenceSignal, FindingConfidence, FindingFamily, IgnoreInstallReport, IgnoreInstallTarget,
    ImplementationVariant, IndexProfile, IndexStatus, IndexTelemetry, IndexingOptions,
    InvestigationAnchor, InvestigationAnchorLabel, InvestigationAssertion,
    InvestigationBenchmarkCase, InvestigationBenchmarkDataset, InvestigationBenchmarkDiffReport,
    InvestigationBenchmarkReport, InvestigationBenchmarkTool, InvestigationCaseLabels,
    InvestigationCaseReport, InvestigationConceptClusterSummary, InvestigationConstraintLabel,
    InvestigationConstraintSummary, InvestigationDivergenceSignalLabel,
    InvestigationDivergenceSummary, InvestigationHints, InvestigationMetricChange,
    InvestigationRouteSegmentLabel, InvestigationRouteSummary, InvestigationSummary,
    InvestigationThresholdVerdict, InvestigationThresholds, InvestigationToolMetricDelta,
    InvestigationToolMetrics, InvestigationTopVariant, MigrationMode, ModeResolutionSource,
    PreflightState, PreflightStatus, PrivacyMode, QualityDeltaSummary, QualityHotspotAggregation,
    QualityHotspotBucket, QualityHotspotRuleCount, QualityHotspotStructuralSignals,
    QualityHotspotsOptions, QualityHotspotsResult, QualityHotspotsSortBy, QualityHotspotsSummary,
    QualityMode, QualityProjectArtifactPaths, QualityProjectDeltaReport, QualityProjectGateStatus,
    QualityProjectHotspotDelta, QualityProjectSnapshotCapture,
    QualityProjectSnapshotCompareAgainst, QualityProjectSnapshotKind,
    QualityProjectSnapshotOptions, QualityProjectSnapshotReport, QualityProjectTopHotFiles,
    QualityProjectTopHotspotBuckets, QualitySource, QualityStatus, QueryBenchmarkCase,
    QueryBenchmarkDataset, QueryBenchmarkOptions, QueryBenchmarkReport, QueryOptions, QueryQrel,
    QueryReport, RankExplainBreakdown, RelatedFileHit, RetrievalStage, RolloutPhase, RouteGap,
    RoutePath, RouteSegment, RouteSegmentKind, RouteTraceResult, RuleViolationFileHit,
    RuleViolationsOptions, RuleViolationsResult, RuleViolationsSortBy, RuleViolationsSummary,
    ScopePreviewResult, SearchHit, SelectedContextItem, SemanticFailMode,
    SensitiveDataExposureScope, SensitiveDataFinding, SensitiveDataOptions,
    SensitiveDataPlaceholderStatus, SensitiveDataResult, SensitiveDataRotationUrgency,
    SensitiveDataSnippetType, SensitiveDataSummary, SensitiveDataValidationStatus,
    SignalMemoryDecision, SignalMemoryEntry, SignalMemoryMarkRequest, SignalMemoryOptions,
    SignalMemoryResult, SignalMemoryStatus, SourceSpan, SymbolBodyAmbiguityStatus, SymbolBodyItem,
    SymbolBodyResolutionKind, SymbolBodyResult, SymbolBodyTimings, SymbolMatch, SymbolReferenceHit,
    WorkspaceBrief, WorkspaceLanguageStat, WorkspaceQualitySummary, WorkspaceQualityTopRule,
    WorkspaceTopSymbol,
};
pub use privacy::{
    sanitize_error_message, sanitize_path_text, sanitize_query_text, sanitize_value_for_privacy,
};
pub use rollout::{
    RollbackLevel, RollbackRecommendation, RollbackSignals, RolloutDecision,
    decide_semantic_rollout, recommend_rollback, stable_cycles_observed,
};
pub use utils::{
    GitignoreUpdate, ProjectIgnoreMatcher, ensure_root_gitignore, install_ignore_rules,
};
