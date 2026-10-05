//! Which project an MCP call acts on, and whether it is current with disk
//! (architecture plan 01): a call's optional `path` and its tool's target
//! decide it before the handler runs.

use serde_json::{Value, json};
use specforge_mcp::McpServer;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn call_tool(server: &mut McpServer, name: &str, arguments: Value) -> Value {
    call(
        server,
        "tools/call",
        json!({"name": name, "arguments": arguments}),
    )
}

fn initialize(server: &mut McpServer, root: &Path) {
    let resp = call(
        server,
        "initialize",
        json!({"projectRoot": root.to_str().unwrap()}),
    );
    assert!(resp["error"].is_null(), "{resp}");
}

/// A server that serves no project: initialize names none and there is no
/// default root.
fn serving_nothing() -> McpServer {
    let mut server = McpServer::new();
    let resp = call(&mut server, "initialize", json!({}));
    assert!(resp["error"].is_null(), "{resp}");
    server
}

/// A project with `specforge.json` listing `extensions` and one spec file.
fn project(extensions: &[&str], spec: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    write_config(dir.path(), extensions);
    fs::write(dir.path().join("main.spec"), spec).unwrap();
    dir
}

fn write_config(root: &Path, extensions: &[&str]) {
    let config = json!({"name": "served", "version": "0.1.0", "extensions": extensions});
    fs::write(root.join("specforge.json"), config.to_string()).unwrap();
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The text of a tool result's first block.
fn tool_text(resp: &Value) -> String {
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text block: {resp}"))
        .to_string()
}

/// The diagnostic codes a `specforge.validate` result reports.
fn validate_codes(server: &mut McpServer, arguments: Value) -> Vec<String> {
    let resp = call_tool(server, "specforge.validate", arguments);
    let diagnostics: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    diagnostics
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap().to_string())
        .collect()
}

/// Whether `specforge.query` finds `id` in the graph the server serves.
fn finds(server: &mut McpServer, id: &str) -> bool {
    let resp = call_tool(server, "specforge.query", json!({"entity_id": id}));
    let payload: Value = resp["result"]["content"][0]["text"]
        .as_str()
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or(Value::Null);
    payload["nodes"]
        .as_array()
        .is_some_and(|nodes| nodes.iter().any(|n| n["id"] == id))
}

/// The root the server serves, canonical.
fn served_root(server: &McpServer) -> Option<PathBuf> {
    server.state().project_root.as_deref().map(canonical)
}

/// A project whose Rust tests `@specforge/cargo-test` collects, with the
/// report an earlier `cargo test` wrote.
fn collect_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let config = json!({
        "name": "c",
        "version": "0.1.0",
        "extensions": ["@specforge/software", "@specforge/testing", "@specforge/cargo-test"]
    });
    fs::write(root.join("specforge.json"), config.to_string()).unwrap();
    fs::write(
        root.join("app.spec"),
        "behavior alpha \"Alpha\" {\n  verify unit \"works\"\n}\n",
    )
    .unwrap();
    fs::write(root.join("Cargo.toml"), "").unwrap();
    fs::create_dir_all(root.join("target/specforge")).unwrap();
    fs::write(
        root.join("target/specforge/t.json"),
        json!({"entries": [{"entity_id": "alpha", "test_name": "works", "status": "pass"}]})
            .to_string(),
    )
    .unwrap();
    dir
}

#[test]
fn validate_with_a_path_while_nothing_is_served_serves_it() {
    let other = project(&[], "behavior adopted \"Adopted\" {\n}\n");
    let mut server = serving_nothing();
    assert_eq!(served_root(&server), None);

    let resp = call_tool(
        &mut server,
        "specforge.validate",
        json!({"path": other.path().to_str().unwrap()}),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp}");

    assert!(finds(&mut server, "adopted"));
    assert_eq!(served_root(&server), Some(canonical(other.path())));
}

