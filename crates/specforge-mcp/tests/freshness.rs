//! C9-07 acceptance: when watch writes a newer `.specforge/graph.json`
//! marker, MCP tools serve a freshly compiled graph instead of the frozen
//! initialize-time one.

use serde_json::{Value, json};
use specforge_mcp::McpServer;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

/// Whether `specforge.query` finds `id` in the graph the server serves.
fn finds(server: &mut McpServer, id: &str) -> bool {
    let resp = call(
        server,
        "tools/call",
        json!({"name": "specforge.query", "arguments": {"entity_id": id}}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap_or("");
    let payload: Value = serde_json::from_str(text).unwrap_or(Value::Null);
    payload["nodes"]
        .as_array()
        .is_some_and(|nodes| nodes.iter().any(|n| n["id"] == id))
}

/// Write the watch marker with an mtime `ahead` of now, so it is newer
/// than any compile regardless of the filesystem's timestamp resolution.
fn write_marker(root: &Path, ahead: Duration) {
    let marker_dir = root.join(".specforge");
    fs::create_dir_all(&marker_dir).unwrap();
    let marker = marker_dir.join("graph.json");
    let stamp = SystemTime::now() + ahead;
    let millis = stamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    fs::write(&marker, format!(r#"{{"updated_at":{millis}}}"#)).unwrap();
    fs::File::options()
        .write(true)
        .open(&marker)
        .unwrap()
        .set_modified(stamp)
        .unwrap();
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

    let project_root = dir.path().to_str().unwrap();
    let mut server = McpServer::new();
    call(
        &mut server,
        "initialize",
        json!({"projectRoot": project_root}),
    );
    assert!(finds(&mut server, "hello_world"), "initialize compiled");

    // A file added after initialize, with no marker yet: the server keeps
    // serving the initialize-time graph.
    fs::write(
        spec_dir.join("added.spec"),
        "feature freshness \"Freshness\" {\n  behaviors [hello_world]\n}\n",
    )
    .unwrap();
    assert!(
        !finds(&mut server, "freshness"),
        "without a newer marker the graph must not be recompiled"
    );

    // Watch writes a marker newer than the last compile: the next tool call
    // refreshes and sees the added entity.
    write_marker(dir.path(), Duration::from_secs(5));
    assert!(
        finds(&mut server, "freshness"),
        "a newer marker must refresh the graph"
    );

    // A file deleted since: the next newer marker drops its entities.
    fs::remove_file(spec_dir.join("added.spec")).unwrap();
    write_marker(dir.path(), Duration::from_secs(10));
    assert!(
        !finds(&mut server, "freshness"),
        "a refresh must drop the entities of a deleted file"
    );
}
