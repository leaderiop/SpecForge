//! C9-07 acceptance: when watch writes a newer `.specforge/graph.json`
//! marker, MCP tools serve a freshly compiled graph instead of the frozen
//! initialize-time one.

use serde_json::{Value, json};
use specforge_mcp::McpServer;
use std::fs;
use tempfile::TempDir;

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

#[test]
fn newer_snapshot_recompiles_the_graph() {
    let dir = TempDir::new().unwrap();
    let spec_dir = dir.path().join("spec");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("test.spec"),
        r#"behavior hello_world "Hello World" {
    contract "The system MUST greet the user"
    verify unit "greets user"
}
"#,
    )
    .unwrap();
    // A marker written by watch, timestamped AFTER the initialize compile.
    let marker_dir = dir.path().join(".specforge");
    fs::create_dir_all(&marker_dir).unwrap();
    fs::write(
        marker_dir.join("graph.json"),
        format!(
            r#"{{"updated_at":{}}}"#,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
                + 5_000, // clearly in the future relative to initialize
        ),
    )
    .unwrap();
    // The new entity the snapshot signals.
    fs::write(
        spec_dir.join("added.spec"),
        "feature freshness \"Freshness\" {\n  behaviors [hello_world]\n}\n",
    )
    .unwrap();

    let project_root = dir.path().to_str().unwrap();
    let mut server = McpServer::new();
    call(
        &mut server,
        "initialize",
        json!({"projectRoot": project_root}),
    );

    // Stale read (no sleep): the marker is newer, so the tool must have
    // refreshed — `freshness` is present in the graph.
    let resp = call(
        &mut server,
        "tools/call",
        json!({"name": "specforge.query", "arguments": {"entity_id": "freshness"}}),
    );
    let result = &resp["result"];
    assert!(
        !result.is_null(),
        "refreshed graph must resolve the new entity: {result}"
    );
    assert!(
        result.to_string().contains("freshness"),
        "tool must see the refreshed graph: {result}"
    );
}
