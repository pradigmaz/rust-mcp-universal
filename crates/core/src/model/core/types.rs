mod agent;
mod context;
mod index;
mod navigation;
mod query;
mod report;
mod workspace;

pub use agent::{AgentBootstrap, AgentQueryBundle};
pub use context::{ContextFile, ContextPackResult, ContextSelection};
pub use index::{
    DbCheckpointResult, DbMaintenanceOptions, DbMaintenanceResult, DbMaintenanceStats,
    DbPruneResult, DeleteIndexResult, IndexProfile, IndexStatus, IndexingOptions,
    ScopePreviewResult,
};
pub use navigation::{
    CallPathEndpoint, CallPathExplain, CallPathResult, CallPathStep, RelatedFileHit, SymbolMatch,
    SymbolReferenceHit,
};
pub use query::{QueryOptions, SearchHit};
pub use report::{
    BudgetInfo, ConfidenceInfo, ConfidenceSignals, IndexTelemetry, QueryReport,
    RankExplainBreakdown, RetrievalStage, SelectedContextItem,
};
pub use workspace::{WorkspaceBrief, WorkspaceLanguageStat, WorkspaceTopSymbol};
