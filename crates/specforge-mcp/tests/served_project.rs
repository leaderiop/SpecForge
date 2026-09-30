//! The project an MCP server serves is replaced whole, by one install, or
//! not at all (`mcp_served_project_consistency`, plan 01 P7).

use serde_json::{Value, json};
use specforge_mcp::McpServer;
use specforge_test::prelude::*;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

use crate::fake_extension::{self, EXT, FakeExtension};

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

fn tool_names(server: &mut McpServer) -> Vec<String> {
    let resp = call(server, "tools/list", json!({}));
    resp["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect()
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

/// Watch's snapshot marker, with an mtime ahead of any compile.
fn write_marker(root: &Path) {
    let marker = root.join(".specforge").join("graph.json");
    fs::create_dir_all(marker.parent().unwrap()).unwrap();
    fs::write(&marker, "{}").unwrap();
    fs::File::options()
        .write(true)
        .open(&marker)
        .unwrap()
        .set_modified(SystemTime::now() + Duration::from_secs(5))
        .unwrap();
}

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "a refresh after a newer watch snapshot updates the extension tools listed"
)]
fn a_refresh_lists_the_tools_of_the_extensions_it_loaded() {
    let ext = Arc::new(FakeExtension::new());
    let dir = project(&[], "");
    let mut server = fake_extension::server_with(&ext);
    initialize(&mut server, dir.path());
    let before = tool_names(&mut server);
    assert!(
        !before.iter().any(|t| t.starts_with("specforge.cmds.")),
        "{before:?}"
    );

    // The project enables the extension, and watch says so.
    write_config(dir.path(), &[EXT]);
    write_marker(dir.path());
    let resp = call_tool(&mut server, "specforge.stats", json!({}));
    assert!(resp["error"].is_null(), "{resp}");

    let after = tool_names(&mut server);
    for tool in ["specforge.cmds.check", "specforge.cmds.report"] {
        assert!(
            after.iter().any(|t| t == tool),
            "{tool} is not listed after the refresh: {after:?}"
        );
    }
    // And the auto-promoted tool dispatches: the refresh registered its
    // surface, not only its descriptor.
    let entries = &server.state().surface_entries;
    assert!(
        entries
            .iter()
            .any(|e| e.contribution_name == "specforge.cmds.report"),
        "{entries:?}"
    );

    // A second refresh lists each tool once.
    write_marker(dir.path());
    call_tool(&mut server, "specforge.stats", json!({}));
    let again = tool_names(&mut server);
    let reports = again
        .iter()
        .filter(|t| *t == "specforge.cmds.report")
        .count();
    assert_eq!(reports, 1, "{again:?}");
}

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "analyze notifies subscribers when the diagnostics it compiled changed"
)]
fn analyze_notifies_the_diagnostics_it_compiled() {
    let dir = project(&[], "behavior login \"Login\" {\n}\n");
    let mut server = McpServer::new();
    initialize(&mut server, dir.path());
    call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://diagnostics"}),
    );
    server.take_notifications();

    // A dangling reference: the next compile reports an E003.
    fs::write(
        dir.path().join("main.spec"),
        "behavior login \"Login\" {\n  invariants [session_limit]\n}\n",
    )
    .unwrap();
    let resp = call_tool(&mut server, "specforge.analyze", json!({}));
    assert!(resp["error"].is_null(), "{resp}");

    let sent = server.take_notifications();
    let changed: Vec<&Value> = sent
        .iter()
        .filter(|n| n["method"] == "specforge/diagnosticsChanged")
        .collect();
    assert_eq!(changed.len(), 1, "{sent:?}");
    let added = changed[0]["params"]["added"].as_array().unwrap();
    assert!(added.iter().any(|d| d["code"] == "E003"), "{added:?}");
}

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "validate with a path to another project leaves the served project in place"
)]
fn validate_on_another_project_keeps_serving_this_one() {
    let served = project(&[], "behavior login \"Login\" {\n}\n");
    let other = project(
        &[],
        "behavior other \"Other\" {\n  invariants [missing_one]\n}\n",
    );
    let mut server = McpServer::new();
    initialize(&mut server, served.path());

    let resp = call_tool(
        &mut server,
        "specforge.validate",
        json!({"path": other.path().to_str().unwrap()}),
    );
    // The other project's diagnostics are the answer...
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let diagnostics: Value = serde_json::from_str(text).unwrap();
    assert!(
        diagnostics
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E003"),
        "{diagnostics}"
    );

    // ...and the server still serves its own project.
    let state = server.state();
    assert_eq!(state.project_root.as_deref(), Some(served.path()));
    assert!(state.graph.node("login").is_some());
    assert!(state.graph.node("other").is_none());
    assert!(state.diagnostics.iter().all(|d| d.code != "E003"));
}

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "a mutation tool that wrote files leaves the server serving what is on disk"
)]
fn a_format_that_wrote_files_is_served() {
    // Three blank lines inside the block: formatting removes them.
    let dir = project(&[], "behavior login \"Login\" {\n\n\n\n}\n");
    let mut server = McpServer::new();
    initialize(&mut server, dir.path());
    let end_line = |server: &McpServer| {
        server
            .state()
            .graph
            .node("login")
            .map(|n| n.source_span.end_line)
    };
    let before = end_line(&server);

    let resp = call_tool(&mut server, "specforge.format", json!({}));
    assert!(resp["error"].is_null(), "{resp}");
    let on_disk = fs::read_to_string(dir.path().join("main.spec")).unwrap();
    assert_ne!(on_disk.lines().count(), 5, "format rewrote the file");

    let after = end_line(&server);
    assert_ne!(
        after, before,
        "the server still serves the unformatted file"
    );
    assert_eq!(after, Some(on_disk.lines().count()));
}
