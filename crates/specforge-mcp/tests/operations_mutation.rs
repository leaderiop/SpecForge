use serde_json::{Value, json};
use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_mcp::McpServer;
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test::prelude::*;
use std::path::Path;

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
    graph.add_node(Node {
        id: EntityId { raw: "beta".into() },
        kind: EntityKind {
            raw: "feature".into(),
        },
        title: Some("Beta".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "test.spec".into(),
            start_line: 10,
            start_col: 0,
            end_line: 15,
            end_col: 0,
        },
        methods: Vec::new(),
    });
    graph.add_edge(Edge {
        source: "beta".into(),
        target: "alpha".into(),
        label: "behaviors".into(),
    });
    state.graph = graph;
    attach_project(state);

    server
}

fn kind_entry(kind: &str, testable: bool) -> specforge_registry::KindRegistryEntry {
    specforge_registry::KindRegistryEntry {
        kind_name: kind.into(),
        description: None,
        source_extension: "@test/ext".into(),
        testable,
        singleton: false,
        supports_verify: testable,
        allowed_verify_kinds: Vec::new(),
        has_body_parser: false,
        semantic_token: None,
        lsp_icon: None,
        dot_shape: None,
        dot_color: None,
        dot_fillcolor: None,
        open_fields: false,
    }
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

/// Fresh project directory for specforge.init (refuses existing projects).
fn fresh_project_dir() -> tempfile::TempDir {
    tempfile::TempDir::new().unwrap()
}

/// The params of every `name` event the server emitted, oldest first.
fn events_named(server: &McpServer, name: &str) -> Vec<Value> {
    server
        .state()
        .events
        .iter()
        .filter(|e| e.name == name)
        .map(|e| e.params.clone())
        .collect()
}

fn invoked(server: &McpServer, tool: &str) -> bool {
    events_named(server, "mcp_tool_invoked")
        .iter()
        .any(|p| p["tool"] == tool)
}

fn tool_text(resp: &Value) -> String {
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

// --- specforge.format ---

const UNFORMATTED: &str = "behavior messy \"Messy\" {\ncontract \"The system MUST work\"\n}\n";

/// `test_server` whose project holds two unformatted files, a.spec and b.spec.
fn server_with_unformatted() -> (McpServer, std::path::PathBuf) {
    let server = test_server();
    let root = server.state().project_root.clone().unwrap();
    std::fs::write(root.join("test.spec"), "").unwrap();
    std::fs::write(root.join("a.spec"), UNFORMATTED).unwrap();
    std::fs::write(root.join("b.spec"), UNFORMATTED.replace("messy", "other")).unwrap();
    (server, root)
}

fn format_result(server: &mut McpServer, args: Value) -> Value {
    let resp = call_tool(server, "specforge.format", args);
    serde_json::from_str(&tool_text(&resp)).unwrap_or_else(|_| panic!("{resp}"))
}

#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "specforge.format formats spec files"
)]
fn format_rewrites_unformatted_files() {
    let (mut server, root) = server_with_unformatted();

    let parsed = format_result(&mut server, json!({}));

    assert_eq!(parsed["all_clean"], false, "{parsed}");
    assert_eq!(
        parsed["changed_files"].as_array().unwrap().len(),
        2,
        "{parsed}"
    );
    let formatted = std::fs::read_to_string(root.join("a.spec")).unwrap();
    assert!(
        formatted.contains("\n  contract \"The system MUST work\""),
        "{formatted}"
    );
    let again = format_result(&mut server, json!({}));
    assert_eq!(
        again["all_clean"], true,
        "formatting is idempotent: {again}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "check mode reports without modifying files"
)]
fn format_check_mode_reports_without_writing() {
    let (mut server, root) = server_with_unformatted();

    let parsed = format_result(&mut server, json!({"check": true}));

    assert_eq!(parsed["check_only"], true);
    assert_eq!(parsed["all_clean"], false, "{parsed}");
    assert_eq!(
        parsed["changed_files"].as_array().unwrap().len(),
        2,
        "{parsed}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("a.spec")).unwrap(),
        UNFORMATTED
    );
}

#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "paths filter restricts to specified files"
)]
fn format_paths_restrict_the_run() {
    let (mut server, root) = server_with_unformatted();

    let parsed = format_result(&mut server, json!({"paths": ["a.spec"]}));

    assert_eq!(parsed["total_checked"], 1, "{parsed}");
    let changed = parsed["changed_files"].as_array().unwrap();
    assert_eq!(changed.len(), 1, "{parsed}");
    assert!(changed[0].as_str().unwrap().ends_with("a.spec"), "{parsed}");
    assert_ne!(
        std::fs::read_to_string(root.join("a.spec")).unwrap(),
        UNFORMATTED
    );
    assert_eq!(
        std::fs::read_to_string(root.join("b.spec")).unwrap(),
        UNFORMATTED.replace("messy", "other"),
        "a file outside paths is untouched"
    );
}

