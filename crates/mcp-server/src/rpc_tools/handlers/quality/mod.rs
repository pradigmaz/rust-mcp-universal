use anyhow::Result;
use serde_json::Value;

use crate::ServerState;

pub(crate) fn mark_signal_memory(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::signal_memory::mark_signal_memory(args, state)
}

pub(crate) fn quality_hotspots(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::quality_hotspots::quality_hotspots(args, state)
}

pub(crate) fn quality_snapshot(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::quality_snapshot::quality_snapshot(args, state)
}

pub(crate) fn rule_violations(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::rule_violations::rule_violations(args, state)
}

pub(crate) fn sensitive_data(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::sensitive_data::sensitive_data(args, state)
}

pub(crate) fn signal_memory(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::signal_memory::signal_memory(args, state)
}
