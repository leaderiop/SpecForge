use serde_json::{Value, json};
use specforge_common::SourceSpan;
use specforge_graph::{Graph, Node};
use specforge_mcp::McpServer;
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test::prelude::*;

// Leak a per-test temp project: process exits make cleanup unnecessary, and
// a real project root is required now that ops perform real work.
fn attach_project(state: &mut specforge_mcp::state::McpState) {
    let dir = tempfile::TempDir::new().unwrap();
    let config = json!({"name":"t","version":"0.1.0","extensions":[]});
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    std::fs::write(
        dir.path().join("test.spec"),
        "behavior alpha \"Alpha\" {\n}\nfeature beta \"Beta\" {\n    behaviors [alpha]\n}\n",
    )
    .unwrap();
    let root = dir.path().to_path_buf();
    std::mem::forget(dir); // outlives the test
    state.project_root = Some(root);
}

fn test_server() -> McpServer {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    let state = server.state_mut();
    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId {
            raw: "alpha".into(),
        },
        kind: EntityKind {
            raw: "behavior".into(),
        },
        title: Some("Alpha".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "test.spec".into(),
            start_line: 1,
            start_col: 0,
            end_line: 5,
            end_col: 0,
        },
        methods: Vec::new(),
    });
    state.serve_graph(graph, Vec::new());
    attach_project(state);
    server
}

/// A server initialized over a temp project with `config` as its
/// specforge.json.
fn server_over(config: Value) -> (McpServer, std::path::PathBuf) {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
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

fn software_project() -> (McpServer, std::path::PathBuf) {
    server_over(json!({"name": "t", "version": "0.1.0", "extensions": ["@specforge/software"]}))
}

/// `specforge.extensions`' entries as (name, status).
fn listed_extensions(server: &mut McpServer) -> Vec<(String, String)> {
    let resp = call_tool(server, "specforge.extensions", json!({}));
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    parsed["extensions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["name"].as_str().unwrap().to_string(),
                e["status"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn invoked(server: &McpServer, tool: &str) -> bool {
    server
        .state()
        .events
        .iter()
        .any(|e| e.name == "mcp_tool_invoked" && e.params["toolName"] == tool)
}

fn call_tool(server: &mut McpServer, tool_name: &str, args: Value) -> Value {
    let req = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "tools/call",
        "params": { "name": tool_name, "arguments": args }
    });
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn tool_text(resp: &Value) -> String {
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

// --- specforge.extensions ---

// B:provide_mcp_extensions_tool — verify unit "returns extensions list"
#[specforge_test(
    behavior = "provide_mcp_extensions_tool",
    verify = "specforge.extensions lists all installed extensions"
)]
fn extensions_returns_list() {
    let (mut server, _root) = software_project();
    let resp = call_tool(&mut server, "specforge.extensions", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let extensions = parsed["extensions"].as_array().unwrap();
    assert_eq!(extensions.len(), 1, "{parsed}");
    let software = &extensions[0];
    assert_eq!(software["name"], "@specforge/software");
    assert_eq!(software["status"], "loaded");
    assert!(
        !software["version"].as_str().unwrap().is_empty(),
        "{software}"
    );
    assert_eq!(
        software["entity_kinds"],
        // The keywords its kinds registered, sorted, as the CLI lists them.
        json!(["behavior", "event", "invariant", "port", "type"])
    );

    // With nothing configured, nothing is listed.
    let mut bare = test_server();
    assert_eq!(listed_extensions(&mut bare), vec![]);
}

// --- specforge.providers ---

// B:provide_mcp_providers_tool — verify unit "returns providers list"
#[specforge_test(
    behavior = "provide_mcp_providers_tool",
    verify = "specforge.providers lists all configured providers"
)]
fn providers_returns_list() {
    let (mut server, _root) = server_over(json!({
        "name": "t", "version": "0.1.0", "extensions": [],
        "providers": [
            {"alias": "tracker", "scheme": "jira", "extension": "@acme/jira"},
            {"alias": "code", "scheme": "gh", "extension": "@acme/gh"}
        ]
    }));
    let resp = call_tool(&mut server, "specforge.providers", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let listed: Vec<(&str, &str)> = parsed["providers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["alias"].as_str().unwrap(), p["scheme"].as_str().unwrap()))
        .collect();
    assert_eq!(listed, vec![("tracker", "jira"), ("code", "gh")]);
    assert_eq!(parsed["count"], 2);
}

// --- specforge.doctor ---

#[test]
fn doctor_returns_report() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.doctor", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["extensions_ok"].is_boolean());
    assert!(parsed["findings"].is_array());
}

// --- specforge.collect ---

/// A project whose Rust tests `@specforge/cargo-test` collects, with a
/// report already written by an earlier `cargo test`.
fn collect_project() -> std::path::PathBuf {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    std::mem::forget(dir); // outlives the test
    let config = json!({
        "name": "t",
        "version": "0.1.0",
        "extensions": ["@specforge/software", "@specforge/testing", "@specforge/cargo-test"]
    });
    std::fs::write(root.join("specforge.json"), config.to_string()).unwrap();
    std::fs::write(
        root.join("app.spec"),
        "behavior alpha \"Alpha\" {\n  verify unit \"works\"\n}\n",
    )
    .unwrap();
    std::fs::write(root.join("Cargo.toml"), "").unwrap();
    std::fs::create_dir_all(root.join("target/specforge")).unwrap();
    std::fs::write(
        root.join("target/specforge/t.json"),
        json!({"entries": [
            {"entity_id": "alpha", "test_name": "works", "status": "pass"},
            {"entity_id": "ghost", "test_name": "stale", "status": "pass"}
        ]})
        .to_string(),
    )
    .unwrap();
    root
}

// B:provide_mcp_collect_tool — verify unit "returns collect result"
#[specforge_test(
    behavior = "provide_mcp_collect_tool",
    verify = "specforge.collect parses test results and maps to entities"
)]
fn collect_returns_result() {
    let mut server = test_server();
    let root = collect_project();
    let resp = call_tool(
        &mut server,
        "specforge.collect",
        json!({"path": root.to_str().unwrap()}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed["status"], "collected");
    assert_eq!(parsed["runners"][0]["name"], "cargo-test");
    assert_eq!(parsed["runners"][0]["ran"], false);
    assert_eq!(parsed["runners"][0]["passed"], 1);
    assert_eq!(parsed["diagnostics"][0]["code"], "W115");
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("specforge-report.json")).unwrap())
            .unwrap();
    assert_eq!(report["results"]["alpha"]["tests"][0]["status"], "pass");
}

