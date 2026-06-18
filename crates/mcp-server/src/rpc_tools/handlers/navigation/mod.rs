use anyhow::Result;
use serde_json::Value;

use crate::ServerState;

pub(crate) fn call_path(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::call_path::call_path(args, state)
}

pub(crate) fn concept_cluster(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::investigation::concept_cluster(args, state)
}

pub(crate) fn constraint_evidence(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::investigation::constraint_evidence(args, state)
}

pub(crate) fn contract_trace(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::investigation::contract_trace(args, state)
}

pub(crate) fn divergence_report(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::investigation::divergence_report(args, state)
}

pub(crate) fn related_files(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::related_files::related_files(args, state)
}

pub(crate) fn related_files_v2(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::related_files::related_files_v2(args, state)
}

pub(crate) fn route_trace(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::investigation::route_trace(args, state)
}

pub(crate) fn symbol_body(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::investigation::symbol_body(args, state)
}

pub(crate) fn symbol_lookup(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::symbol_lookup::symbol_lookup(args, state)
}

pub(crate) fn symbol_lookup_v2(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::symbol_lookup::symbol_lookup_v2(args, state)
}

pub(crate) fn symbol_references(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::symbol_references::symbol_references(args, state)
}

pub(crate) fn symbol_references_v2(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::symbol_references::symbol_references_v2(args, state)
}
