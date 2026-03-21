use anyhow::Result;
use rmu_core::{
    Engine, IndexProfile, IndexingOptions, MigrationMode, PrivacyMode, sanitize_value_for_privacy,
};
use serde_json::{Value, json};

use crate::ServerState;
use crate::rpc_tools::errors::{invalid_params_error, tool_domain_error};
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_string_list, reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::parsing::{
    format_changed_since, parse_optional_changed_since, parse_optional_changed_since_commit,
    parse_optional_index_profile, parse_optional_migration_mode, parse_optional_privacy_mode,
};

pub(super) fn index(args: &Value, tool_name: &str, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        tool_name,
        &[
            "profile",
            "changed_since",
            "changed_since_commit",
            "include_paths",
            "exclude_paths",
            "reindex",
            "migration_mode",
        ],
    )?;
    let include_paths =
        parse_optional_string_list(args, tool_name, "include_paths")?.unwrap_or_default();
    let exclude_paths =
        parse_optional_string_list(args, tool_name, "exclude_paths")?.unwrap_or_default();
    let changed_since = parse_optional_changed_since(args, tool_name)?;
    let changed_since_commit = parse_optional_changed_since_commit(args, tool_name)?;
    if changed_since.is_some() && changed_since_commit.is_some() {
        return Err(invalid_params_error(format!(
            "{tool_name} `changed_since` and `changed_since_commit` are mutually exclusive"
        )));
    }
    let reindex = parse_optional_bool(args, tool_name, "reindex")?.unwrap_or(false);
    let migration_mode =
        parse_optional_migration_mode(args, tool_name)?.unwrap_or(MigrationMode::Auto);
    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )
    .map_err(|err| tool_domain_error(err.to_string()))?;
    let options = effective_indexing_options(
        &engine,
        parse_optional_index_profile(args, tool_name)?,
        changed_since,
        changed_since_commit,
        include_paths.clone(),
        exclude_paths.clone(),
        reindex,
    );
    let summary = engine
        .index_path_with_options(&options)
        .map_err(|err| tool_domain_error(err.to_string()))?;
    let semantic_vectors_rebuilt = summary.changed > 0 || summary.added > 0;
    tool_result(json!({
        "summary": {
            "profile": options.profile.map(IndexProfile::as_str),
            "changed_since": summary.changed_since.map(format_changed_since).transpose()?,
            "changed_since_commit": summary.changed_since_commit,
            "resolved_merge_base_commit": summary.resolved_merge_base_commit,
            "reindex": reindex,
            "include_paths": include_paths,
            "exclude_paths": exclude_paths,
            "scanned": summary.scanned,
            "indexed": summary.indexed,
            "skipped_binary_or_large": summary.skipped_binary_or_large,
            "skipped_before_changed_since": summary.skipped_before_changed_since,
            "semantic_vectors_rebuilt": semantic_vectors_rebuilt,
            "added": summary.added,
            "changed": summary.changed,
            "unchanged": summary.unchanged,
            "deleted": summary.deleted,
            "lock_wait_ms": summary.lock_wait_ms,
            "embedding_cache_hits": summary.embedding_cache_hits,
            "embedding_cache_misses": summary.embedding_cache_misses
        }
    }))
}

pub(super) fn scope_preview(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "scope_preview",
        &[
            "profile",
            "changed_since",
            "changed_since_commit",
            "include_paths",
            "exclude_paths",
            "reindex",
            "privacy_mode",
            "migration_mode",
        ],
    )?;
    let include_paths =
        parse_optional_string_list(args, "scope_preview", "include_paths")?.unwrap_or_default();
    let exclude_paths =
        parse_optional_string_list(args, "scope_preview", "exclude_paths")?.unwrap_or_default();
    let changed_since = parse_optional_changed_since(args, "scope_preview")?;
    let changed_since_commit = parse_optional_changed_since_commit(args, "scope_preview")?;
    if changed_since.is_some() && changed_since_commit.is_some() {
        return Err(invalid_params_error(
            "scope_preview `changed_since` and `changed_since_commit` are mutually exclusive",
        ));
    }
    let reindex = parse_optional_bool(args, "scope_preview", "reindex")?.unwrap_or(false);
    let privacy_mode =
        parse_optional_privacy_mode(args, "scope_preview")?.unwrap_or(PrivacyMode::Off);
    let migration_mode =
        parse_optional_migration_mode(args, "scope_preview")?.unwrap_or(MigrationMode::Auto);
    let engine = Engine::new_read_only_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )
    .map_err(|err| tool_domain_error(err.to_string()))?;
    let options = effective_indexing_options(
        &engine,
        parse_optional_index_profile(args, "scope_preview")?,
        changed_since,
        changed_since_commit,
        include_paths,
        exclude_paths,
        reindex,
    );
    let preview = engine
        .scope_preview_with_options(&options)
        .map_err(|err| tool_domain_error(err.to_string()))?;
    let mut payload = serde_json::to_value(preview)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

pub(super) fn delete_index(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "delete_index", &["confirm", "migration_mode"])?;
    let confirm = parse_optional_bool(args, "delete_index", "confirm")?.unwrap_or(false);
    if !confirm {
        return Err(invalid_params_error("delete_index requires `confirm=true`"));
    }
    let migration_mode =
        parse_optional_migration_mode(args, "delete_index")?.unwrap_or(MigrationMode::Auto);
    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )
    .map_err(|err| tool_domain_error(err.to_string()))?;
    let summary = engine
        .delete_index_storage()
        .map_err(|err| tool_domain_error(err.to_string()))?;
    tool_result(serde_json::to_value(summary)?)
}

fn effective_indexing_options(
    engine: &Engine,
    requested_profile: Option<IndexProfile>,
    changed_since: Option<time::OffsetDateTime>,
    changed_since_commit: Option<String>,
    include_paths: Vec<String>,
    exclude_paths: Vec<String>,
    reindex: bool,
) -> IndexingOptions {
    engine.resolve_indexing_options(&IndexingOptions {
        profile: requested_profile
            .or_else(|| engine.resolve_default_index_profile(None))
            .or(Some(IndexProfile::Mixed)),
        changed_since,
        changed_since_commit,
        include_paths,
        exclude_paths,
        reindex,
    })
}
