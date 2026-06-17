use serde::{Deserialize, Serialize};

use crate::bootstrap::NormalizedStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyMode {
    #[default]
    Off,
    Mask,
    Hash,
}

impl PrivacyMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(Self::Off),
            "mask" => Some(Self::Mask),
            "hash" => Some(Self::Hash),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageMode {
    #[default]
    Codex,
    Project,
}

impl StorageMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "codex" => Some(Self::Codex),
            "project" => Some(Self::Project),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Project => "project",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightState {
    Ok,
    Warning,
    Incompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightStatus {
    pub status: PreflightState,
    pub project_path: String,
    pub project_root: String,
    pub memory_root: String,
    pub storage_mode: StorageMode,
    pub binary_path: String,
    pub running_binary_version: String,
    pub running_binary_stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_schema_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_schema_version: Option<u32>,
    #[serde(default)]
    pub same_binary_other_pids: Vec<u32>,
    pub stale_process_suspected: bool,
    pub safe_recovery_hint: String,
    #[serde(default)]
    pub missing_canonical_paths: Vec<String>,
    pub legacy_root_layout_detected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_root_path: Option<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexedCounts {
    pub notes: usize,
    pub decisions: usize,
    pub risks: usize,
    pub constraints: usize,
    pub artifacts: usize,
    pub relations: usize,
    pub aliases: usize,
    pub tags: usize,
    pub observations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInconsistencies {
    #[serde(default)]
    pub missing_from_index: Vec<String>,
    #[serde(default)]
    pub orphaned_in_index: Vec<String>,
    #[serde(default)]
    pub stale_fingerprints: Vec<String>,
    #[serde(default)]
    pub parse_failures: Vec<String>,
}

impl IndexInconsistencies {
    pub fn is_empty(&self) -> bool {
        self.missing_from_index.is_empty()
            && self.orphaned_in_index.is_empty()
            && self.stale_fingerprints.is_empty()
            && self.parse_failures.is_empty()
    }

    pub fn total_items(&self) -> usize {
        self.missing_from_index.len()
            + self.orphaned_in_index.len()
            + self.stale_fingerprints.len()
            + self.parse_failures.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatus {
    pub project: String,
    pub project_root: String,
    pub memory_root: String,
    pub storage_mode: StorageMode,
    pub db_path: String,
    pub schema_version: Option<u32>,
    pub indexed: bool,
    pub counts: IndexedCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync: Option<String>,
    pub drift_detected: bool,
    pub fingerprint_drift_detected: bool,
    pub pending_markdown_files: usize,
    pub inconsistencies: IndexInconsistencies,
    #[serde(default)]
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatus {
    pub project: String,
    pub project_root: String,
    pub memory_root: String,
    pub storage_mode: StorageMode,
    pub health: String,
    pub counts: IndexedCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync: Option<String>,
    pub drift_detected: bool,
    pub parser_health: String,
    pub index_health: String,
    #[serde(default)]
    pub issues: Vec<String>,
    pub recommended_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBindingResult {
    pub project: String,
    pub project_root: String,
    pub memory_root: String,
    pub storage_mode: StorageMode,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSourceCandidate {
    pub root: String,
    #[serde(default)]
    pub canonical_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrateMemoryRootResult {
    pub dry_run: bool,
    pub target_storage_mode: StorageMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_root: Option<String>,
    pub destination_root: String,
    #[serde(default)]
    pub canonical_paths: Vec<String>,
    #[serde(default)]
    pub candidate_sources: Vec<MigrationSourceCandidate>,
    pub destination_exists: bool,
    pub destination_has_memory: bool,
    pub migrated: bool,
    pub rebuilt: bool,
    #[serde(default)]
    pub deleted_source_paths: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBrief {
    pub project: String,
    pub summary: String,
    pub top_decisions: Vec<NodeSummary>,
    pub top_risks: Vec<NodeSummary>,
    pub recent_changes: Vec<NodeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub node_type: String,
    pub status: String,
    pub file_path: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_status: Option<NormalizedStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateCandidate {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub node_type: String,
    pub status: String,
    pub summary: String,
    pub why_matched: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub node_type: String,
    pub status: String,
    pub file_path: String,
    pub summary: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDetails {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub node_type: String,
    pub status: String,
    pub project: String,
    pub file_path: String,
    pub summary: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub observations: Vec<String>,
    #[serde(default)]
    pub relations: Vec<NodeRelation>,
    #[serde(default)]
    pub references: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRelation {
    pub source_slug: String,
    pub target_slug: String,
    pub relation_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphResult {
    pub nodes: Vec<NodeDetails>,
    pub relations: Vec<NodeRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildIndexResult {
    pub rebuilt: bool,
    pub indexed_files: usize,
    pub counts: IndexedCounts,
    pub duration_ms: u128,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeWriteRef {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub node_type: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNodeInput {
    pub node_type: String,
    pub title: String,
    pub slug: Option<String>,
    pub status: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNodeInput {
    pub node: String,
    pub title: Option<String>,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub tags: Option<Vec<String>>,
    pub aliases: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddObservationInput {
    pub node: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkNodesInput {
    pub source: String,
    pub target: String,
    pub relation_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNodeResult {
    pub node: NodeWriteRef,
    pub sync_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNodeResult {
    pub node: NodeWriteRef,
    pub changed: bool,
    pub sync_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationWriteResult {
    pub node: NodeWriteRef,
    pub added: bool,
    pub sync_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkNodesResult {
    pub source: NodeWriteRef,
    pub target: NodeWriteRef,
    pub relation_kind: String,
    pub changed: bool,
    pub sync_status: String,
}
