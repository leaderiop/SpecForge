//! The management operations over MCP (architecture plan 05): the
//! extensions listing, doctor, remove, validate and stats over the served
//! project, and the inference tools without one.

use serde_json::{Value, json};
use specforge_mcp::McpServer;
use specforge_test::prelude::*;

use crate::fake_extension::{self, FakeExtension};
use crate::tool_errors::mcp_error;

const GREET: &str = "@sdk/greet";

fn call_tool(server: &mut McpServer, tool_name: &str, args: Value) -> Value {
    let req = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "tools/call",
        "params": { "name": tool_name, "arguments": args }
    });
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

/// A tool's JSON answer.
fn answer(resp: &Value) -> Value {
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text block: {resp}"));
    serde_json::from_str(text).unwrap_or_else(|e| panic!("not JSON ({e}): {resp}"))
}

/// A server initialized over a temp project whose `specforge.json` is
/// `config` as written (valid JSON or not), with one behavior.
fn server_over_text(config: &str) -> (McpServer, std::path::PathBuf) {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("specforge.json"), config).unwrap();
    std::fs::write(
        dir.path().join("test.spec"),
        "behavior alpha \"Alpha\" {\n}\n",
    )
    .unwrap();
    let root = dir.path().to_path_buf();
    std::mem::forget(dir); // outlives the test
    let mut server = McpServer::new();
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"projectRoot": root.to_str().unwrap()}});
    server.handle_message(&req.to_string());
    (server, root)
}

/// [`server_over_text`] over a config written from JSON.
fn server_over(config: Value) -> (McpServer, std::path::PathBuf) {
    server_over_text(&config.to_string())
}

fn greet_blob() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/greet-extension/greet.wasm")
}

/// A served project with `@sdk/greet` installed from its local blob: a lock
/// entry, the binary under `.specforge/extensions/` and a `specforge.json`
/// entry.
fn server_with_greet_installed() -> (McpServer, std::path::PathBuf) {
    let (mut server, root) =
        server_over(json!({"name": "t", "version": "0.1.0", "extensions": []}));
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": greet_blob().to_str().unwrap()}),
    );
    assert_eq!(resp["result"]["isError"], false, "install failed: {resp}");
    assert!(root.join(".specforge/extensions").join(GREET).exists());
    (server, root)
}

/// Doctor's findings, without the z3 probe's (it depends on PATH).
fn project_findings(report: &Value) -> Vec<Value> {
    report["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("no findings: {report}"))
        .iter()
        .filter(|f| f["code"] != "z3_missing")
        .cloned()
        .collect()
}

#[specforge_test(
    behavior = "provide_mcp_extensions_tool",
    verify = "specforge.extensions lists all installed extensions"
)]
fn extensions_lists_the_lock_entries_and_the_kinds_in_the_graph() {
    let config = json!({"name": "t", "version": "0.1.0", "extensions": ["@specforge/software"]});

    // No lock: no lock entries; the graph's one behavior.
    let (mut server, _root) = server_over(config.clone());
    let listed = answer(&call_tool(&mut server, "specforge.extensions", json!({})));
    assert_eq!(listed["lock_file_entries"], json!([]), "{listed}");
    assert_eq!(
        listed["entity_kinds_in_graph"],
        json!(["behavior"]),
        "{listed}"
    );

    // A lock: each entry as {name, version}, in lock order.
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    std::fs::write(
        dir.path().join("test.spec"),
        "behavior alpha \"Alpha\" {\n}\ninvariant beta \"Beta\" {\n}\n",
    )
    .unwrap();
    let lock = json!({"lockfile_version": 1, "entries": [
        {"name": "@acme/zeta", "version": "2.0.0", "source": "registry", "wasm_hash": "h1"},
        {"name": "@acme/alpha", "version": "1.0.0", "source": "registry", "wasm_hash": "h2"},
    ]});
    std::fs::write(dir.path().join("specforge.lock"), lock.to_string()).unwrap();
    let mut server = McpServer::new();
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"projectRoot": dir.path().to_str().unwrap()}});
    server.handle_message(&req.to_string());

    let listed = answer(&call_tool(&mut server, "specforge.extensions", json!({})));
    assert_eq!(
        listed["lock_file_entries"],
        json!([
            {"name": "@acme/zeta", "version": "2.0.0"},
            {"name": "@acme/alpha", "version": "1.0.0"},
        ]),
        "{listed}"
    );
    assert_eq!(
        listed["entity_kinds_in_graph"],
        json!(["behavior", "invariant"]),
        "{listed}"
    );
}