/// Each `(field, type)` of a spec type holds in `value`: `string`,
/// `integer`, `boolean`, `array` or `string[]`.
fn assert_fields(value: &Value, fields: &[(&str, &str)]) {
    for (field, kind) in fields {
        let v = &value[*field];
        let holds = match *kind {
            "string" => v.is_string(),
            "integer" => v.is_u64() || v.is_i64(),
            "boolean" => v.is_boolean(),
            "array" => v.is_array(),
            "string[]" => v.as_array().is_some_and(|a| a.iter().all(Value::is_string)),
            other => panic!("no check for {other}"),
        };
        assert!(holds, "{field} is not {kind}: {value}");
    }
}

#[specforge_test(type = "McpExtensionInfo", verify = "McpExtensionInfo schema is valid")]
fn each_listed_extension_is_an_mcp_extension_info() {
    let (mut server, _root) = server_with_product();

    let resp = call_tool(&mut server, "specforge.extensions", json!({}));
    let listing: Value = serde_json::from_str(&tool_text(&resp)).unwrap();

    let entries = listing["extensions"].as_array().unwrap();
    assert!(!entries.is_empty(), "{listing}");
    for entry in entries {
        assert_fields(
            entry,
            &[
                ("name", "string"),
                ("version", "string"),
                ("source", "string"),
                ("entity_kinds", "string[]"),
                ("entity_count", "integer"),
                ("validation_rules", "integer"),
                ("status", "string"),
            ],
        );
        let status = entry["status"].as_str().unwrap();
        assert!(
            ["loaded", "not_loaded", "not_configured"].contains(&status),
            "{entry}"
        );
    }
    let greet = entries.iter().find(|e| e["name"] == GREET).unwrap();
    assert_eq!(greet["entity_kinds"], json!(["greeting"]), "{greet}");
    assert!(
        greet["source"].as_str().unwrap().starts_with("local:"),
        "{greet}"
    );
}

