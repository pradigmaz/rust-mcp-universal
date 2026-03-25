mod agent;
mod context;
mod index;
mod investigation;
mod investigation_cluster;
mod investigation_embed;
mod navigation;
mod preflight;
mod quality;
mod query;
mod report;
mod workspace;

pub use agent::{AgentBootstrap, AgentQueryBundle};
pub use context::{ContextFile, ContextPackResult, ContextSelection};
pub use index::{
    DbCheckpointResult, DbMaintenanceOptions, DbMaintenanceResult, DbMaintenanceStats,
    DbPruneResult, DeleteIndexResult, IgnoreInstallReport, IgnoreInstallTarget, IndexProfile,
    IndexStatus, IndexingOptions, ScopePreviewResult,
};
pub use investigation::{
    AxisObservation, ConceptSeed, ConceptSeedKind, ConstraintEvidence, ConstraintEvidenceResult,
    DivergenceAxis, DivergenceReport, DivergenceSignal, InvestigationAnchor, RouteGap, RoutePath,
    RouteSegment, RouteSegmentKind, RouteTraceResult, SourceSpan, SymbolBodyAmbiguityStatus,
    SymbolBodyItem, SymbolBodyResolutionKind, SymbolBodyResult,
};
pub use investigation_cluster::{
    ConceptClusterExpansionPolicy, ConceptClusterResult, ConceptClusterSummary,
    ImplementationVariant, SemanticState, VariantScoreBreakdown,
};
pub use investigation_embed::{
    InvestigationConceptClusterSummary, InvestigationConstraintSummary,
    InvestigationDivergenceSummary, InvestigationHints, InvestigationRouteSummary,
    InvestigationSummary, InvestigationTopVariant,
};
pub use navigation::{
    CallPathEndpoint, CallPathExplain, CallPathResult, CallPathStep, RelatedFileHit, SymbolMatch,
    SymbolReferenceHit,
};
pub use preflight::{PreflightState, PreflightStatus};
pub use quality::{
    QualityCategory, QualityDeltaSummary, QualityHotspotAggregation, QualityHotspotBucket,
    QualityHotspotRuleCount, QualityHotspotStructuralSignals, QualityHotspotsOptions,
    QualityHotspotsResult, QualityHotspotsSortBy, QualityHotspotsSummary, QualityLocation,
    QualityMetricValue, QualityMode, QualityRiskScoreBreakdown, QualityRiskScoreComponents,
    QualityRiskScoreWeights, QualitySeverity, QualitySource, QualityStatus, QualitySuppression,
    QualityViolationEntry, RuleViolationFileHit, RuleViolationsOptions, RuleViolationsResult,
    RuleViolationsSortBy, RuleViolationsSummary, SuppressedQualityViolationEntry,
    WorkspaceQualityCategoryCount, WorkspaceQualitySeverityCount, WorkspaceQualitySummary,
    WorkspaceQualityTopMetric, WorkspaceQualityTopRule,
};
pub use query::{QueryOptions, SearchHit};
pub use report::{
    BudgetInfo, ConfidenceInfo, ConfidenceSignals, IndexTelemetry, QueryReport,
    RankExplainBreakdown, RetrievalStage, SelectedContextItem,
};
pub use workspace::{
    WorkspaceBrief, WorkspaceLanguageStat, WorkspaceRepairHint, WorkspaceTopSymbol,
};
