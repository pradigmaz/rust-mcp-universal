use std::io;

use anyhow::Result;
use clap::Parser;

mod protocol;
mod rpc_tools;
mod state;
mod transport;

#[cfg(test)]
pub(crate) use protocol::{PROTOCOL_VERSION, RpcRequest, RpcResponse, handle_request};
pub(crate) use protocol::{parse_error_response, process_raw_message};
pub(crate) use state::{App, ServerState};
#[cfg(test)]
pub(crate) use transport::{WireMode, read_framed_message, run_stdio_server, write_framed_message};

fn main() -> Result<()> {
    let app = App::parse();
    app.validate_runtime_flags()?;
    let mut state = ServerState::new(app.project_path, app.db_path);

    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    transport::run_stdio_server(&mut reader, &mut writer, &mut state)
}

#[cfg(test)]
#[path = "main_tests/mod.rs"]
mod tests;
