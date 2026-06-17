use anyhow::Result;
use obsidian_memory_core::as_state_failure;
use serde_json::{Value, json};

use crate::ServerState;

use super::errors::{
    invalid_params_error, is_invalid_params_error, is_tool_domain_error, tool_domain_error,
};
use super::handlers::{
    add_observation, context_pack, create_node, decision_log, index_status, link_nodes,
    memory_status, migrate_memory_root, open_nodes, preflight, project_brief, read_graph,
    rebuild_index, recent_changes, risk_hotspots, search_memory, set_project, unlink_nodes,
    update_node,
};
use super::result::{tool_compatibility_error_result, tool_state_error_result};

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

    if !is_known_tool(name) {
        return Err(invalid_params_error(format!("unknown tool: {name}")));
    }

    if tool_requires_bound_project(name) {
        if let Some(binding_failure) = state.binding_failure() {
            return Ok(tool_state_error_result(
                binding_failure.code,
                binding_failure.message,
                binding_failure.details,
            ));
        }
    }

    if !matches!(
        name,
        "preflight" | "memory_status" | "index_status" | "migrate_memory_root"
    ) && tool_requires_bound_project(name)
    {
        if let Some(compatibility_error) = runtime_compatibility_guard(state)? {
            return Ok(compatibility_error);
        }
    }

    match name {
        "set_project" => set_project(&args, state),
        "project_brief" => project_brief(&args, state).map_err(into_tool_error),
        "recent_changes" => recent_changes(&args, state).map_err(into_tool_error),
        "decision_log" => decision_log(&args, state).map_err(into_tool_error),
        "risk_hotspots" => risk_hotspots(&args, state).map_err(into_tool_error),
        "context_pack" => context_pack(&args, state).map_err(into_tool_error),
        "search_memory" => search_memory(&args, state).map_err(into_tool_error),
        "open_nodes" => open_nodes(&args, state).map_err(into_tool_error),
        "read_graph" => read_graph(&args, state).map_err(into_tool_error),
        "memory_status" => memory_status(&args, state).map_err(into_tool_error),
        "preflight" => preflight(&args, state).map_err(into_tool_error),
        "index_status" => index_status(&args, state).map_err(into_tool_error),
        "rebuild_index" => rebuild_index(&args, state).map_err(into_tool_error),
        "migrate_memory_root" => migrate_memory_root(&args, state).map_err(into_tool_error),
        "create_node" => create_node(&args, state).map_err(into_tool_error),
        "add_observation" => add_observation(&args, state).map_err(into_tool_error),
        "link_nodes" => link_nodes(&args, state).map_err(into_tool_error),
        "unlink_nodes" => unlink_nodes(&args, state).map_err(into_tool_error),
        "update_node" => update_node(&args, state).map_err(into_tool_error),
        _ => unreachable!("known tools are handled before dispatch"),
    }
}

fn is_known_tool(name: &str) -> bool {
    matches!(
        name,
        "set_project"
            | "project_brief"
            | "recent_changes"
            | "decision_log"
            | "risk_hotspots"
            | "context_pack"
            | "search_memory"
            | "open_nodes"
            | "read_graph"
            | "memory_status"
            | "preflight"
            | "index_status"
            | "rebuild_index"
            | "migrate_memory_root"
            | "create_node"
            | "add_observation"
            | "link_nodes"
            | "unlink_nodes"
            | "update_node"
    )
}

fn tool_requires_bound_project(name: &str) -> bool {
    !matches!(name, "set_project" | "preflight")
}

fn runtime_compatibility_guard(state: &ServerState) -> Result<Option<Value>> {
    let engine = state.bound_engine()?;
    let status = engine.preflight_status()?;
    if status.running_binary_stale || !status.errors.is_empty() {
        let message = status
            .errors
            .first()
            .cloned()
            .unwrap_or_else(|| "compatibility check failed before tool execution".to_string());
        return Ok(Some(tool_compatibility_error_result(
            message,
            Some(&status),
        )));
    }
    Ok(None)
}

fn into_tool_error(err: anyhow::Error) -> anyhow::Error {
    if is_invalid_params_error(&err)
        || is_tool_domain_error(&err)
        || as_state_failure(&err).is_some()
    {
        err
    } else {
        tool_domain_error(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use crate::ServerState;
    use crate::state::ProjectBindingSource;

    use super::{handle_tool_call, is_known_tool};

    fn temp_root(prefix: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("obsidian-memory-dispatch-{prefix}-{suffix}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    #[test]
    fn bootstrap_tool_names_are_known_and_routable() {
        let root = temp_root("bootstrap");
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
        let mut state = ServerState::new();
        state.bind_project_path(
            root.clone(),
            obsidian_memory_core::StorageMode::Project,
            ProjectBindingSource::SetProject,
        );

        for (name, arguments, expected_key) in [
            ("recent_changes", json!({}), "changes"),
            ("decision_log", json!({"topic": "workspace"}), "decisions"),
            ("risk_hotspots", json!({}), "risks"),
            ("context_pack", json!({"seed": "workspace"}), "budget"),
        ] {
            assert!(is_known_tool(name));
            let response = handle_tool_call(
                Some(json!({
                    "name": name,
                    "arguments": arguments
                })),
                &mut state,
            )
            .expect("dispatch response");
            assert!(response["structuredContent"].get(expected_key).is_some());
        }

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn diagnostics_bypass_compatibility_guard_but_rebuild_does_not() {
        let root = temp_root("compat");
        let index_note = root.join("_index.md");
        std::fs::write(
            &index_note,
            "---\nid: project-compat\ntype: Project\ntitle: Workspace\nstatus: active\nproject: workspace\ncreated_at: 1\nupdated_at: 1\n---\n\n# Workspace\n\n## Summary\nProject summary.\n\n## Observations\n\n## Relations\n\n## References\n",
        )
        .expect("index note");
        let engine = obsidian_memory_core::Engine::new_with_mode(
            &root,
            obsidian_memory_core::StorageMode::Project,
        )
        .expect("engine");
        engine.rebuild_index().expect("rebuild");

        let mut state = ServerState::new();
        state.bind_project_path(
            root.clone(),
            obsidian_memory_core::StorageMode::Project,
            ProjectBindingSource::SetProject,
        );

        let conn =
            rusqlite::Connection::open(root.join("memory").join(".derived").join("index.db"))
                .expect("db");
        conn.execute(
            "UPDATE meta SET value = '999' WHERE key = 'schema_version'",
            [],
        )
        .expect("schema version");
        drop(conn);

        let memory_status = handle_tool_call(
            Some(json!({"name": "memory_status", "arguments": {}})),
            &mut state,
        )
        .expect("memory_status");
        assert_eq!(memory_status["isError"], false);

        let index_status = handle_tool_call(
            Some(json!({"name": "index_status", "arguments": {}})),
            &mut state,
        )
        .expect("index_status");
        assert_eq!(index_status["isError"], false);

        let rebuild = handle_tool_call(
            Some(json!({"name": "rebuild_index", "arguments": {}})),
            &mut state,
        )
        .expect("rebuild_index");
        assert_eq!(rebuild["isError"], true);
        assert_eq!(
            rebuild["structuredContent"]["code"],
            json!("E_SCHEMA_MISMATCH")
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
