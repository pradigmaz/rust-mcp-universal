use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SessionLifecycle {
    Uninitialized,
    AwaitingInitialized,
    Running,
    ShutdownRequested,
}

#[derive(Parser, Debug)]
#[command(name = "rmu-mcp-server")]
pub(crate) struct App {
    #[arg(long, default_value = ".", num_args = 0..=1, default_missing_value = ".")]
    pub(crate) project_path: PathBuf,
    #[arg(long)]
    pub(crate) db_path: Option<PathBuf>,
    #[arg(long, value_parser = ["stdio"])]
    pub(crate) transport: Option<String>,
}

impl App {
    pub(crate) fn validate_runtime_flags(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ServerState {
    pub(crate) project_path: PathBuf,
    pub(crate) db_path: Option<PathBuf>,
    lifecycle: SessionLifecycle,
    exit_requested: bool,
}

impl ServerState {
    pub(crate) fn new(project_path: PathBuf, db_path: Option<PathBuf>) -> Self {
        Self {
            project_path,
            db_path,
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
}