fn collected(root: &std::path::Path) -> Value {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.collect",
        json!({"path": root.to_str().unwrap()}),
    );
    serde_json::from_str(&tool_text(&resp)).unwrap_or_else(|_| panic!("{resp}"))
}

#[specforge_test(type = "McpCollectResult", verify = "McpCollectResult schema is valid")]
fn collect_result_is_an_mcp_collect_result() {
    let root = collect_project();

    let result = collected(&root);

    assert_fields(
        &result,
        &[
            ("status", "string"),
            ("runners", "array"),
            ("diagnostics", "array"),
            ("report", "string"),
        ],
    );
    assert_eq!(
        result["report"],
        root.join("specforge-report.json").display().to_string()
    );
    assert_eq!(result["diagnostics"][0]["code"], "W115", "{result}");
}

#[specforge_test(type = "McpCollectRunner", verify = "McpCollectRunner schema is valid")]
fn each_collect_runner_is_an_mcp_collect_runner() {
    let root = collect_project();

    let result = collected(&root);

    let runner = &result["runners"][0];
    assert_fields(
        runner,
        &[
            ("name", "string"),
            ("extension", "string"),
            ("ran", "boolean"),
            ("files", "integer"),
            ("entities", "integer"),
            ("passed", "integer"),
            ("failed", "integer"),
            ("skipped", "integer"),
            ("by_convention", "integer"),
        ],
    );
    // Read, not run: no exit code.
    assert!(runner.get("exit_code").is_none(), "{runner}");
}

// B:provide_mcp_collect_tool — verify unit "unapproved command is refused"
#[specforge_test(
    behavior = "provide_mcp_collect_tool",
    verify = "specforge.collect refuses to run an unapproved command"
)]
fn collect_refuses_unapproved_command() {
    let mut server = test_server();
    let root = collect_project();
    // A fresh temp project was never approved, and the server never asks.
    let resp = call_tool(
        &mut server,
        "specforge.collect",
        json!({"path": root.to_str().unwrap(), "run": true}),
    );
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["diagnostic"]["code"], "E059", "{error}");
    assert!(
        root.join("target/specforge/t.json").exists(),
        "nothing ran, so the old report is untouched"
    );
}

// B:provide_mcp_collect_tool — verify unit "no collector is an error"
#[specforge_test(
    behavior = "provide_mcp_collect_tool",
    verify = "a project without a collector returns an E058 error"
)]
fn collect_without_collector_errors() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.collect", json!({}));
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["diagnostic"]["code"], "E058", "{error}");
}

// B:provide_mcp_collect_tool — verify contract
#[specforge_test(
    behavior = "provide_mcp_collect_tool",
    verify = "Provide MCP Collect Tool: MCP collect tool holds — filesystem_available, compiler_api_available, report_emitted, collector_delegated, never_prompts, tool_invoked_emitted"
)]
fn collect_contract() {
    let mut server = test_server();
    let root = collect_project();
    let path = root.to_str().unwrap();
    // Delegated to the enabled extension's collector; the report is written.
    let ok = call_tool(&mut server, "specforge.collect", json!({"path": path}));
    let parsed: Value = serde_json::from_str(&tool_text(&ok)).unwrap();
    assert_eq!(parsed["runners"][0]["extension"], "@specforge/cargo-test");
    assert!(root.join("specforge-report.json").is_file());
    // Never prompts: running an unapproved command is an error.
    let err = call_tool(
        &mut server,
        "specforge.collect",
        json!({"path": path, "run": true}),
    );
    crate::tool_errors::mcp_error(&err);
}

// --- specforge.render ---