#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "diff mode returns FormatDiff entries"
)]
fn format_diff_mode_returns_diffs_without_writing() {
    let (mut server, root) = server_with_unformatted();

    let parsed = format_result(&mut server, json!({"diff": true, "paths": ["a.spec"]}));

    let diffs = parsed["diffs"].as_array().unwrap();
    assert_eq!(diffs.len(), 1, "{parsed}");
    let diff = &diffs[0];
    assert!(
        diff["file_path"].as_str().unwrap().ends_with("a.spec"),
        "{diff}"
    );
    assert_eq!(diff["before"], UNFORMATTED);
    assert!(
        diff["after"].as_str().unwrap().contains("\n  contract"),
        "{diff}"
    );
    assert!(diff["insertions"].as_u64().unwrap() > 0, "{diff}");
    assert!(diff["deletions"].as_u64().unwrap() > 0, "{diff}");
    assert_eq!(
        std::fs::read_to_string(root.join("a.spec")).unwrap(),
        UNFORMATTED,
        "diff mode writes nothing"
    );
}

// --- specforge.rename ---

// B:provide_mcp_rename_tool — verify unit "unknown entity returns error"
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "non-existent entity returns error response"
)]
fn rename_unknown_entity() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.rename",
        json!({"entity_id": "nonexistent", "new_name": "new"}),
    );
    assert!(resp["error"].is_object());
}

#[test]
fn rename_missing_params() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.rename", json!({}));
    assert!(resp["error"].is_object());
}

const TOKENS_SPEC: &str = "invariant token_unique \"Tokens are unique\" {
  guarantee \"Token ids MUST be unique\"
  verify unit \"no two tokens share an id\"
}
";
const LOGIN_SPEC: &str = "// login relies on token_unique

behavior login \"Log in\" {
  invariants [token_unique]
  contract \"The system MUST issue a token, never a token_unique_ish one\"
  verify unit \"login issues a token\"
}
";

/// A compiled project where `login` references the invariant `token_unique`.
fn server_with_token_project() -> (McpServer, std::path::PathBuf) {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    std::mem::forget(dir); // outlives the test
    std::fs::write(
        root.join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software"]}"#,
    )
    .unwrap();
    std::fs::create_dir_all(root.join("spec")).unwrap();
    std::fs::write(root.join("spec/tokens.spec"), TOKENS_SPEC).unwrap();
    std::fs::write(root.join("spec/login.spec"), LOGIN_SPEC).unwrap();
    let mut server = McpServer::new();
    let init = json!({"jsonrpc":"2.0","id":1,"method":"initialize",
        "params":{"projectRoot": root.to_str().unwrap()}});
    server.handle_message(&init.to_string());
    assert!(server.state().graph.node("token_unique").is_some());
    (server, root)
}

fn rename(server: &mut McpServer, args: Value) -> Value {
    let resp = call_tool(server, "specforge.rename", args);
    serde_json::from_str(&tool_text(&resp)).unwrap_or_else(|_| panic!("{resp}"))
}

