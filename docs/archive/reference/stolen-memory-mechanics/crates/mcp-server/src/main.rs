use std::io;

use anyhow::Result;

mod path_input;
mod protocol;
mod rpc_tools;
mod state;
mod transport;

pub(crate) use protocol::{parse_error_response, process_raw_message};
pub(crate) use state::ServerState;

fn main() -> Result<()> {
    let mut state = ServerState::new();
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    transport::run_stdio_server(&mut reader, &mut writer, &mut state)
}
