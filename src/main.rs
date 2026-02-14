mod formatter_manager;
mod mcp;
mod protocol;
mod targets;
mod tools;

use crate::formatter_manager::FormatterManager;
use crate::mcp::handle_request;
use crate::protocol::{read_mcp_message, write_mcp_message};
use std::io::{self, BufReader};

fn main() -> io::Result<()> {
    let manager =
        FormatterManager::new().map_err(|e| io::Error::other(format!("Init error: {e}")))?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    loop {
        let message = match read_mcp_message(&mut reader) {
            Ok(Some(msg)) => msg,
            Ok(None) => break,
            Err(err) => {
                eprintln!("Failed to read MCP message: {err}");
                break;
            }
        };

        if let Some(response) = handle_request(&message, &manager) {
            write_mcp_message(&mut writer, &response)?;
        }
    }

    Ok(())
}
