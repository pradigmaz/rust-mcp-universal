mod indexing;
mod maintenance;
mod modes;
mod preflight;
mod query;

#[path = "mod/dispatch.rs"]
mod dispatch;
#[path = "mod/preflight_helpers.rs"]
mod preflight_helpers;
#[cfg(test)]
#[path = "mod/preflight_tests.rs"]
mod preflight_tests;

use anyhow::Result;

use crate::args::App;

pub(crate) fn run(app: App) -> Result<()> {
    let prepared = preflight_helpers::prepare(app)?;
    dispatch::run(prepared)
}