#[test]
fn collect_with_another_path_keeps_serving_this_one() {
    let served = project(&[], "behavior login \"Login\" {\n}\n");
    let other = collect_project();
    let mut server = McpServer::new();
    initialize(&mut server, served.path());

    let resp = call_tool(
        &mut server,
        "specforge.collect",
        json!({"path": other.path().to_str().unwrap()}),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    assert!(other.path().join("specforge-report.json").is_file());

    assert_eq!(served_root(&server), Some(canonical(served.path())));
    assert!(server.state().graph().node("login").is_some());
    assert!(server.state().graph().node("alpha").is_none());
}

#[test]
fn format_with_another_path_leaves_the_served_spans() {
    // Blank lines inside each block: formatting removes them.
    let unformatted = "behavior login \"Login\" {\n\n\n\n}\n";
    let served = project(&[], unformatted);
    let other = project(&[], "behavior other \"Other\" {\n\n\n\n}\n");
    let mut server = McpServer::new();
    initialize(&mut server, served.path());
    let span = |server: &McpServer| {
        server
            .state()
            .graph()
            .node("login")
            .map(|n| n.source_span.clone())
    };
    let before = span(&server);

    let resp = call_tool(
        &mut server,
        "specforge.format",
        json!({"path": other.path().to_str().unwrap()}),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    let formatted = fs::read_to_string(other.path().join("main.spec")).unwrap();
    assert_ne!(
        formatted.lines().count(),
        5,
        "format rewrote the other project"
    );

    assert_eq!(
        fs::read_to_string(served.path().join("main.spec")).unwrap(),
        unformatted
    );
    assert_eq!(span(&server), before);
    assert!(server.state().graph().node("other").is_none());
}

#[test]
fn use_cached_serves_the_last_compile_when_diagnostics_exist() {
    // No extension: the compile reports I002, so the served project has
    // diagnostics to serve.
    let dir = project(&[], "behavior login \"Login\" {\n}\n");
    let mut server = McpServer::new();
    initialize(&mut server, dir.path());
    assert!(validate_codes(&mut server, json!({})).contains(&"I002".to_string()));

    // A dangling reference written since: a compile reports E003.
    fs::write(
        dir.path().join("main.spec"),
        "behavior login \"Login\" {\n  invariants [missing_one]\n}\n",
    )
    .unwrap();
    let cached = validate_codes(&mut server, json!({"use_cached": true}));
    assert!(!cached.contains(&"E003".to_string()), "{cached:?}");

    let fresh = validate_codes(&mut server, json!({}));
    assert!(fresh.contains(&"E003".to_string()), "{fresh:?}");
}

#[test]
fn doctor_use_cached_reports_the_last_compile() {
    let dir = project(&[], "behavior login \"Login\" {\n}\n");
    let mut server = McpServer::new();
    initialize(&mut server, dir.path());

    // An extension enabled since, which is not installed: E028.
    write_config(dir.path(), &["@acme/missing"]);
    let load_failures = |server: &mut McpServer, arguments: Value| -> Vec<Value> {
        let resp = call_tool(server, "specforge.doctor", arguments);
        let report: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
        report["load_failures"].as_array().unwrap().clone()
    };
    let cached = load_failures(&mut server, json!({"use_cached": true}));
    assert!(cached.is_empty(), "{cached:?}");

    let fresh = load_failures(&mut server, json!({}));
    assert!(fresh.iter().any(|f| f["code"] == "E028"), "{fresh:?}");
}

#[test]
fn extension_tool_without_a_project_is_refused() {
    use specforge_mcp::types::McpToolDescriptor;
    use specforge_registry::{SurfaceRegistryEntry, SurfaceType};

    let mut server = serving_nothing();
    let state = server.state_mut();
    state.tool_registry.push(McpToolDescriptor {
        name: "test.list_items".into(),
        description: "List all items".into(),
        input_schema: json!({"type": "object", "properties": {}}),
        category: Some("core".into()),
        source: Some("@test/ext".into()),
        ..Default::default()
    });
    state.edit_environment(|env| {
        env.registries.surfaces.push(SurfaceRegistryEntry {
            extension_name: "@test/ext".into(),
            surface_type: SurfaceType::McpTool,
            contribution_name: "test.list_items".into(),
            export_name: "mcp__list_items".into(),
        });
    });

    let resp = call_tool(&mut server, "test.list_items", json!({}));
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "precondition_failed", "{error}");
}
