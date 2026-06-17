mod bootstrap;
mod canonical;
mod diagnostics;
mod graph_hubs;
mod indexing;
mod migration;
mod notes;
mod preflight;
mod query;
mod schema;
mod write;

use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::Connection;
use serde_json::{Value, json};

use crate::bootstrap::{ContextPack, DecisionLogEntry, RecentChangeItem, RiskHotspots};
use crate::context::ProjectContext;
use crate::model::{
    AddObservationInput, CreateNodeInput, CreateNodeResult, GraphResult, IndexStatus,
    LinkNodesInput, LinkNodesResult, MemoryStatus, MigrateMemoryRootResult, NodeDetails,
    ObservationWriteResult, PreflightStatus, ProjectBrief, RebuildIndexResult, SearchHit,
    StorageMode, UpdateNodeInput, UpdateNodeResult,
};
pub use write::{WriteFailure, as_write_failure};

pub const CURRENT_SCHEMA_VERSION: u32 = 2;
const DEFAULT_DB_DIR: &str = ".derived";
const DEFAULT_DB_NAME: &str = "index.db";

#[derive(Debug, Clone)]
pub struct Engine {
    pub context: ProjectContext,
    pub project_root: PathBuf,
    pub memory_root: PathBuf,
    pub storage_mode: StorageMode,
    pub db_path: PathBuf,
}

impl Engine {
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        Self::new_with_mode(project_root, StorageMode::Codex)
    }

    pub fn new_with_mode(
        project_root: impl AsRef<Path>,
        storage_mode: StorageMode,
    ) -> Result<Self> {
        let context = ProjectContext::resolve(project_root, storage_mode)?;
        Self::from_context(context)
    }

    pub fn from_context(context: ProjectContext) -> Result<Self> {
        let db_path = default_db_path(&context.memory_root);
        Ok(Self {
            project_root: context.project_root.clone(),
            memory_root: context.memory_root.clone(),
            storage_mode: context.storage_mode,
            context,
            db_path,
        })
    }

    pub fn ensure_index_ready(&self, auto_index: bool) -> Result<()> {
        if !self.db_path.exists() {
            if auto_index {
                let rebuild = self.rebuild_index()?;
                if rebuild.errors.is_empty() {
                    return Ok(());
                }
            }
            return Err(StateFailure::new(
                "E_REBUILD_REQUIRED",
                "derived index is missing; run rebuild_index before using project memory",
                json!({
                    "kind": "derived_state",
                    "db_path": self.db_path.display().to_string(),
                    "safe_recovery_hint": "run rebuild_index to build the SQLite derived index from Markdown truth"
                }),
            )
            .into());
        }

        let status = diagnostics::index_status(self)?;
        if !status.drift_detected && status.failures.is_empty() {
            return Ok(());
        }
        if auto_index {
            let rebuild = self.rebuild_index()?;
            if rebuild.errors.is_empty() {
                let refreshed = diagnostics::index_status(self)?;
                if !refreshed.drift_detected && refreshed.failures.is_empty() {
                    return Ok(());
                }
                return Err(stale_index_failure(&refreshed).into());
            }
        }
        Err(stale_index_failure(&status).into())
    }

    pub fn rebuild_index(&self) -> Result<RebuildIndexResult> {
        indexing::rebuild_index(self)
    }

    pub fn sync_after_write(&self) -> Result<RebuildIndexResult> {
        write::sync_after_write(self)
    }

    pub fn index_status(&self) -> Result<IndexStatus> {
        diagnostics::index_status(self)
    }

    pub fn memory_status(&self) -> Result<MemoryStatus> {
        diagnostics::memory_status(self)
    }

    pub fn project_brief(&self) -> Result<ProjectBrief> {
        bootstrap::project_brief(self)
    }

    pub fn recent_changes(&self, limit: usize) -> Result<Vec<RecentChangeItem>> {
        bootstrap::recent_changes(self, limit)
    }

    pub fn decision_log(&self, topic: Option<&str>, limit: usize) -> Result<Vec<DecisionLogEntry>> {
        bootstrap::decision_log(self, topic, limit)
    }

    pub fn risk_hotspots(&self, limit: usize) -> Result<RiskHotspots> {
        bootstrap::risk_hotspots(self, limit)
    }

    pub fn context_pack(
        &self,
        seed: &str,
        limit: usize,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<ContextPack> {
        bootstrap::context_pack(self, seed, limit, max_chars, max_tokens)
    }

    pub fn search_memory(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
        query::search_memory(self, query, limit)
    }

    pub fn open_nodes(&self, slugs: &[String]) -> Result<Vec<NodeDetails>> {
        query::open_nodes(self, slugs)
    }

    pub fn read_graph(&self, slugs: &[String]) -> Result<GraphResult> {
        query::read_graph(self, slugs)
    }

    pub fn preflight_status(&self) -> Result<PreflightStatus> {
        preflight::preflight_status(self)
    }

    pub fn migrate_memory_root(
        &self,
        target_storage_mode: StorageMode,
        dry_run: bool,
        source_root: Option<PathBuf>,
    ) -> Result<MigrateMemoryRootResult> {
        migration::migrate_memory_root(self, target_storage_mode, dry_run, source_root)
    }

    pub fn create_node(
        &self,
        input: CreateNodeInput,
    ) -> std::result::Result<CreateNodeResult, WriteFailure> {
        write::create_node(self, input)
    }

    pub fn add_observation(
        &self,
        input: AddObservationInput,
    ) -> std::result::Result<ObservationWriteResult, WriteFailure> {
        write::add_observation(self, input)
    }

    pub fn link_nodes(
        &self,
        input: LinkNodesInput,
    ) -> std::result::Result<LinkNodesResult, WriteFailure> {
        write::link_nodes(self, input)
    }

    pub fn unlink_nodes(
        &self,
        input: LinkNodesInput,
    ) -> std::result::Result<LinkNodesResult, WriteFailure> {
        write::unlink_nodes(self, input)
    }

    pub fn update_node(
        &self,
        input: UpdateNodeInput,
    ) -> std::result::Result<UpdateNodeResult, WriteFailure> {
        write::update_node(self, input)
    }

    pub(super) fn open_connection(&self) -> Result<Connection> {
        Ok(Connection::open(&self.db_path)?)
    }
}

