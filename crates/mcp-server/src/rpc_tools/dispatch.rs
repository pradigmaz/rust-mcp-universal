use std::path::PathBuf;

use anyhow::Result;
use rmu_core::{
    Engine, IndexProfile, IndexingOptions, MigrationMode, PrivacyMode, sanitize_value_for_privacy,
};
use serde_json::{Value, json};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use crate::ServerState;

use super::errors::{invalid_params_error, is_invalid_params_error, tool_domain_error};
use super::handlers::{
    agent_bootstrap, build_context_under_budget, call_path, context_pack, db_maintenance,
    query_benchmark, query_report, related_files, related_files_v2, search_candidates,
    semantic_search, symbol_lookup, symbol_lookup_v2, symbol_references, symbol_references_v2,
};
use super::parsing::{
    parse_optional_bool, parse_optional_string_list, parse_required_non_empty_string,
    reject_unknown_fields,
};
use super::result::tool_result;

pub(super) fn handle_tool_call(params: Option<Value>, state: &mut ServerState) -> Result<Value> {
    let params = params.ok_or_else(|| invalid_params_error("tools/call params are required"))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params_error("tools/call requires string field `name`"))?;
    let args = match params.get("arguments") {
        Some(value) if value.is_object() => value.clone(),
        Some(value) => {
            return Err(invalid_params_error(format!(
                "tools/call `arguments` must be object, got {}",
                value
            )));
        }
        None => json!({}),
    };

    match name {
        "set_project_path" => {
            reject_unknown_fields(&args, "set_project_path", &["project_path"])?;
            let project_path =
                parse_required_non_empty_string(&args, "set_project_path", "project_path")?;
            let path = PathBuf::from(&project_path);
            let metadata = std::fs::metadata(&path).map_err(|_| {
                invalid_params_error(format!(
                    "set_project_path `project_path` does not exist: {project_path}"
                ))
            })?;
            if !metadata.is_dir() {
                return Err(invalid_params_error(format!(
                    "set_project_path `project_path` must be an existing directory: {project_path}"
                )));
            }
            state.project_path = path;
            tool_result(json!({"ok": true, "project_path": project_path}))
        }
        "index_status" => {
            reject_unknown_fields(&args, "index_status", &["migration_mode"])?;
            let migration_mode = parse_optional_migration_mode(&args, "index_status")?
                .unwrap_or(MigrationMode::Auto);
            let engine = Engine::new_read_only_with_migration_mode(
                state.project_path.clone(),
                state.db_path.clone(),
                migration_mode,
            )
            .map_err(|err| tool_domain_error(err.to_string()))?;
            let status = engine
                .index_status()
                .map_err(|err| tool_domain_error(err.to_string()))?;
            tool_result(serde_json::to_value(status)?)
        }
        "workspace_brief" => {
            reject_unknown_fields(&args, "workspace_brief", &["migration_mode"])?;
            let migration_mode = parse_optional_migration_mode(&args, "workspace_brief")?
                .unwrap_or(MigrationMode::Auto);
            let engine = Engine::new_read_only_with_migration_mode(
                state.project_path.clone(),
                state.db_path.clone(),
                migration_mode,
            )
            .map_err(|err| tool_domain_error(err.to_string()))?;
            let brief = engine
                .workspace_brief_with_policy(false)
                .map_err(|err| tool_domain_error(err.to_string()))?;
            tool_result(serde_json::to_value(brief)?)
        }
        "agent_bootstrap" => agent_bootstrap(&args, state).map_err(into_tool_error),
        "index" | "semantic_index" => {
            reject_unknown_fields(
                &args,
                name,
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
                parse_optional_string_list(&args, name, "include_paths")?.unwrap_or_default();
            let exclude_paths =
                parse_optional_string_list(&args, name, "exclude_paths")?.unwrap_or_default();
            let profile = parse_optional_index_profile(&args, name)?.or(Some(IndexProfile::Mixed));
            let changed_since = parse_optional_changed_since(&args, name)?;
            let changed_since_commit = parse_optional_changed_since_commit(&args, name)?;
            if changed_since.is_some() && changed_since_commit.is_some() {
                return Err(invalid_params_error(format!(
                    "{name} `changed_since` and `changed_since_commit` are mutually exclusive"
                )));
            }
            let reindex = parse_optional_bool(&args, name, "reindex")?.unwrap_or(false);
            let migration_mode =
                parse_optional_migration_mode(&args, name)?.unwrap_or(MigrationMode::Auto);
            let engine = Engine::new_with_migration_mode(
                state.project_path.clone(),
                state.db_path.clone(),
                migration_mode,
            )
            .map_err(|err| tool_domain_error(err.to_string()))?;
            let summary = engine
                .index_path_with_options(&IndexingOptions {
                    profile,
                    changed_since,
                    changed_since_commit,
                    include_paths: include_paths.clone(),
                    exclude_paths: exclude_paths.clone(),
                    reindex,
                })
                .map_err(|err| tool_domain_error(err.to_string()))?;
            let semantic_vectors_rebuilt = summary.changed > 0 || summary.added > 0;
            tool_result(json!({
                "summary": {
                    "profile": profile.map(IndexProfile::as_str),
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
        "scope_preview" => {
            reject_unknown_fields(
                &args,
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
                parse_optional_string_list(&args, "scope_preview", "include_paths")?
                    .unwrap_or_default();
            let exclude_paths =
                parse_optional_string_list(&args, "scope_preview", "exclude_paths")?
                    .unwrap_or_default();
            let profile =
                parse_optional_index_profile(&args, "scope_preview")?.or(Some(IndexProfile::Mixed));
            let changed_since = parse_optional_changed_since(&args, "scope_preview")?;
            let changed_since_commit = parse_optional_changed_since_commit(&args, "scope_preview")?;
            if changed_since.is_some() && changed_since_commit.is_some() {
                return Err(invalid_params_error(
                    "scope_preview `changed_since` and `changed_since_commit` are mutually exclusive",
                ));
            }
            let reindex = parse_optional_bool(&args, "scope_preview", "reindex")?.unwrap_or(false);
            let privacy_mode =
                parse_optional_privacy_mode(&args, "scope_preview")?.unwrap_or(PrivacyMode::Off);
            let migration_mode = parse_optional_migration_mode(&args, "scope_preview")?
                .unwrap_or(MigrationMode::Auto);
            let engine = Engine::new_read_only_with_migration_mode(
                state.project_path.clone(),
                state.db_path.clone(),
                migration_mode,
            )
            .map_err(|err| tool_domain_error(err.to_string()))?;
            let preview = engine
                .scope_preview_with_options(&IndexingOptions {
                    profile,
                    changed_since,
                    changed_since_commit,
                    include_paths,
                    exclude_paths,
                    reindex,
                })
                .map_err(|err| tool_domain_error(err.to_string()))?;
            let mut payload = serde_json::to_value(preview)?;
            sanitize_value_for_privacy(privacy_mode, &mut payload);
            tool_result(payload)
        }
        "delete_index" => {
            reject_unknown_fields(&args, "delete_index", &["confirm", "migration_mode"])?;
            let confirm = parse_optional_bool(&args, "delete_index", "confirm")?.unwrap_or(false);
            if !confirm {
                return Err(invalid_params_error("delete_index requires `confirm=true`"));
            }
            let migration_mode = parse_optional_migration_mode(&args, "delete_index")?
                .unwrap_or(MigrationMode::Auto);
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
        "symbol_lookup" => symbol_lookup(&args, state).map_err(into_tool_error),
        "symbol_lookup_v2" => symbol_lookup_v2(&args, state).map_err(into_tool_error),
        "symbol_references" => symbol_references(&args, state).map_err(into_tool_error),
        "symbol_references_v2" => symbol_references_v2(&args, state).map_err(into_tool_error),
        "related_files" => related_files(&args, state).map_err(into_tool_error),
        "related_files_v2" => related_files_v2(&args, state).map_err(into_tool_error),
        "call_path" => call_path(&args, state).map_err(into_tool_error),
        "search_candidates" => search_candidates(&args, state).map_err(into_tool_error),
        "semantic_search" => semantic_search(&args, state).map_err(into_tool_error),
        "build_context_under_budget" => {
            build_context_under_budget(&args, state).map_err(into_tool_error)
        }
        "context_pack" => context_pack(&args, state).map_err(into_tool_error),
        "query_report" => query_report(&args, state).map_err(into_tool_error),
        "query_benchmark" => query_benchmark(&args, state).map_err(into_tool_error),
        "db_maintenance" => db_maintenance(&args, state).map_err(into_tool_error),
        _ => Err(invalid_params_error(format!("unknown tool: {name}"))),
    }
}

fn into_tool_error(err: anyhow::Error) -> anyhow::Error {
    if is_invalid_params_error(&err) {
        err
    } else {
        tool_domain_error(err.to_string())
    }
}

fn parse_optional_migration_mode(args: &Value, tool_name: &str) -> Result<Option<MigrationMode>> {
    let Some(raw) = args.get("migration_mode") else {
        return Ok(None);
    };
    let Some(raw_string) = raw.as_str() else {
        return Err(invalid_params_error(format!(
            "{tool_name} `migration_mode` must be string"
        )));
    };
    let parsed = MigrationMode::parse(raw_string).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `migration_mode` must be one of: auto, off"
        ))
    })?;
    Ok(Some(parsed))
}

fn parse_optional_privacy_mode(args: &Value, tool_name: &str) -> Result<Option<PrivacyMode>> {
    let Some(raw) = args.get("privacy_mode") else {
        return Ok(None);
    };
    let Some(raw_string) = raw.as_str() else {
        return Err(invalid_params_error(format!(
            "{tool_name} `privacy_mode` must be string"
        )));
    };
    let parsed = PrivacyMode::parse(raw_string).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `privacy_mode` must be one of: off, mask, hash"
        ))
    })?;
    Ok(Some(parsed))
}