#[test]
fn extensions_entry_fields() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.extensions", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["extensions"].is_array());
    assert!(parsed["entity_kinds_in_graph"].is_array() || parsed["extensions"].is_array());
}

#[test]
fn providers_entry_fields() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.providers", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["providers"].is_array());
}

// --- Contract tests ---

// B:provide_mcp_extensions_tool — verify contract
#[specforge_test(
    behavior = "provide_mcp_extensions_tool",
    verify = "Provide MCP Extensions Tool: MCP extensions tool holds — compiler_api_available, extensions_listed, config_reflected, tool_invoked_emitted"
)]
fn extensions_contract() {
    // compiler_api_available: the server compiled the project.
    let (mut server, root) = software_project();
    assert!(!server.state().registries().manifests.is_empty());

    // extensions_listed: name, version, entity kinds and status.
    let resp = call_tool(&mut server, "specforge.extensions", json!({}));
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    let entry = &parsed["extensions"][0];
    assert_eq!(entry["name"], "@specforge/software");
    assert!(entry["version"].is_string(), "{entry}");
    assert!(
        !entry["entity_kinds"].as_array().unwrap().is_empty(),
        "{entry}"
    );
    assert_eq!(entry["status"], "loaded");

    // config_reflected: specforge.json now drops software and adds testing.
    std::fs::write(
        root.join("specforge.json"),
        json!({"name": "t", "version": "0.1.0", "extensions": ["@specforge/testing"]}).to_string(),
    )
    .unwrap();
    assert_eq!(
        listed_extensions(&mut server),
        vec![
            (
                "@specforge/software".to_string(),
                "not_configured".to_string()
            ),
            ("@specforge/testing".to_string(), "not_loaded".to_string()),
        ]
    );
    // After the next compile, only the configured extension is loaded.
    call_tool(&mut server, "specforge.validate", json!({}));
    assert_eq!(
        listed_extensions(&mut server),
        vec![("@specforge/testing".to_string(), "loaded".to_string())]
    );

    // tool_invoked_emitted
    assert!(invoked(&server, "specforge.extensions"));
}

// B:provide_mcp_providers_tool — verify contract
#[specforge_test(
    behavior = "provide_mcp_providers_tool",
    verify = "Provide MCP Providers Tool: MCP providers tool holds — compiler_api_available, providers_listed, tool_invoked_emitted"
)]
fn providers_contract() {
    // compiler_api_available: a compiled project configuring one provider
    // whose scheme no loaded extension serves.
    let (mut server, _root) = server_over(json!({
        "name": "t", "version": "0.1.0", "extensions": ["@specforge/software"],
        "providers": [{"alias": "tracker", "scheme": "jira", "extension": "@acme/jira"}]
    }));
    assert!(!server.state().registries().manifests.is_empty());

    // providers_listed: scheme, alias, extension and status.
    let resp = call_tool(&mut server, "specforge.providers", json!({}));
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(
        parsed["providers"],
        json!([{"scheme": "jira", "alias": "tracker", "extension": "@acme/jira", "status": "extension_not_loaded"}])
    );

    // tool_invoked_emitted
    assert!(invoked(&server, "specforge.providers"));
}

