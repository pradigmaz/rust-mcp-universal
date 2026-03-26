use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;
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
    Cli,
    InitializeRoots,
    InitializeProjectPath,
    SetProjectPath,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProjectBinding {
    Unbound,
    Bound {
        project_path: PathBuf,
        source: ProjectBindingSource,
    },
    Ambiguous {
        candidates: Vec<PathBuf>,
        source: ProjectBindingSource,
    },
}

#[derive(Parser, Debug)]
#[command(name = "rmu-mcp-server")]
pub(crate) struct App {
    #[arg(long, num_args = 0..=1, default_missing_value = ".")]
    pub(crate) project_path: Option<PathBuf>,
    #[arg(long)]
    pub(crate) db_path: Option<PathBuf>,
    #[arg(long, value_parser = ["stdio"])]
    pub(crate) transport: Option<String>,
}

impl App {
    pub(crate) fn validate_runtime_flags(&self) -> Result<()> {
        if let Some(project_path) = &self.project_path {
            validate_existing_directory(project_path)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ServerState {
    pub(crate) project_path: PathBuf,
    pub(crate) db_path: Option<PathBuf>,
    binding: ProjectBinding,
    db_pinned: bool,
    lifecycle: SessionLifecycle,
    exit_requested: bool,
}

impl ServerState {
    pub(crate) fn new(project_path: Option<PathBuf>, db_path: Option<PathBuf>) -> Self {
        let resolved_cli_project_path = project_path.as_ref().map(|path| {
            normalize_existing_directory(path.as_path()).unwrap_or_else(|| path.clone())
        });
        let normalized_project_path = resolved_cli_project_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));
        let binding = resolved_cli_project_path
            .map(|resolved_path| ProjectBinding::Bound {
                project_path: resolved_path,
                source: ProjectBindingSource::Cli,
            })
            .unwrap_or(ProjectBinding::Unbound);
        let db_pinned = db_path.is_some();
        Self {
            project_path: normalized_project_path,
            db_path,
            binding,
            db_pinned,
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

    pub(crate) fn resolved_project_path(&self) -> Option<&Path> {
        match self.binding() {
            ProjectBinding::Bound { project_path, .. } => Some(project_path.as_path()),
            ProjectBinding::Unbound | ProjectBinding::Ambiguous { .. } => None,
        }
    }

    pub(crate) fn db_pinned(&self) -> bool {
        self.db_pinned
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
        source: ProjectBindingSource,
    ) {
        self.project_path = project_path.clone();
        self.binding = ProjectBinding::Bound {
            project_path,
            source,
        };
    }

    pub(crate) fn matches_bound_project(&self, candidate: &Path) -> bool {
        matches!(
            &self.binding,
            ProjectBinding::Bound { project_path, .. } if project_path == candidate
        )
    }

    pub(crate) fn binding_failure(&self) -> Option<BindingFailure> {
        match &self.binding {
            ProjectBinding::Unbound => Some(BindingFailure {
                code: "E_PROJECT_NOT_BOUND",
                message:
                    "project is not bound; initialize with a workspace root or call set_project_path"
                        .to_string(),
                details: json!({
                    "kind": "project_binding",
                    "binding_status": "unbound",
                    "db_pinned": self.db_pinned,
                    "safe_recovery_hint": "provide initialize roots/projectPath or call set_project_path before using project-scoped tools"
                }),
            }),
            ProjectBinding::Ambiguous { candidates, .. } => Some(BindingFailure {
                code: "E_PROJECT_AMBIGUOUS",
                message:
                    "project binding is ambiguous; provide a single workspace root or call set_project_path"
                        .to_string(),
                details: json!({
                    "kind": "project_binding",
                    "binding_status": "ambiguous",
                    "db_pinned": self.db_pinned,
                    "candidates": candidates,
                    "safe_recovery_hint": "narrow initialize roots to one repository or call set_project_path explicitly"
                }),
            }),
            ProjectBinding::Bound { .. } => None,
        }
    }
}

fn binding_source_name(source: ProjectBindingSource) -> &'static str {
    match source {
        ProjectBindingSource::Cli => "cli",
        ProjectBindingSource::InitializeRoots => "initialize_roots",
        ProjectBindingSource::InitializeProjectPath => "initialize_project_path",
        ProjectBindingSource::SetProjectPath => "set_project_path",
    }
}

#[derive(Debug)]
pub(crate) struct BindingFailure {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) details: Value,
}

fn validate_existing_directory(path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(path)?;
    anyhow::ensure!(
        metadata.is_dir(),
        "project_path must point to an existing directory: {}",
        path.display()
    );
    Ok(())
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