#[specforge_test(
    behavior = "read_views_over_the_project_view",
    verify = "specforge stats and specforge.stats report the same numbers"
)]
fn stats_and_validate_count_the_served_projects_surface_conflicts() {
    let (mut server, _ext, _dir) = fake_extension::initialized(
        FakeExtension::new().with_output("mcp__check", json!({"checked": true})),
    );
    let reported = server.state().diagnostics();
    let infos = reported
        .iter()
        .filter(|d| d.severity == specforge_common::Severity::Info)
        .count();
    assert!(reported.iter().any(|d| d.code == "I017"), "{reported:?}");

    let stats = answer(&call_tool(&mut server, "specforge.stats", json!({})));
    assert_eq!(stats["diagnostic_summary"]["infos"], infos, "{stats}");

    let validated = answer(&call_tool(&mut server, "specforge.validate", json!({})));
    let codes: Vec<&str> = validated
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"I017"), "{validated}");
}

// pins R2; flipped by 05-T6: remove_extension uninstalls the binary and
// empties the lock, and only then finds specforge.json unreadable.
#[test]
fn remove_extension_with_an_unreadable_config_uninstalls_before_it_fails() {
    let (mut server, root) = server_with_greet_installed();
    std::fs::write(
        root.join("specforge.json"),
        r#"{ "extensions": ["@sdk/greet",  }"#,
    )
    .unwrap();

    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": GREET}),
    );

    let error = mcp_error(&resp);
    assert_eq!(error["code"], "schema_mismatch", "{error}");
    let lock = std::fs::read_to_string(root.join("specforge.lock")).unwrap();
    assert!(!lock.contains(GREET), "{lock}");
}

#[specforge_test(
    behavior = "provide_mcp_infer_progress_tool",
    verify = "graceful handling when specforge-infer.json is missing"
)]
fn infer_tools_without_a_project_answer_their_empty_documents() {
    let mut server = McpServer::new();
    server.handle_message(
        &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}).to_string(),
    );

    let progress = answer(&call_tool(
        &mut server,
        "specforge.infer_progress",
        json!({}),
    ));
    assert_eq!(
        progress["summary"],
        json!({"files_total": 0, "files_analyzed": 0, "entities_produced": 0}),
        "{progress}"
    );
    assert_eq!(
        progress["message"], "No project root available",
        "{progress}"
    );

    let gaps = answer(&call_tool(&mut server, "specforge.infer_gaps", json!({})));
    assert_eq!(gaps["total_pub_items"], 0, "{gaps}");
    assert_eq!(gaps["message"], "No project root available", "{gaps}");
}

// pins R3; flipped by 05-T5: an unparsable specforge.json is served as "no
// extensions configured", and doctor finds nothing.
#[test]
fn validate_over_an_unparsable_config_reports_only_i002_today() {
    let (mut server, _root) = server_over_text(r#"{ "extensions": ["#);

    let resp = call_tool(&mut server, "specforge.validate", json!({}));
    let validated = answer(&resp);
    let codes: Vec<&str> = validated
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["I002"], "{validated}");
    assert_eq!(
        resp["result"]["_meta"]["specforge/check"]["warnings"], 0,
        "{resp}"
    );
    assert_eq!(
        resp["result"]["_meta"]["specforge/check"]["ok"], true,
        "{resp}"
    );

    let report = answer(&call_tool(&mut server, "specforge.doctor", json!({})));
    assert_eq!(project_findings(&report), Vec::<Value>::new(), "{report}");
}
