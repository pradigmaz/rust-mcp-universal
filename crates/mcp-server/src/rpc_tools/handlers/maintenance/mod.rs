use anyhow::Result;
use serde_json::Value;

use crate::ServerState;

pub(crate) fn db_maintenance(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::maintenance_impl::db_maintenance(args, state)
}

pub(crate) fn preflight(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::maintenance_impl::preflight(args, state)
}
