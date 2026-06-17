use anyhow::Result;
use obsidian_memory_core::sanitize_value_for_privacy;
use serde_json::{Value, json};

use crate::ServerState;
use crate::rpc_tools::errors::invalid_params_error;
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_non_empty_string, parse_optional_usize_with_min,
    parse_required_non_empty_string, reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::modes::parse_optional_privacy_mode;

pub(crate) fn recent_changes(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "recent_changes",
        &["limit", "auto_index", "privacy_mode"],
    )?;
    let limit = parse_bounded_usize(args, "recent_changes", "limit", 1, 10, 50)?;
    let auto_index = parse_optional_bool(args, "recent_changes", "auto_index")?.unwrap_or(false);
    let privacy_mode =
        parse_optional_privacy_mode(args, "recent_changes", "privacy_mode")?.unwrap_or_default();
    let engine = state.bound_engine()?;
    engine.ensure_index_ready(auto_index)?;
    let mut payload = serde_json::to_value(json!({
        "changes": engine.recent_changes(limit)?
    }))?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

pub(crate) fn decision_log(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "decision_log",
        &["topic", "limit", "auto_index", "privacy_mode"],
    )?;
    let topic = parse_optional_non_empty_string(args, "decision_log", "topic")?;
    let limit = parse_bounded_usize(args, "decision_log", "limit", 1, 10, 50)?;
    let auto_index = parse_optional_bool(args, "decision_log", "auto_index")?.unwrap_or(false);
    let privacy_mode =
        parse_optional_privacy_mode(args, "decision_log", "privacy_mode")?.unwrap_or_default();
    let engine = state.bound_engine()?;
    engine.ensure_index_ready(auto_index)?;
    let mut payload = serde_json::to_value(json!({
        "decisions": engine.decision_log(topic.as_deref(), limit)?
    }))?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

pub(crate) fn risk_hotspots(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "risk_hotspots",
        &["limit", "auto_index", "privacy_mode"],
    )?;
    let limit = parse_bounded_usize(args, "risk_hotspots", "limit", 1, 10, 50)?;
    let auto_index = parse_optional_bool(args, "risk_hotspots", "auto_index")?.unwrap_or(false);
    let privacy_mode =
        parse_optional_privacy_mode(args, "risk_hotspots", "privacy_mode")?.unwrap_or_default();
    let engine = state.bound_engine()?;
    engine.ensure_index_ready(auto_index)?;
    let mut payload = serde_json::to_value(engine.risk_hotspots(limit)?)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

pub(crate) fn context_pack(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "context_pack",
        &[
            "seed",
            "limit",
            "max_chars",
            "max_tokens",
            "auto_index",
            "privacy_mode",
        ],
    )?;
    let seed = parse_required_non_empty_string(args, "context_pack", "seed")?;
    let limit = parse_bounded_usize(args, "context_pack", "limit", 1, 8, 12)?;
    let max_chars = parse_bounded_usize(args, "context_pack", "max_chars", 1, 4000, 12000)?;
    let max_tokens = parse_bounded_usize(args, "context_pack", "max_tokens", 1, 1200, 3000)?;
    let auto_index = parse_optional_bool(args, "context_pack", "auto_index")?.unwrap_or(false);
    let privacy_mode =
        parse_optional_privacy_mode(args, "context_pack", "privacy_mode")?.unwrap_or_default();
    let engine = state.bound_engine()?;
    engine.ensure_index_ready(auto_index)?;
    let mut payload =
        serde_json::to_value(engine.context_pack(&seed, limit, max_chars, max_tokens)?)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

fn parse_bounded_usize(
    args: &Value,
    tool: &str,
    field: &str,
    minimum: usize,
    default: usize,
    maximum: usize,
) -> Result<usize> {
    let parsed = parse_optional_usize_with_min(args, tool, field, minimum, default)?;
    if parsed > maximum {
        return Err(invalid_params_error(format!(
            "{tool} requires `{field}` <= {maximum}, got {parsed}"
        )));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use crate::ServerState;
    use crate::state::ProjectBindingSource;

    use super::{context_pack, decision_log, recent_changes, risk_hotspots};

    fn temp_root(prefix: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("obsidian-memory-mcp-{prefix}-{suffix}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn bound_state(root: &Path) -> ServerState {
        let mut state = ServerState::new();
        state.bind_project_path(
            root.to_path_buf(),
            obsidian_memory_core::StorageMode::Project,
            ProjectBindingSource::SetProject,
        );
        state
    }

    #[test]
    fn bootstrap_handlers_publish_expected_top_level_keys() {
        let root = temp_root("handlers");
        let mut state = bound_state(&root);
        let engine = obsidian_memory_core::Engine::new_with_mode(
            &root,
            obsidian_memory_core::StorageMode::Project,
        )
        .expect("engine");
        engine
            .create_node(obsidian_memory_core::CreateNodeInput {
                node_type: "Project".to_string(),
                title: "Workspace".to_string(),
                slug: Some("_index".to_string()),
                status: Some("active".to_string()),
                summary: Some("Project summary".to_string()),
                tags: Vec::new(),
                aliases: Vec::new(),
            })
            .expect("project");
        engine
            .create_node(obsidian_memory_core::CreateNodeInput {
                node_type: "Decision".to_string(),
                title: "Auth Decision".to_string(),
                slug: Some("auth-decision".to_string()),
                status: Some("active".to_string()),
                summary: Some("Auth summary".to_string()),
                tags: Vec::new(),
                aliases: Vec::new(),
            })
            .expect("decision");

        let changes = recent_changes(&json!({}), &mut state).expect("recent_changes");
        let decisions = decision_log(&json!({"topic": "auth"}), &mut state).expect("decision_log");
        let hotspots = risk_hotspots(&json!({}), &mut state).expect("risk_hotspots");
        let pack = context_pack(&json!({"seed": "auth"}), &mut state).expect("context_pack");

        assert!(changes["structuredContent"].get("changes").is_some());
        assert!(decisions["structuredContent"].get("decisions").is_some());
        assert!(hotspots["structuredContent"].get("risks").is_some());
        assert!(hotspots["structuredContent"].get("constraints").is_some());
        for key in [
            "seed",
            "brief",
            "included_nodes",
            "recent_changes",
            "risks",
            "budget",
        ] {
            assert!(
                pack["structuredContent"].get(key).is_some(),
                "missing key {key}"
            );
        }

        let _ = std::fs::remove_dir_all(root);
    }
}