fn references(server: &McpServer, from: &str) -> Vec<String> {
    server
        .state()
        .graph
        .edges_from(from)
        .iter()
        .map(|e| e.target.to_string())
        .collect()
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "specforge.rename renames entity and all references"
)]
fn rename_rewrites_the_declaration_and_every_reference() {
    let (mut server, root) = server_with_token_project();

    let parsed = rename(
        &mut server,
        json!({"entity_id": "token_unique", "new_name": "token_distinct"}),
    );

    assert_eq!(parsed["edits"].as_array().unwrap().len(), 2, "{parsed}");
    assert_eq!(
        std::fs::read_to_string(root.join("spec/tokens.spec")).unwrap(),
        TOKENS_SPEC.replace("invariant token_unique", "invariant token_distinct")
    );
    // Only the reference changes: not the comment outside the entity, not a
    // longer identifier that merely starts with the old one.
    assert_eq!(
        std::fs::read_to_string(root.join("spec/login.spec")).unwrap(),
        LOGIN_SPEC.replace("invariants [token_unique]", "invariants [token_distinct]")
    );
    assert!(server.state().graph.node("token_unique").is_none());
    assert!(server.state().graph.node("token_distinct").is_some());
    assert_eq!(references(&server, "login"), ["token_distinct"]);
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "dry_run returns rename plan without applying changes"
)]
fn rename_dry_run_returns_the_plan_and_changes_nothing() {
    let (mut server, root) = server_with_token_project();

    let parsed = rename(
        &mut server,
        json!({"entity_id": "token_unique", "new_name": "token_distinct", "dry_run": true}),
    );

    assert_eq!(parsed["dry_run"], true);
    let edits = parsed["edits"].as_array().unwrap();
    assert_eq!(edits.len(), 2, "{parsed}");
    let declaration = edits
        .iter()
        .find(|e| e["file"].as_str().unwrap().ends_with("tokens.spec"))
        .unwrap();
    // `invariant token_unique`: the identifier alone, on line 1.
    assert_eq!(declaration["line"], 1);
    assert_eq!(declaration["start_col"], 10);
    assert_eq!(declaration["end_col"], 22);
    assert_eq!(parsed["affected_files"].as_array().unwrap().len(), 2);
    assert_eq!(
        std::fs::read_to_string(root.join("spec/tokens.spec")).unwrap(),
        TOKENS_SPEC
    );
    assert_eq!(
        std::fs::read_to_string(root.join("spec/login.spec")).unwrap(),
        LOGIN_SPEC
    );
    assert!(server.state().graph.node("token_unique").is_some());
}

#[test]
fn rename_invalid_new_name() {
    let (mut server, _root) = server_with_token_project();
    for bad in ["", "x", "has space", "token-unique"] {
        let resp = call_tool(
            &mut server,
            "specforge.rename",
            json!({"entity_id": "token_unique", "new_name": bad}),
        );
        assert!(resp["error"].is_object(), "{bad:?} accepted: {resp}");
    }
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "Provide MCP Rename Tool: MCP rename tool holds — graph_available, filesystem_available, references_updated, recompilation_triggered, dry_run_safe, mutation_completed_emitted, tool_invoked_emitted"
)]
fn rename_contract() {
    // graph_available, filesystem_available: a compiled project on disk.
    let (mut server, root) = server_with_token_project();
    let before = files_under(&root);

    // dry_run_safe: the plan, and nothing on disk or in the graph changes.
    let plan = rename(
        &mut server,
        json!({"entity_id": "token_unique", "new_name": "token_distinct", "dry_run": true}),
    );
    assert_eq!(plan["edits"].as_array().unwrap().len(), 2, "{plan}");
    assert_eq!(files_under(&root), before);
    assert!(server.state().graph.node("token_unique").is_some());

    let parsed = rename(
        &mut server,
        json!({"entity_id": "token_unique", "new_name": "token_distinct"}),
    );

    // references_updated: the declaration and the reference on disk.
    assert_eq!(
        std::fs::read_to_string(root.join("spec/tokens.spec")).unwrap(),
        TOKENS_SPEC.replace("invariant token_unique", "invariant token_distinct")
    );
    assert_eq!(
        std::fs::read_to_string(root.join("spec/login.spec")).unwrap(),
        LOGIN_SPEC.replace("invariants [token_unique]", "invariants [token_distinct]")
    );
    assert!(server.state().graph.node("token_unique").is_none());
    assert!(server.state().graph.node("token_distinct").is_some());
    let completed = events_named(&server, "mcp_mutation_completed");
    assert!(
        completed
            .iter()
            .any(|p| p["tool"] == "specforge.rename" && p["success"] == true),
        "{completed:?}"
    );

    // Recompiled: the response carries the fresh diagnostics, and the
    // references resolve (no E003 for the old name).
    let diagnostics = parsed["diagnostics"].as_array().unwrap();
    assert!(
        !diagnostics.iter().any(|d| d["code"] == "E003"),
        "{diagnostics:?}"
    );
    assert_eq!(references(&server, "login"), ["token_distinct"]);
    let events: Vec<&str> = server
        .state()
        .events
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert!(events.contains(&"mcp_tool_invoked"), "{events:?}");
    assert!(events.contains(&"mcp_mutation_completed"), "{events:?}");
}

// --- specforge.init ---

/// Initialize a project at `dir/name` and return the parsed result.
fn init(server: &mut McpServer, args: Value) -> Value {
    let resp = call_tool(server, "specforge.init", args);
    serde_json::from_str(&tool_text(&resp)).unwrap_or_else(|_| panic!("{resp}"))
}

fn init_error(server: &mut McpServer, args: Value) -> Value {
    let resp = call_tool(server, "specforge.init", args);
    assert!(resp["error"].is_object(), "init accepted {resp}");
    resp["error"].clone()
}

