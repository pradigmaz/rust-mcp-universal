pub mod bootstrap;
pub mod context;
pub mod engine;
pub mod model;
mod privacy;
mod utils;

pub use bootstrap::{
    ContextPack, ContextPackBudget, ContextPackNode, DecisionLogEntry, NormalizedStatus,
    RecentChangeItem, RiskHotspotItem, RiskHotspots, normalize_status,
};
pub use context::ProjectContext;
pub use engine::{
    CURRENT_SCHEMA_VERSION, Engine, StateFailure, WriteFailure, as_state_failure, as_write_failure,
};
pub use model::{
    AddObservationInput, CreateNodeInput, CreateNodeResult, DuplicateCandidate, GraphResult,
    IndexInconsistencies, IndexStatus, IndexedCounts, LinkNodesInput, LinkNodesResult,
    MemoryStatus, MigrateMemoryRootResult, MigrationSourceCandidate, NodeDetails, NodeRelation,
    NodeSummary, NodeWriteRef, ObservationWriteResult, PreflightState, PreflightStatus,
    PrivacyMode, ProjectBindingResult, ProjectBrief, RebuildIndexResult, SearchHit, StorageMode,
    UpdateNodeInput, UpdateNodeResult,
};
pub use privacy::{
    sanitize_error_message, sanitize_path_text, sanitize_query_text, sanitize_value_for_privacy,
};
