use specforge_mcp::McpServer;
use std::io::{self, BufRead, Write};
use std::path::Path;

pub fn run(path: &Path) -> i32 {
    // The client drives the handshake; its `initialize` compiles `path`
    // unless it names a `projectRoot` of its own.
    let mut server = McpServer::with_project_root(path.to_path_buf());

    // Stdio loop: read JSON-RPC from stdin, write responses to stdout
    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(response) = server.handle_message(trimmed) {
            let mut out = stdout.lock();
            let _ = writeln!(out, "{}", response);
            let _ = out.flush();
        }
    }

    0
}