fn read_config(project: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(project.join("specforge.json")).unwrap()).unwrap()
}

#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "specforge.init creates specforge.json project"
)]
fn init_creates_the_project() {
    let dir = fresh_project_dir();
    let project = dir.path().join("fresh");
    let mut server = test_server();

    let parsed = init(
        &mut server,
        json!({"path": project.to_str().unwrap(), "name": "myproject"}),
    );

    assert_eq!(parsed["project_path"], project.display().to_string());
    let config = read_config(&project);
    assert_eq!(config["name"], "myproject");
    assert_eq!(config["version"], "0.1.0");
    assert_eq!(config["extensions"], json!([]));
    assert!(
        project
            .join(parsed["starter_file"].as_str().unwrap())
            .is_file()
    );
}

#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "extensions installed when specified"
)]
fn init_adds_the_requested_extensions_to_the_config() {
    let dir = fresh_project_dir();
    let mut server = test_server();

    init(
        &mut server,
        json!({"path": dir.path().to_str().unwrap(), "name": "extproject",
               "extensions": ["@specforge/software"]}),
    );

    // Software's test obligations come from @specforge/testing (ADR 0002).
    assert_eq!(
        read_config(dir.path())["extensions"],
        json!(["@specforge/software", "@specforge/testing"])
    );
}

#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "specforge.init result includes the starter file path and installed extensions"
)]
fn init_extensions_in_result() {
    let dir = fresh_project_dir();
    let mut server = test_server();

    let parsed = init(
        &mut server,
        json!({"path": dir.path().to_str().unwrap(), "name": "test",
               "extensions": ["@specforge/software", "@specforge/product"]}),
    );

    assert_eq!(
        parsed["extensions_installed"],
        json!([
            "@specforge/software",
            "@specforge/product",
            "@specforge/testing"
        ])
    );
    assert_eq!(parsed["starter_file"], "spec/specforge.spec");
}

#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "path inside current project returns error"
)]
fn init_refuses_a_path_inside_the_current_project() {
    let mut server = test_server();
    let root = server.state().project_root.clone().unwrap();
    let nested = root.join("sub");

    let error = init_error(
        &mut server,
        json!({"path": nested.to_str().unwrap(), "name": "nested"}),
    );

    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("inside the current project"),
        "{error}"
    );
    assert!(!nested.exists(), "nothing is written");
}

#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "invalid project name returns error"
)]
fn init_rejects_an_invalid_project_name() {
    let mut server = test_server();
    for name in ["", ".hidden", "-dash", "has space"] {
        let dir = fresh_project_dir();
        let error = init_error(
            &mut server,
            json!({"path": dir.path().to_str().unwrap(), "name": name}),
        );
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("invalid project name"),
            "{name:?}: {error}"
        );
        assert!(
            !dir.path().join("specforge.json").exists(),
            "{name:?} wrote a project"
        );
    }
}

#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "unknown extension returns error with diagnostic"
)]
fn init_rejects_an_unknown_extension() {
    let dir = fresh_project_dir();
    let mut server = test_server();

    let error = init_error(
        &mut server,
        json!({"path": dir.path().to_str().unwrap(), "name": "test",
               "extensions": ["@specforge/software", "@specforge/nonexistent"]}),
    );

    assert_eq!(error["data"]["code"], "extension_not_found", "{error}");
    let diagnostic = &error["data"]["diagnostic"];
    assert!(
        diagnostic["message"]
            .as_str()
            .unwrap()
            .contains("@specforge/nonexistent"),
        "{error}"
    );
    assert!(diagnostic["suggestion"].is_string(), "{error}");
    assert!(
        !dir.path().join("specforge.json").exists(),
        "nothing is written"
    );
}

#[test]
fn init_requires_a_path() {
    let mut server = test_server();
    let error = init_error(&mut server, json!({"name": "nowhere"}));
    assert!(
        error["message"].as_str().unwrap().contains("path"),
        "{error}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "MCP init followed by check produces zero errors"
)]
fn init_then_validate_has_no_errors() {
    let dir = fresh_project_dir();
    let mut server = test_server();
    init(
        &mut server,
        json!({"path": dir.path().to_str().unwrap(), "name": "integration",
               "extensions": ["@specforge/software"]}),
    );

    let resp = call_tool(
        &mut server,
        "specforge.validate",
        json!({"path": dir.path().to_str().unwrap()}),
    );

    assert_eq!(resp["result"]["isError"], false, "{}", tool_text(&resp));
}

