use std::path::{Path, PathBuf};

use anyhow::Result;
use obsidian_memory_core::{Engine, ProjectContext, StorageMode};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SessionLifecycle {
    Uninitialized,
    AwaitingInitialized,
    Running,
    ShutdownRequested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProjectBindingSource {
    InitializeRoots,
    InitializeProjectPath,
    SetProject,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProjectBinding {
    Unbound,
    Bound {
        project_path: PathBuf,
        storage_mode: StorageMode,
        source: ProjectBindingSource,
    },
    Ambiguous {
        candidates: Vec<PathBuf>,
        source: ProjectBindingSource,
    },
}

#[derive(Debug)]
pub(crate) struct ServerState {
    pub(crate) project_path: PathBuf,
    default_storage_mode: StorageMode,
    binding: ProjectBinding,
    lifecycle: SessionLifecycle,
    exit_requested: bool,
}

impl ServerState {
    pub(crate) fn new() -> Self {
        Self {
            project_path: PathBuf::from("."),
            default_storage_mode: StorageMode::Codex,
            binding: ProjectBinding::Unbound,
            lifecycle: SessionLifecycle::Uninitialized,
            exit_requested: false,
        }
    }

    pub(crate) fn lifecycle(&self) -> SessionLifecycle {
        self.lifecycle
    }

    pub(crate) fn set_lifecycle(&mut self, lifecycle: SessionLifecycle) {
        self.lifecycle = lifecycle;
    }

    pub(crate) fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    pub(crate) fn should_exit(&self) -> bool {
        self.exit_requested
    }

    pub(crate) fn binding(&self) -> &ProjectBinding {
        &self.binding
    }

    pub(crate) fn binding_status(&self) -> &'static str {
        match self.binding() {
            ProjectBinding::Unbound => "unbound",
            ProjectBinding::Bound { .. } => "bound",
            ProjectBinding::Ambiguous { .. } => "ambiguous",
        }
    }

    pub(crate) fn binding_source(&self) -> Option<&'static str> {
        match self.binding() {
            ProjectBinding::Unbound => None,
            ProjectBinding::Bound { source, .. } | ProjectBinding::Ambiguous { source, .. } => {
                Some(binding_source_name(*source))
            }
        }
    }

    pub(crate) fn default_storage_mode(&self) -> StorageMode {
        self.default_storage_mode
    }

    pub(crate) fn set_default_storage_mode(&mut self, storage_mode: StorageMode) {
        self.default_storage_mode = storage_mode;
    }

    pub(crate) fn effective_storage_mode(&self) -> StorageMode {
        match self.binding() {
            ProjectBinding::Bound { storage_mode, .. } => *storage_mode,
            _ => self.default_storage_mode,
        }
    }

    pub(crate) fn apply_binding(&mut self, binding: ProjectBinding) {
        if let ProjectBinding::Bound { project_path, .. } = &binding {
            self.project_path = project_path.clone();
        }
        self.binding = binding;
    }

    pub(crate) fn bind_project_path(
        &mut self,
        project_path: PathBuf,
        storage_mode: StorageMode,
        source: ProjectBindingSource,
    ) {
        self.project_path = project_path.clone();
        self.binding = ProjectBinding::Bound {
            project_path,
            storage_mode,
            source,
        };
    }

    pub(crate) fn bound_context(&self) -> Result<ProjectContext> {
        let context = ProjectContext::resolve(&self.project_path, self.effective_storage_mode())?;
        context.ensure_project_binding_marker()?;
        Ok(context)
    }

    pub(crate) fn bound_engine(&self) -> Result<Engine> {
        Engine::from_context(self.bound_context()?)
    }

    pub(crate) fn binding_failure(&self) -> Option<BindingFailure> {
        match &self.binding {
            ProjectBinding::Unbound => Some(BindingFailure {
                code: "E_PROJECT_NOT_BOUND",
                message:
                    "project is not bound; initialize with a workspace root or call set_project"
                        .to_string(),
                details: json!({
                    "kind": "project_binding",
                    "binding_status": "unbound",
                    "safe_recovery_hint": "provide initialize roots/projectPath or call set_project before using project-scoped tools"
                }),
            }),
            ProjectBinding::Ambiguous { candidates, .. } => Some(BindingFailure {
                code: "E_PROJECT_AMBIGUOUS",
                message:
                    "project binding is ambiguous; provide a single workspace root or call set_project"
                        .to_string(),
                details: json!({
                    "kind": "project_binding",
                    "binding_status": "ambiguous",
                    "candidates": candidates,
                    "safe_recovery_hint": "narrow initialize roots to one vault or call set_project explicitly"
                }),
            }),
            ProjectBinding::Bound { .. } => None,
        }
    }
}

fn binding_source_name(source: ProjectBindingSource) -> &'static str {
    match source {
        ProjectBindingSource::InitializeRoots => "initialize_roots",
        ProjectBindingSource::InitializeProjectPath => "initialize_project_path",
        ProjectBindingSource::SetProject => "set_project",
    }
}

#[derive(Debug)]
pub(crate) struct BindingFailure {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) details: Value,
}

pub(crate) fn normalize_existing_directory(path: &Path) -> Option<PathBuf> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_dir() {
        return None;
    }
    std::fs::canonicalize(path).ok().map(strip_verbatim_prefix)
}

fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(raw) = path.to_str().and_then(|value| value.strip_prefix(r"\\?\")) {
            return PathBuf::from(raw);
        }
    }
    path
}