// B:provide_mcp_doctor_tool — verify contract
#[specforge_test(
    behavior = "provide_mcp_doctor_tool",
    verify = "Provide MCP Doctor Tool: MCP doctor tool holds — compiler_api_available, health_checked, resolution_steps_provided, tool_invoked_emitted"
)]
fn doctor_contract() {
    // compiler_api_available: a project with one extension installed.
    let (mut server, root) = server_with_product();

    // health_checked: a healthy install passes ...
    let healthy = doctor(&mut server);
    assert_eq!(healthy["extensions_ok"], true, "{healthy}");
    assert_eq!(healthy["cache_status"], "ok", "{healthy}");
    assert_eq!(healthy["installed_count"], 1, "{healthy}");
    assert_eq!(healthy["conflicts"], json!([]), "{healthy}");

    // ... and a corrupted one fails.
    tamper_with_installed_binary(&root);
    let broken = doctor(&mut server);
    assert_eq!(broken["extensions_ok"], false, "{broken}");
    assert_eq!(broken["cache_status"], "stale", "{broken}");

    // resolution_steps_provided: every finding says how to fix it, the
    // same way every time.
    let findings = broken["findings"].as_array().unwrap();
    assert!(!findings.is_empty(), "{broken}");
    for finding in findings {
        for field in ["check", "status", "code", "remediation"] {
            assert!(finding[field].is_string(), "{field} missing: {finding}");
        }
    }
    let stale = findings
        .iter()
        .find(|f| f["code"] == "stale_hash")
        .unwrap_or_else(|| panic!("no stale_hash finding: {broken}"));
    assert!(
        stale["remediation"]
            .as_str()
            .unwrap()
            .contains("specforge add"),
        "{stale}"
    );
    assert_eq!(doctor(&mut server), broken);

    // tool_invoked_emitted
    assert!(invoked(&server, "specforge.doctor"));
}

// --- specforge.doctor: real checks ---

const GREET: &str = "@sdk/greet";

/// `test_server` with the product blob installed in its project.
fn server_with_product() -> (McpServer, std::path::PathBuf) {
    let mut server = test_server();
    let root = server.state().project_root.clone().unwrap();
    let blob = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/greet-extension/greet.wasm");
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": blob.to_str().unwrap()}),
    );
    assert!(resp["result"].is_object(), "install failed: {resp}");
    (server, root)
}

/// Overwrite the installed binary, as a corrupted cache would.
fn tamper_with_installed_binary(root: &std::path::Path) {
    let dir = root.join(".specforge/extensions").join(GREET);
    let wasm = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "wasm"))
        .unwrap_or_else(|| panic!("no installed wasm in {}", dir.display()));
    std::fs::write(wasm, b"not the installed module").unwrap();
}

fn doctor(server: &mut McpServer) -> Value {
    let resp = call_tool(server, "specforge.doctor", json!({}));
    serde_json::from_str(&tool_text(&resp)).unwrap_or_else(|_| panic!("{resp}"))
}

#[specforge_test(
    behavior = "provide_mcp_doctor_tool",
    verify = "response checks wasm cache integrity"
)]
fn doctor_flags_an_installed_binary_that_no_longer_matches_the_lock() {
    let (mut server, root) = server_with_product();
    let healthy = doctor(&mut server);
    assert_eq!(healthy["cache_status"], "ok", "{healthy}");
    assert_eq!(healthy["extensions_ok"], true, "{healthy}");

    tamper_with_installed_binary(&root);
    let report = doctor(&mut server);

    assert_eq!(report["cache_status"], "stale", "{report}");
    assert_eq!(report["extensions_ok"], false, "{report}");
    let finding = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["code"] == "stale_hash")
        .unwrap_or_else(|| panic!("no stale_hash finding: {report}"));
    assert_eq!(finding["status"], "error");
    assert!(
        finding["check"].as_str().unwrap().contains(GREET),
        "{finding}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_doctor_tool",
    verify = "response provides deterministic resolution steps"
)]
fn doctor_gives_each_issue_the_same_remediation_every_time() {
    let (mut server, root) = server_with_product();
    tamper_with_installed_binary(&root);

    let first = doctor(&mut server);
    let second = doctor(&mut server);

    assert_eq!(first, second, "doctor is deterministic");
    let findings = first["findings"].as_array().unwrap();
    assert!(!findings.is_empty(), "{first}");
    for finding in findings {
        let remediation = finding["remediation"].as_str().unwrap_or_default();
        assert!(!remediation.is_empty(), "no remediation: {finding}");
    }
    let stale = findings.iter().find(|f| f["code"] == "stale_hash").unwrap();
    assert!(
        stale["remediation"]
            .as_str()
            .unwrap()
            .contains("specforge add"),
        "{stale}"
    );
}

/// The finding with `code` in a doctor report.
fn finding<'a>(report: &'a Value, code: &str) -> Option<&'a Value> {
    report["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("no findings: {report}"))
        .iter()
        .find(|f| f["code"] == code)
}