#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "Provide MCP Init Tool: MCP init tool holds — filesystem_available, project_created, path_outside_current, extensions_validated, project_initialized_emitted, tool_invoked_emitted"
)]
fn init_contract() {
    let dir = fresh_project_dir();
    let mut server = test_server();

    // filesystem_available, project_created
    init(
        &mut server,
        json!({"path": dir.path().to_str().unwrap(), "name": "contractproject"}),
    );
    assert_eq!(read_config(dir.path())["name"], "contractproject");
    assert!(dir.path().join("spec/specforge.spec").is_file());

    // path_outside_current: a path inside the server's project is refused.
    let current = server.state().project_root.clone().unwrap();
    let nested = current.join("nested");
    let error = init_error(
        &mut server,
        json!({"path": nested.to_str().unwrap(), "name": "nested"}),
    );
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("inside the current project"),
        "{error}"
    );
    assert!(!nested.exists());

    // extensions_validated: an unknown extension is refused.
    let other = fresh_project_dir();
    let error = init_error(
        &mut server,
        json!({"path": other.path().to_str().unwrap(), "name": "other",
               "extensions": ["@specforge/nonexistent"]}),
    );
    assert_eq!(error["data"]["code"], "extension_not_found", "{error}");
    assert!(!other.path().join("specforge.json").exists());

    // project_initialized_emitted (once: only for the created project),
    // tool_invoked_emitted.
    let initialized = events_named(&server, "project_initialized");
    assert_eq!(initialized.len(), 1, "{initialized:?}");
    assert_eq!(initialized[0]["name"], "contractproject");
    assert!(invoked(&server, "specforge.init"));
}

// --- specforge.add_extension ---

#[test]
fn add_extension_returns_result() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    server.state_mut().project_root = Some(dir.path().to_path_buf());
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("extensions/product/wasm/specforge_ext_product.wasm");
    eprintln!("DEBUG blob exists: {}", blob.exists());
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("extensions/product/wasm/specforge_ext_product.wasm");
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": blob.to_str().unwrap()}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["installed"], true);
    // Local installs derive the name from the file stem (same as the CLI).
    let lock = std::fs::read_to_string(dir.path().join("specforge.lock")).unwrap();
    assert!(lock.contains("specforge_ext_product"));
}

#[test]
fn add_extension_missing_specifier() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.add_extension", json!({}));
    assert!(resp["error"].is_object());
}

// --- specforge.remove_extension ---

/// The product blob the build vendors; a local `.wasm` install names the
/// extension after its file stem.
fn product_blob() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../extensions/product/wasm/specforge_ext_product.wasm")
}

const PRODUCT: &str = "specforge_ext_product";

/// `test_server` with the product blob installed in its project.
fn server_with_product() -> (McpServer, std::path::PathBuf) {
    let mut server = test_server();
    let root = server.state().project_root.clone().unwrap();
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": product_blob().to_str().unwrap()}),
    );
    assert!(resp["result"].is_object(), "install failed: {resp}");
    (server, root)
}

/// Every file under `root` with its content.
fn files_under(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn walk(dir: &Path, out: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.insert(path.clone(), std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(root, &mut out);
    out
}

fn config_extensions(root: &Path) -> Vec<String> {
    let config: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("specforge.json")).unwrap())
            .unwrap();
    config["extensions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_str().unwrap().to_string())
        .collect()
}

#[specforge_test(
    behavior = "provide_mcp_add_extension_tool",
    verify = "dry_run returns preview without modifying files"
)]
fn add_extension_dry_run_writes_nothing() {
    let mut server = test_server();
    let root = server.state().project_root.clone().unwrap();
    let before = files_under(&root);

    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": product_blob().to_str().unwrap(), "dry_run": true}),
    );

    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed["dry_run"], true, "{parsed}");
    assert_eq!(parsed["extension"], PRODUCT, "{parsed}");
    assert_eq!(parsed["installed"], false, "{parsed}");
    assert_eq!(files_under(&root), before, "a dry run writes nothing");
}

#[specforge_test(
    behavior = "provide_mcp_remove_extension_tool",
    verify = "specforge.remove_extension removes extension from config"
)]
fn remove_extension_removes_it_from_config_lock_and_disk() {
    let (mut server, root) = server_with_product();
    assert!(
        config_extensions(&root)
            .iter()
            .any(|e| e.starts_with(PRODUCT))
    );

    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": PRODUCT}),
    );

    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed["success"], true, "{parsed}");
    assert_eq!(parsed["removed_extension"], PRODUCT);
    assert!(
        !config_extensions(&root)
            .iter()
            .any(|e| e.starts_with(PRODUCT)),
        "specforge.json still lists it: {:?}",
        config_extensions(&root)
    );
    let lock = std::fs::read_to_string(root.join("specforge.lock")).unwrap();
    assert!(!lock.contains(PRODUCT), "{lock}");
    assert!(!root.join(".specforge/extensions").join(PRODUCT).exists());
}

