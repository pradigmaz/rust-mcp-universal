use anyhow::Result;
use rmu_core::{Engine, MigrationMode, PrivacyMode, RolloutPhase};

use crate::args::{App, Command};

use super::modes::{parse_migration_mode, parse_privacy_mode, parse_rollout_phase};
use super::preflight::preflight_validate;

pub(super) struct PreparedRun {
    pub(super) engine: Engine,
    pub(super) json: bool,
    pub(super) privacy_mode: PrivacyMode,
    pub(super) vector_layer_enabled: bool,
    pub(super) rollout_phase: RolloutPhase,
    pub(super) migration_mode: MigrationMode,
    pub(super) command: Command,
}

pub(super) fn prepare(app: App) -> Result<PreparedRun> {
    let App {
        project_path,
        db_path,
        json,
        privacy_mode,
        vector_layer_enabled,
        rollout_phase,
        migration_mode,
        command,
    } = app;

    let privacy_mode = parse_privacy_mode(&privacy_mode)?;
    let rollout_phase = parse_rollout_phase(&rollout_phase)?;
    let migration_mode = parse_migration_mode(&migration_mode)?;
    preflight_validate(&command)?;
    let engine = match &command {
        Command::Status | Command::ScopePreview(_) => {
            Engine::new_read_only_with_migration_mode(project_path, db_path, migration_mode)?
        }
        _ => Engine::new_with_migration_mode(project_path, db_path, migration_mode)?,
    };

    Ok(PreparedRun {
        engine,
        json,
        privacy_mode,
        vector_layer_enabled,
        rollout_phase,
        migration_mode,
        command,
    })
}
