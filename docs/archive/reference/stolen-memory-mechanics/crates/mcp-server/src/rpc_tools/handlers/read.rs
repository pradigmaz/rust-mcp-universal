use anyhow::Result;
use obsidian_memory_core::sanitize_value_for_privacy;
use serde_json::{Value, json};

use crate::ServerState;
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_usize_with_min, parse_required_non_empty_string,
    parse_required_string_list, reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::modes::parse_optional_privacy_mode;

pub(crate) fn search_memory(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "search_memory",
        &["query", "limit", "auto_index", "privacy_mode"],
    )?;
    let query = parse_required_non_empty_string(args, "search_memory", "query")?;
    let limit = parse_optional_usize_with_min(args, "search_memory", "limit", 1, 10)?;
    let auto_index = parse_optional_bool(args, "search_memory", "auto_index")?.unwrap_or(false);
    let privacy_mode =
        parse_optional_privacy_mode(args, "search_memory", "privacy_mode")?.unwrap_or_default();
    let engine = state.bound_engine()?;
    engine.ensure_index_ready(auto_index)?;
    let mut payload = serde_json::to_value(json!({
        "query": query,
        "hits": engine.search_memory(&query, limit)?
    }))?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

pub(crate) fn open_nodes(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "open_nodes", &["slugs", "auto_index", "privacy_mode"])?;
    let slugs = parse_required_string_list(args, "open_nodes", "slugs")?;
    let auto_index = parse_optional_bool(args, "open_nodes", "auto_index")?.unwrap_or(false);
    let privacy_mode =
        parse_optional_privacy_mode(args, "open_nodes", "privacy_mode")?.unwrap_or_default();
    let engine = state.bound_engine()?;
    engine.ensure_index_ready(auto_index)?;
    let mut payload = serde_json::to_value(json!({
        "nodes": engine.open_nodes(&slugs)?
    }))?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

pub(crate) fn read_graph(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(args, "read_graph", &["slugs", "auto_index", "privacy_mode"])?;
    let slugs = parse_required_string_list(args, "read_graph", "slugs")?;
    let auto_index = parse_optional_bool(args, "read_graph", "auto_index")?.unwrap_or(false);
    let privacy_mode =
        parse_optional_privacy_mode(args, "read_graph", "privacy_mode")?.unwrap_or_default();
    let engine = state.bound_engine()?;
    engine.ensure_index_ready(auto_index)?;
    let mut payload = serde_json::to_value(engine.read_graph(&slugs)?)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}
