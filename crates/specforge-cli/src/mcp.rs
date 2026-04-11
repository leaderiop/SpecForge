use specforge_mcp::McpServer;
use std::io::{self, BufRead, Write};
use std::path::Path;

pub fn run(path: &Path) -> i32 {
    let mut server = McpServer::new();

    // Auto-initialize with project root
    let init_params = serde_json::json!({
        "projectRoot": path.to_str().unwrap_or(".")
    });
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize",
        "params": init_params
    });
    if let Some(resp) = server.handle_message(&init_req.to_string()) {
        // Write init response to stdout
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let _ = writeln!(out, "{}", resp);
        let _ = out.flush();
    }

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
