#[path = "handlers/bootstrap.rs"]
mod bootstrap;
#[path = "handlers/diagnostics.rs"]
mod diagnostics;
#[path = "handlers/modes.rs"]
mod modes;
#[path = "handlers/project.rs"]
mod project;
#[path = "handlers/read.rs"]
mod read;
#[path = "handlers/write.rs"]
mod write;

pub(crate) use bootstrap::{context_pack, decision_log, recent_changes, risk_hotspots};
pub(crate) use diagnostics::{
    index_status, memory_status, migrate_memory_root, preflight, rebuild_index,
};
pub(crate) use project::{project_brief, set_project};
pub(crate) use read::{open_nodes, read_graph, search_memory};
pub(crate) use write::{add_observation, create_node, link_nodes, unlink_nodes, update_node};
