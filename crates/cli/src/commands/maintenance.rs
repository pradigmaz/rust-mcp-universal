use std::path::Path;

use anyhow::Result;
use rmu_core::{
    DbMaintenanceOptions, Engine, IgnoreInstallTarget, PrivacyMode, install_ignore_rules,
    sanitize_path_text, sanitize_value_for_privacy,
};

use crate::error::{CODE_CONFIRM_REQUIRED, cli_error};
use crate::output::{print_json, print_line};

pub(crate) struct DbMaintenanceArgs {
    pub(crate) integrity_check: bool,
    pub(crate) checkpoint: bool,
    pub(crate) vacuum: bool,
    pub(crate) analyze: bool,
    pub(crate) stats: bool,
    pub(crate) prune: bool,
}

pub(crate) fn run_install_ignore_rules(
    project_root: &Path,
    json: bool,
    privacy_mode: PrivacyMode,
    target: IgnoreInstallTarget,
) -> Result<()> {
    let report = install_ignore_rules(project_root, target)?;
    if json {
        let mut value = serde_json::to_value(&report)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        print_line(format!(
            "target={}, path={}, created={}, updated={}",
            report.target.as_str(),
            sanitize_path_text(privacy_mode, &report.path),
            report.created,
            report.updated
        ));
    }

    Ok(())
}

pub(crate) fn run_delete_index(
    engine: &Engine,
    json: bool,
    yes: bool,
    privacy_mode: PrivacyMode,
) -> Result<()> {
    if !yes {
        return Err(cli_error(
            CODE_CONFIRM_REQUIRED,
            "delete-index requires --yes",
        ));
    }

    let summary = engine.delete_index_storage()?;
    if json {
        let mut value = serde_json::to_value(&summary)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        print_line(format!(
            "deleted_index: removed_count={}, db={}",
            summary.removed_count,
            sanitize_path_text(privacy_mode, &summary.db_path)
        ));
    }

    Ok(())
}

pub(crate) fn run_db_maintenance(
    engine: &Engine,
    json: bool,
    privacy_mode: PrivacyMode,
    args: DbMaintenanceArgs,
) -> Result<()> {
    let DbMaintenanceArgs {
        integrity_check,
        checkpoint,
        vacuum,
        analyze,
        stats,
        prune,
    } = args;
    let result = engine.db_maintenance(DbMaintenanceOptions {
        integrity_check,
        checkpoint,
        vacuum,
        analyze,
        stats,
        prune,
    })?;
    if json {
        let mut value = serde_json::to_value(&result)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        let checkpoint_info = result
            .checkpoint
            .as_ref()
            .map(|cp| {
                format!(
                    "busy={}, wal_pages={}, checkpointed_pages={}",
                    cp.busy, cp.wal_pages, cp.checkpointed_pages
                )
            })
            .unwrap_or_else(|| "skipped".to_string());
        let integrity_info = result
            .integrity_ok
            .map(|ok| ok.to_string())
            .unwrap_or_else(|| "skipped".to_string());
        let stats_info = result
            .stats
            .as_ref()
            .map(|s| {
                format!(
                    "page_size={}, page_count={}, freelist_count={}, approx_free_bytes={}, db_size_bytes={}, wal_size_bytes={}, shm_size_bytes={}, total_size_bytes={}",
                    s.page_size,
                    s.page_count,
                    s.freelist_count,
                    s.approx_free_bytes,
                    s.db_size_bytes,
                    s.wal_size_bytes,
                    s.shm_size_bytes,
                    s.total_size_bytes
                )
            })
            .unwrap_or_else(|| "skipped".to_string());
        let prune_info = result
            .prune
            .as_ref()
            .map(|summary| {
                format!(
                    "removed_databases={}, removed_sidecars={}, removed_bytes={}",
                    summary.removed_databases, summary.removed_sidecars, summary.removed_bytes
                )
            })
            .unwrap_or_else(|| "skipped".to_string());
        print_line(format!(
            "db={}, integrity_ok={}, checkpoint={}, vacuum_ran={}, analyze_ran={}, prune={}, stats={}",
            sanitize_path_text(privacy_mode, &result.db_path),
            integrity_info,
            checkpoint_info,
            result.vacuum_ran,
            result.analyze_ran,
            prune_info,
            stats_info
        ));
    }
    Ok(())
}

pub(crate) fn run_status(engine: &Engine, json: bool, privacy_mode: PrivacyMode) -> Result<()> {
    let status = engine.index_status()?;
    if json {
        let mut value = serde_json::to_value(&status)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        print_line(format!(
            "files={}, symbols={}, deps={}, refs={}, semantic_vectors={}, file_chunks={}, chunk_embeddings={}, semantic_model={}, lock_wait_ms={}, cache_hits={}, cache_misses={}, db={}",
            status.files,
            status.symbols,
            status.module_deps,
            status.refs,
            status.semantic_vectors,
            status.file_chunks,
            status.chunk_embeddings,
            status.semantic_model,
            status.last_index_lock_wait_ms,
            status.last_embedding_cache_hits,
            status.last_embedding_cache_misses,
            sanitize_path_text(privacy_mode, &status.db_path)
        ));
    }

    Ok(())
}

pub(crate) fn run_preflight(engine: &Engine, json: bool, privacy_mode: PrivacyMode) -> Result<()> {
    let status = engine.preflight_status()?;
    if json {
        let mut value = serde_json::to_value(&status)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        print_line(format!("status={:?}", status.status));
        print_line(format!(
            "running_binary_version={}",
            status.running_binary_version
        ));
        print_line(format!(
            "running_binary_stale={}",
            status.running_binary_stale
        ));
        print_line(format!(
            "supported_schema_version={}",
            status
                .supported_schema_version
                .map_or_else(|| "unknown".to_string(), |value| value.to_string())
        ));
        print_line(format!(
            "db_schema_version={}",
            status
                .db_schema_version
                .map_or_else(|| "unknown".to_string(), |value| value.to_string())
        ));
        print_line(format!(
            "stale_process_suspected={}",
            status.stale_process_suspected
        ));
        print_line(format!(
            "project={}",
            sanitize_path_text(privacy_mode, &status.project_path)
        ));
        print_line(format!(
            "db={}",
            sanitize_path_text(privacy_mode, &engine.db_path.display().to_string())
        ));
        if !status.errors.is_empty() {
            print_line(format!("errors={}", status.errors.join(" | ")));
        }
        print_line(format!("safe_recovery_hint={}", status.safe_recovery_hint));
    }
    Ok(())
}

pub(crate) fn run_brief(engine: &Engine, json: bool, privacy_mode: PrivacyMode) -> Result<()> {
    let brief = engine.workspace_brief()?;
    if json {
        let mut value = serde_json::to_value(&brief)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        print_line(format!(
            "auto_indexed={}, files={}, symbols={}, semantic_vectors={}, file_chunks={}, chunk_embeddings={}, semantic_model={}",
            brief.auto_indexed,
            brief.index_status.files,
            brief.index_status.symbols,
            brief.index_status.semantic_vectors,
            brief.index_status.file_chunks,
            brief.index_status.chunk_embeddings,
            brief.index_status.semantic_model
        ));
    }

    Ok(())
}