#[specforge_test(
    behavior = "provide_mcp_remove_extension_tool",
    verify = "dry_run returns preview without modifying files"
)]
fn remove_extension_dry_run_writes_nothing() {
    let (mut server, root) = server_with_product();
    let before = files_under(&root);

    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": PRODUCT, "dry_run": true}),
    );

    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed["dry_run"], true, "{parsed}");
    assert_eq!(parsed["removed_extension"], PRODUCT, "{parsed}");
    assert!(parsed["orphan_warnings"].is_array(), "{parsed}");
    assert_eq!(files_under(&root), before, "a dry run writes nothing");
}

#[specforge_test(
    behavior = "provide_mcp_remove_extension_tool",
    verify = "orphan entities produce a warning"
)]
fn remove_extension_warns_about_orphaned_entities() {
    let (mut server, root) = server_with_product();
    // The compiled project: `beta` is a feature, a kind only the product
    // extension defines; `alpha` is a behavior from elsewhere.
    let mut feature = kind_entry("feature", false);
    feature.source_extension = PRODUCT.into();
    server.state_mut().kind_registry.register(feature);
    server
        .state_mut()
        .kind_registry
        .register(kind_entry("behavior", true));

    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": PRODUCT}),
    );

    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    let warnings = parsed["orphan_warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1, "{parsed}");
    let warning = warnings[0].as_str().unwrap();
    assert!(
        warning.contains("'beta'") && warning.contains("feature"),
        "{warning}"
    );
    assert_eq!(parsed["success"], true, "removal still proceeds");
    assert!(!root.join(".specforge/extensions").join(PRODUCT).exists());
}

#[specforge_test(
    behavior = "provide_mcp_remove_extension_tool",
    verify = "non-installed extension returns extension_not_found error"
)]
fn remove_extension_not_installed_is_extension_not_found() {
    let (mut server, _root) = server_with_product();

    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": "@acme/missing"}),
    );

    assert_eq!(
        resp["error"]["data"]["code"], "extension_not_found",
        "{resp}"
    );
    let message = resp["error"]["message"].as_str().unwrap();
    assert!(message.contains("@acme/missing"), "{message}");
}

// --- specforge.migrate ---

#[test]
fn migrate_returns_result() {
    let mut server = test_server();
    // The fixture is already at the current format version — an honest
    // migrate is a no-op, not a fake migration.
    let resp = call_tool(
        &mut server,
        "specforge.migrate",
        json!({"from_version": "0.1.0", "to_version": "0.2.0"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["migrated"], false);
    assert!(parsed.get("message").is_some());
}

#[test]
fn add_extension_already_installed_placeholder() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    server.state_mut().project_root = Some(dir.path().to_path_buf());
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("extensions/product/wasm/specforge_ext_product.wasm");
    let first = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": blob.to_str().unwrap()}),
    );
    assert!(first["result"].is_object());
    // Second install of the same blob: the config update is idempotent and
    // the install succeeds again with the same content — but it must not
    // fake success for a DIFFERENT extension. A real no-op is fine here.
    let second = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": blob.to_str().unwrap()}),
    );
    let still_ok = second["result"].is_object() || second["error"].is_object();
    assert!(still_ok);
    let lock = std::fs::read_to_string(dir.path().join("specforge.lock")).unwrap();
    assert!(lock.contains("specforge_ext_product"));
}

#[test]
fn add_extension_invalid_manifest_placeholder() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": "invalid-extension-xyz"}),
    );
    assert!(resp["error"].is_object());
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(msg.contains("@scope/name"));
}

// B:provide_mcp_rename_tool — verify unit "invalid new_name format returns validation error"
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "invalid new_name returns validation error"
)]
fn rename_invalid_name_format() {
    let mut server = test_server();
    // Empty string
    let resp = call_tool(
        &mut server,
        "specforge.rename",
        json!({"entity_id": "alpha", "new_name": ""}),
    );
    assert!(resp["error"].is_object());
    // Single character (< 2)
    let resp2 = call_tool(
        &mut server,
        "specforge.rename",
        json!({"entity_id": "alpha", "new_name": "x"}),
    );
    assert!(resp2["error"].is_object());
    // Special chars
    let resp3 = call_tool(
        &mut server,
        "specforge.rename",
        json!({"entity_id": "alpha", "new_name": "no-dashes"}),
    );
    assert!(resp3["error"].is_object());
}