#[derive(Debug, Clone)]
pub struct StateFailure {
    pub code: String,
    pub message: String,
    pub details: Value,
}

impl StateFailure {
    pub fn new(code: &str, message: impl Into<String>, details: Value) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            details,
        }
    }
}

impl std::fmt::Display for StateFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for StateFailure {}

pub fn as_state_failure(err: &anyhow::Error) -> Option<&StateFailure> {
    err.chain()
        .find_map(|cause| cause.downcast_ref::<StateFailure>())
}

fn default_db_path(project_root: &Path) -> PathBuf {
    project_root.join(DEFAULT_DB_DIR).join(DEFAULT_DB_NAME)
}

fn stale_index_failure(status: &IndexStatus) -> StateFailure {
    let message = if status.fingerprint_drift_detected {
        "derived index is stale; rebuild_index is required before using project memory"
    } else if !status.failures.is_empty() {
        "derived index is unhealthy; rebuild_index is required before using project memory"
    } else {
        "derived index is not ready; rebuild_index is required before using project memory"
    };
    StateFailure::new(
        "E_STALE_INDEX",
        message,
        json!({
            "kind": "derived_state",
            "safe_recovery_hint": "run rebuild_index to resync the SQLite derived index from Markdown truth",
            "status": status
        }),
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::model::StorageMode;

    use super::{Engine, as_state_failure};

    fn temp_root(prefix: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("obsidian-memory-engine-{prefix}-{suffix}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn write_note(root: &Path, relative: &str, title: &str, node_type: &str, body: &str) {
        let path = root.join("memory").join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        let slug = if relative == "_index.md" {
            "_index".to_string()
        } else {
            Path::new(relative)
                .file_stem()
                .and_then(|value| value.to_str())
                .expect("slug")
                .to_string()
        };
        let content = format!(
            "---\nid: {node_type}-{slug}\ntype: {node_type}\ntitle: {title}\nstatus: active\nproject: workspace\ncreated_at: 1\nupdated_at: 1\n---\n\n# {title}\n\n## Summary\n{body}\n\n## Observations\n\n## Relations\n\n## References\n"
        );
        std::fs::write(path, content).expect("write note");
    }

    #[test]
    fn ensure_index_ready_requires_rebuild_when_db_is_missing() {
        let root = temp_root("missing-db");
        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

        let err = engine
            .ensure_index_ready(false)
            .expect_err("missing db should fail");
        let failure = as_state_failure(&err).expect("state failure");
        assert_eq!(failure.code, "E_REBUILD_REQUIRED");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn ensure_index_ready_reports_stale_index_when_markdown_drift_exists() {
        let root = temp_root("stale");
        write_note(
            &root,
            "_index.md",
            "Workspace",
            "Project",
            "Project summary.",
        );
        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
        engine.rebuild_index().expect("rebuild");

        write_note(
            &root,
            "_index.md",
            "Workspace",
            "Project",
            "Updated summary.",
        );

        let err = engine
            .ensure_index_ready(false)
            .expect_err("stale index should fail");
        let failure = as_state_failure(&err).expect("state failure");
        assert_eq!(failure.code, "E_STALE_INDEX");

        let _ = std::fs::remove_dir_all(root);
    }
}