#[specforge_test(
    behavior = "provide_mcp_doctor_tool",
    verify = "specforge.doctor reports an extension that fails to load (E028, E033) as an error"
)]
fn doctor_reports_an_extension_that_fails_to_load() {
    let (mut server, root) = server_with_product();

    // A binary that no longer matches its lock entry is refused (E033).
    tamper_with_installed_binary(&root);
    let tampered = doctor(&mut server);
    let e033 = finding(&tampered, "E033").unwrap_or_else(|| panic!("no E033: {tampered}"));
    assert_eq!(e033["status"], "error", "{e033}");
    assert!(e033["check"].as_str().unwrap().contains(GREET), "{e033}");
    assert_eq!(tampered["extensions_ok"], false, "{tampered}");

    // Enabled but not installed at all: no lock entry, nothing for the
    // binary check to see. Only the load says so (E028).
    std::fs::remove_file(root.join("specforge.lock")).unwrap();
    std::fs::remove_dir_all(root.join(".specforge")).unwrap();
    let missing = doctor(&mut server);
    let e028 = finding(&missing, "E028").unwrap_or_else(|| panic!("no E028: {missing}"));
    assert_eq!(e028["status"], "error", "{e028}");
    assert!(
        e028["remediation"]
            .as_str()
            .unwrap()
            .contains(&format!("specforge add {GREET}")),
        "{e028}"
    );
    assert_eq!(missing["extensions_ok"], false, "{missing}");
}

#[specforge_test(
    behavior = "provide_mcp_doctor_tool",
    verify = "specforge.doctor compiles the project afresh unless use_cached is set"
)]
fn doctor_compiles_afresh_unless_use_cached() {
    let (mut server, root) = software_project();
    assert!(finding(&doctor(&mut server), "E028").is_none());

    // The agent enables an extension by editing the config directly.
    std::fs::write(
        root.join("specforge.json"),
        json!({"name": "t", "version": "0.1.0",
               "extensions": ["@specforge/software", "@acme/missing"]})
        .to_string(),
    )
    .unwrap();

    let resp = call_tool(&mut server, "specforge.doctor", json!({"use_cached": true}));
    let cached: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert!(finding(&cached, "E028").is_none(), "{cached}");

    let fresh = doctor(&mut server);
    let e028 = finding(&fresh, "E028").unwrap_or_else(|| panic!("no E028: {fresh}"));
    assert!(
        e028["check"].as_str().unwrap().contains("@acme/missing"),
        "{e028}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_doctor_tool",
    verify = "specforge.doctor detects extension conflicts"
)]
fn doctor_lists_extension_conflicts_from_the_compile() {
    let mut server = test_server();
    // What the compiler reports when two extensions register one kind.
    server
        .state_mut()
        .surface_diagnostics
        .push(specforge_common::Diagnostic {
            code: "E026".into(),
            severity: specforge_common::Severity::Error,
            message: "entity kind 'feature' is already registered by '@specforge/product'".into(),
            span: None,
            suggestion: None,
            data: None,
        });
    server
        .state_mut()
        .surface_diagnostics
        .push(specforge_common::Diagnostic {
            code: "W001".into(),
            severity: specforge_common::Severity::Warning,
            message: "an unrelated warning".into(),
            span: None,
            suggestion: None,
            data: None,
        });

    // Over those diagnostics, as the last compile's, not a fresh compile.
    server.state_mut().loaded_at = Some(std::time::SystemTime::now());
    let resp = call_tool(&mut server, "specforge.doctor", json!({"use_cached": true}));
    let report: Value = serde_json::from_str(&tool_text(&resp)).unwrap();

    let conflicts = report["conflicts"].as_array().unwrap();
    assert_eq!(conflicts.len(), 1, "{report}");
    assert!(
        conflicts[0]
            .as_str()
            .unwrap()
            .contains("already registered"),
        "{report}"
    );
    let finding = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["code"] == "E026")
        .unwrap_or_else(|| panic!("no E026 finding: {report}"));
    assert_eq!(finding["status"], "error");
    assert!(finding["remediation"].is_string(), "{finding}");
}