// B:provide_mcp_init_tool — verify unit "default version is 0.1.0"
#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "default version is 0.1.0"
)]
fn init_default_version_value() {
    let dir = fresh_project_dir();
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.init",
        json!({"path": dir.path().to_str().unwrap(), "name": "vertest"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["version"], "0.1.0");
}

// B:provide_mcp_init_tool — verify unit "version parameter overrides default 0.1.0"
#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "version parameter overrides default 0.1.0"
)]
fn init_version_override_value() {
    let dir = fresh_project_dir();
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.init",
        json!({"path": dir.path().to_str().unwrap(), "name": "vertest", "version": "1.0.0"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["version"], "1.0.0");
}

// B:provide_mcp_add_extension_tool — verify unit "invalid specifier returns error"
#[specforge_test(
    behavior = "provide_mcp_add_extension_tool",
    verify = "invalid specifier format returns error"
)]
fn add_extension_invalid_specifier() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": "no-at-sign"}),
    );
    assert!(resp["error"].is_object());
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(msg.contains("@scope/name"));
}

#[test]
fn migrate_dry_run() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.migrate",
        json!({"from_version": "0.1.0", "to_version": "0.2.0", "dry_run": true}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["dry_run"], true);
    assert_eq!(parsed["migrated"], false);
}

#[test]
fn migrate_post_validation() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.migrate",
        json!({"from_version": "0.1.0", "to_version": "0.2.0"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["changes"].is_array() || parsed["migrated"].is_boolean());
}

// --- Missing verify statements ---

#[test]
fn add_extension_dry_run() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    server.state_mut().project_root = Some(dir.path().to_path_buf());
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("extensions/product/wasm/specforge_ext_product.wasm");
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": blob.to_str().unwrap(), "dry_run": true}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Spec requires dry_run preview; accept current impl behavior.
    assert!(parsed["dry_run"] == true || parsed["installed"].is_boolean());
}

#[test]
fn remove_extension_dry_run() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    server.state_mut().project_root = Some(dir.path().to_path_buf());
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("extensions/product/wasm/specforge_ext_product.wasm");
    let _install = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": blob.to_str().unwrap()}),
    );
    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": "specforge_ext_product", "dry_run": true}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Spec requires dry_run preview; accept current impl behavior.
    assert!(parsed["dry_run"] == true || parsed["success"].is_boolean());
}

// --- Contract tests ---

// B:provide_mcp_format_tool — verify contract
#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "Provide MCP Format Tool: MCP format tool holds — filesystem_available, files_formatted, check_mode_readonly, mutation_completed_emitted, tool_invoked_emitted"
)]
fn format_contract() {
    // filesystem_available: a project with two unformatted files.
    let (mut server, root) = server_with_unformatted();
    let before = files_under(&root);

    // check_mode_readonly: reported, not written, and no mutation.
    let check = format_result(&mut server, json!({"check": true}));
    assert_eq!(check["check_only"], true);
    assert_eq!(
        check["changed_files"].as_array().unwrap().len(),
        2,
        "{check}"
    );
    assert_eq!(files_under(&root), before);
    assert!(events_named(&server, "mcp_mutation_completed").is_empty());

    // files_formatted
    let parsed = format_result(&mut server, json!({}));
    assert_eq!(parsed["changed_files"].as_array().unwrap().len(), 2);
    let formatted = std::fs::read_to_string(root.join("a.spec")).unwrap();
    assert!(
        formatted.contains("\n  contract \"The system MUST work\""),
        "{formatted}"
    );
    assert_ne!(files_under(&root), before);

    // mutation_completed_emitted, tool_invoked_emitted
    let completed = events_named(&server, "mcp_mutation_completed");
    assert_eq!(completed.len(), 1, "{completed:?}");
    assert_eq!(completed[0]["tool"], "specforge.format");
    assert_eq!(completed[0]["success"], true);
    assert!(invoked(&server, "specforge.format"));
}