fn parse_optional_index_profile(args: &Value, tool_name: &str) -> Result<Option<IndexProfile>> {
    let Some(raw) = args.get("profile") else {
        return Ok(None);
    };
    let Some(raw_string) = raw.as_str() else {
        return Err(invalid_params_error(format!(
            "{tool_name} `profile` must be string"
        )));
    };
    let parsed = IndexProfile::parse(raw_string).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `profile` must be one of: rust-monorepo, mixed, docs-heavy"
        ))
    })?;
    Ok(Some(parsed))
}

fn parse_optional_changed_since(args: &Value, tool_name: &str) -> Result<Option<OffsetDateTime>> {
    let Some(raw) = args.get("changed_since") else {
        return Ok(None);
    };
    let Some(raw_string) = raw.as_str() else {
        return Err(invalid_params_error(format!(
            "{tool_name} `changed_since` must be string"
        )));
    };
    let parsed = OffsetDateTime::parse(raw_string.trim(), &Rfc3339)
        .map(|value| value.to_offset(UtcOffset::UTC))
        .map_err(|_| {
            invalid_params_error(format!(
                "{tool_name} `changed_since` must be RFC3339 timestamp with timezone"
            ))
        })?;
    Ok(Some(parsed))
}

fn parse_optional_changed_since_commit(args: &Value, tool_name: &str) -> Result<Option<String>> {
    let Some(raw) = args.get("changed_since_commit") else {
        return Ok(None);
    };
    let Some(raw_string) = raw.as_str() else {
        return Err(invalid_params_error(format!(
            "{tool_name} `changed_since_commit` must be string"
        )));
    };
    let trimmed = raw_string.trim();
    if trimmed.is_empty() {
        return Err(invalid_params_error(format!(
            "{tool_name} `changed_since_commit` must be non-empty"
        )));
    }
    Ok(Some(trimmed.to_string()))
}

fn format_changed_since(value: OffsetDateTime) -> Result<String> {
    value
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(Into::into)
}