// --- specforge.render ---

fn render(server: &mut McpServer, args: Value) -> Value {
    let resp = call_tool(server, "specforge.render", args);
    serde_json::from_str(&tool_text(&resp)).unwrap_or_else(|_| panic!("{resp}"))
}

#[specforge_test(
    behavior = "provide_mcp_render_tool",
    verify = "specforge.render writes output files to out_dir"
)]
fn render_writes_the_output_into_out_dir() {
    let mut server = test_server();
    let out = tempfile::TempDir::new().unwrap();
    let out_dir = out.path().join("rendered");

    let parsed = render(
        &mut server,
        json!({"format": "dot", "out_dir": out_dir.to_str().unwrap()}),
    );

    let written = out_dir.join("graph.dot");
    assert_eq!(
        parsed["output_files"],
        json!([written.display().to_string()]),
        "{parsed}"
    );
    let dot = std::fs::read_to_string(&written).unwrap();
    assert!(dot.starts_with("digraph"), "{dot}");
    assert!(dot.contains("alpha"), "{dot}");
}

#[specforge_test(
    behavior = "provide_mcp_render_tool",
    verify = "registered renderer invoked for matching format"
)]
fn render_uses_the_renderer_the_format_names() {
    let mut server = test_server();
    let out = tempfile::TempDir::new().unwrap();

    let parsed = render(
        &mut server,
        json!({"format": "json", "out_dir": out.path().to_str().unwrap()}),
    );
    assert_eq!(parsed["format"], "json");
    let graph: Value =
        serde_json::from_str(&std::fs::read_to_string(out.path().join("graph.json")).unwrap())
            .unwrap();
    assert_eq!(graph["nodes"][0]["id"], "alpha", "{graph}");

    // Without out_dir the rendering comes back inline and nothing is written.
    let inline = render(&mut server, json!({"format": "dot"}));
    assert!(inline["output"].as_str().unwrap().starts_with("digraph"));
    assert_eq!(inline["output_files"], json!([]));
}

#[test]
fn render_scope_limits_the_graph_to_one_entity() {
    let mut server = test_server();
    let scoped = render(&mut server, json!({"format": "brief", "scope": "alpha"}));
    assert!(
        scoped["output"].as_str().unwrap().contains("alpha"),
        "{scoped}"
    );

    let missing = call_tool(
        &mut server,
        "specforge.render",
        json!({"format": "brief", "scope": "no_such_entity"}),
    );
    crate::tool_errors::mcp_error(&missing);
}

#[specforge_test(
    behavior = "provide_mcp_render_tool",
    verify = "unrecognized format returns error listing available renderers"
)]
fn render_unknown_format_lists_the_available_renderers() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.render", json!({"format": "yaml"}));

    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "invalid_input", "{error}");
    let message = error["message"].as_str().unwrap();
    assert!(
        message.contains("Unrecognized renderer format: yaml"),
        "{message}"
    );
    assert_eq!(
        error["data"]["available_renderers"],
        json!(["json", "dot", "context", "brief"]),
        "{resp}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_render_tool",
    verify = "Provide MCP Render Tool: MCP render tool holds — graph_available, filesystem_available, files_written, files_listed, tool_invoked_emitted"
)]
fn render_contract() {
    let mut server = test_server();
    let out = tempfile::TempDir::new().unwrap();

    let parsed = render(
        &mut server,
        json!({"format": "context", "out_dir": out.path().to_str().unwrap()}),
    );

    let files = parsed["output_files"].as_array().unwrap();
    assert_eq!(files.len(), 1, "{parsed}");
    let written = std::path::Path::new(files[0].as_str().unwrap());
    assert!(written.starts_with(out.path()), "{parsed}");
    assert!(std::fs::read_to_string(written).unwrap().contains("alpha"));
    assert!(
        server
            .state()
            .events
            .iter()
            .any(|e| e.name == "mcp_tool_invoked"
                && e.params["toolName"] == "specforge.render"
                && e.params["category"] == "management")
    );
}