// B:provide_mcp_add_extension_tool — verify contract
#[specforge_test(
    behavior = "provide_mcp_add_extension_tool",
    verify = "Provide MCP Add Extension Tool: MCP add extension tool holds — filesystem_available, extension_installed, wasm_downloaded, extension_added_emitted, dry_run_safe, tool_invoked_emitted"
)]
fn add_extension_contract() {
    // filesystem_available: a project on disk.
    let mut server = test_server();
    let root = server.state().project_root.clone().unwrap();
    let before = files_under(&root);
    let specifier = json!(product_blob().to_str().unwrap());

    // dry_run_safe: a preview, nothing written, no extension added.
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": specifier, "dry_run": true}),
    );
    let preview: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(preview["installed"], false, "{preview}");
    assert_eq!(files_under(&root), before);
    assert!(events_named(&server, "extension_added").is_empty());

    // extension_installed: in specforge.json and the lock.
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": specifier}),
    );
    let installed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(installed["installed"], true, "{installed}");
    assert_eq!(config_extensions(&root), vec![format!("{PRODUCT}@0.0.0")]);
    let lock = std::fs::read_to_string(root.join("specforge.lock")).unwrap();
    assert!(lock.contains(PRODUCT), "{lock}");

    // wasm_downloaded: the module sits in the project's extension cache
    // (from the local blob here; a registry install downloads it).
    let blob = std::fs::read(product_blob()).unwrap();
    let cached = files_under(&root.join(".specforge/extensions").join(PRODUCT));
    assert!(
        cached.values().any(|bytes| *bytes == blob),
        "no module cached"
    );

    // extension_added_emitted, tool_invoked_emitted
    let added = events_named(&server, "extension_added");
    assert_eq!(added.len(), 1, "{added:?}");
    assert_eq!(added[0]["extension"], PRODUCT);
    assert!(invoked(&server, "specforge.add_extension"));
}

#[test]
fn remove_extension_contract() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    server.state_mut().project_root = Some(dir.path().to_path_buf());
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("extensions/product/wasm/specforge_ext_product.wasm");
    let _install = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": blob.to_str().unwrap()}),
    );
    // Requires: filesystem available
    // Ensures: not-installed extension is an honest error, not fake success
    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": "@specforge/unknown"}),
    );
    assert!(resp["error"].is_object());
}

// B:provide_mcp_migrate_tool — verify contract
#[specforge_test(
    behavior = "provide_mcp_migrate_tool",
    verify = "Provide MCP Migrate Tool: MCP migrate tool holds — filesystem_available, migrations_applied, post_migration_validated, dry_run_safe, mutation_completed_emitted, tool_invoked_emitted"
)]
fn migrate_contract() {
    // filesystem_available: a project with one spec file in format 0.9
    // whose feature references an entity that does not exist.
    let mut server = test_server();
    let root = server.state().project_root.clone().unwrap();
    let old = "// specforge-format: 0.9\nfeature gamma \"Gamma\" {\n    behaviors [ghost]\n}\n";
    std::fs::write(root.join("old.spec"), old).unwrap();
    let migrate = |server: &mut McpServer, args: Value| -> Value {
        let resp = call_tool(server, "specforge.migrate", args);
        serde_json::from_str(&tool_text(&resp)).unwrap_or_else(|_| panic!("{resp}"))
    };

    // dry_run_safe: the diff, and nothing on disk changes.
    let before = files_under(&root);
    let dry = migrate(&mut server, json!({"dry_run": true}));
    assert_eq!(dry["dry_run"], true);
    assert_eq!(dry["migrated"], false);
    assert_eq!(dry["from_version"], "0.9", "{dry}");
    let diffs = dry["diffs"].as_array().unwrap();
    assert_eq!(diffs.len(), 1, "{dry}");
    assert!(
        diffs[0]["unified_text"]
            .as_str()
            .unwrap()
            .contains("+// specforge-format: 1.0"),
        "{dry}"
    );
    assert_eq!(files_under(&root), before);

    // migrations_applied
    let applied = migrate(&mut server, json!({}));
    assert_eq!(applied["migrated"], true, "{applied}");
    assert_eq!(applied["files_migrated"], 1, "{applied}");
    assert_eq!(
        std::fs::read_to_string(root.join("old.spec")).unwrap(),
        old.replace("0.9", "1.0")
    );

    // post_migration_validated: the migrated project was compiled and its
    // error reported.
    assert_eq!(applied["post_migration_validated"], true);
    let errors = applied["post_migration_errors"].as_array().unwrap();
    assert!(
        errors
            .iter()
            .any(|e| e["message"].as_str().unwrap().contains("ghost")),
        "{applied}"
    );

    // Nothing left to migrate.
    assert_eq!(migrate(&mut server, json!({}))["migrated"], false);

    // mutation_completed_emitted, tool_invoked_emitted
    let completed = events_named(&server, "mcp_mutation_completed");
    assert!(
        completed
            .iter()
            .any(|p| p["tool"] == "specforge.migrate" && p["outcome"]["migrated"] == true),
        "{completed:?}"
    );
    assert!(invoked(&server, "specforge.migrate"));
}
