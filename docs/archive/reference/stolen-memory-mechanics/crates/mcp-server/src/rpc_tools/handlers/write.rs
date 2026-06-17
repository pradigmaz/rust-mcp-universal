use anyhow::Result;
use obsidian_memory_core::{AddObservationInput, CreateNodeInput, LinkNodesInput, UpdateNodeInput};
use serde_json::Value;

use crate::ServerState;
use crate::rpc_tools::parsing::{
    parse_optional_non_empty_string, parse_optional_string, parse_optional_string_list,
    parse_required_non_empty_string, reject_unknown_fields,
};
use crate::rpc_tools::result::{tool_result, tool_state_error_result};

fn write_error_result(error: obsidian_memory_core::WriteFailure) -> Value {
    tool_state_error_result(&error.code, error.message, error.details)
}

pub(crate) fn create_node(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "create_node",
        &[
            "type", "title", "slug", "status", "summary", "tags", "aliases",
        ],
    )?;
    let node_type = parse_required_non_empty_string(args, "create_node", "type")?;
    let title = parse_required_non_empty_string(args, "create_node", "title")?;
    let slug = parse_optional_non_empty_string(args, "create_node", "slug")?;
    let status = parse_optional_non_empty_string(args, "create_node", "status")?;
    let summary = parse_optional_string(args, "create_node", "summary")?;
    let tags = parse_optional_string_list(args, "create_node", "tags")?.unwrap_or_default();
    let aliases = parse_optional_string_list(args, "create_node", "aliases")?.unwrap_or_default();

    let engine = state.bound_engine()?;
    engine.ensure_index_ready(true)?;
    match engine.create_node(CreateNodeInput {
        node_type,
        title,
        slug,
        status,
        summary,
        tags,
        aliases,
    }) {
        Ok(result) => tool_result(serde_json::to_value(result)?),
        Err(error) => Ok(write_error_result(error)),
    }
}

pub(crate) fn add_observation(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "add_observation", &["node", "content"])?;
    let node = parse_required_non_empty_string(args, "add_observation", "node")?;
    let content = parse_required_non_empty_string(args, "add_observation", "content")?;

    let engine = state.bound_engine()?;
    engine.ensure_index_ready(true)?;
    match engine.add_observation(AddObservationInput { node, content }) {
        Ok(result) => tool_result(serde_json::to_value(result)?),
        Err(error) => Ok(write_error_result(error)),
    }
}

pub(crate) fn link_nodes(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "link_nodes", &["source", "target", "relation_kind"])?;
    let source = parse_required_non_empty_string(args, "link_nodes", "source")?;
    let target = parse_required_non_empty_string(args, "link_nodes", "target")?;
    let relation_kind = parse_required_non_empty_string(args, "link_nodes", "relation_kind")?;

    let engine = state.bound_engine()?;
    engine.ensure_index_ready(true)?;
    match engine.link_nodes(LinkNodesInput {
        source,
        target,
        relation_kind,
    }) {
        Ok(result) => tool_result(serde_json::to_value(result)?),
        Err(error) => Ok(write_error_result(error)),
    }
}

pub(crate) fn unlink_nodes(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "unlink_nodes", &["source", "target", "relation_kind"])?;
    let source = parse_required_non_empty_string(args, "unlink_nodes", "source")?;
    let target = parse_required_non_empty_string(args, "unlink_nodes", "target")?;
    let relation_kind = parse_required_non_empty_string(args, "unlink_nodes", "relation_kind")?;

    let engine = state.bound_engine()?;
    engine.ensure_index_ready(true)?;
    match engine.unlink_nodes(LinkNodesInput {
        source,
        target,
        relation_kind,
    }) {
        Ok(result) => tool_result(serde_json::to_value(result)?),
        Err(error) => Ok(write_error_result(error)),
    }
}

pub(crate) fn update_node(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "update_node",
        &["node", "title", "status", "summary", "tags", "aliases"],
    )?;
    let node = parse_required_non_empty_string(args, "update_node", "node")?;
    let title = parse_optional_non_empty_string(args, "update_node", "title")?;
    let status = parse_optional_non_empty_string(args, "update_node", "status")?;
    let summary = parse_optional_string(args, "update_node", "summary")?;
    let tags = parse_optional_string_list(args, "update_node", "tags")?;
    let aliases = parse_optional_string_list(args, "update_node", "aliases")?;

    let engine = state.bound_engine()?;
    engine.ensure_index_ready(true)?;
    match engine.update_node(UpdateNodeInput {
        node,
        title,
        status,
        summary,
        tags,
        aliases,
    }) {
        Ok(result) => tool_result(serde_json::to_value(result)?),
        Err(error) => Ok(write_error_result(error)),
    }
}
