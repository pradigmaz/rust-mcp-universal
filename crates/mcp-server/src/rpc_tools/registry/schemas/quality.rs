#[path = "quality/rules.rs"]
mod rules;
#[path = "quality/signal.rs"]
mod signal;
#[path = "quality/snapshot.rs"]
mod snapshot;

pub(crate) use rules::{quality_hotspots_schema, rule_violations_schema};
pub(crate) use signal::{mark_signal_memory_schema, signal_memory_schema};
pub(crate) use snapshot::quality_snapshot_schema;
