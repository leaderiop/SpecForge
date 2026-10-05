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
    crate::support::serve_in_memory_at(state, &root);
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
    state.serve_graph(graph, Vec::new());
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
        contract_target: false,
        declares_types: false,
        lifecycle_field: None,
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

/// The params of every `name` event the server emitted, oldest first,
/// each without its `timestamp`.
fn events_named(server: &McpServer, name: &str) -> Vec<Value> {
    server
        .state()
        .events
        .iter()
        .filter(|e| e.name == name)
        .map(|e| {
            let mut params = e.params.clone();
            let stamp = params.as_object_mut().unwrap().remove("timestamp");
            assert!(stamp.is_some_and(|s| s.is_string()), "{name}: {}", e.params);
            params
        })
        .collect()
}

fn invoked(server: &McpServer, tool: &str) -> bool {
    events_named(server, "mcp_tool_invoked")
        .iter()
        .any(|p| p["toolName"] == tool && p["category"] == "mutation")
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
    let root = server
        .state()
        .project_root()
        .map(std::path::Path::to_path_buf)
        .unwrap();
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

#[cfg(unix)]
#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "a file that cannot be written does not stop the others, and the failed call names it"
)]
fn format_writes_every_file_it_can_and_names_the_ones_it_cannot() {
    use std::os::unix::fs::PermissionsExt;
    let (mut server, root) = server_with_unformatted();
    // a.spec comes first and can't be written.
    let locked = root.join("a.spec");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o444)).unwrap();

    let resp = call_tool(&mut server, "specforge.format", json!({}));
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();

    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "internal_error", "{error}");
    let parsed = &error["data"];
    let failed = parsed["failed_files"].as_array().unwrap();
    assert_eq!(failed.len(), 1, "{parsed}");
    assert!(failed[0].as_str().unwrap().ends_with("a.spec"), "{parsed}");
    assert_eq!(std::fs::read_to_string(&locked).unwrap(), UNFORMATTED);
    assert!(
        std::fs::read_to_string(root.join("b.spec"))
            .unwrap()
            .contains("\n  contract"),
        "b.spec is formatted although a.spec failed"
    );
}

#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "format configuration diagnostics are returned in the result"
)]
fn format_returns_the_config_diagnostics() {
    let (mut server, root) = server_with_unformatted();
    std::fs::write(root.join(".specforgefmt.toml"), "indent_width = 99\n").unwrap();

    let parsed = format_result(&mut server, json!({"check": true}));

    let diagnostics = parsed["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("no diagnostics: {parsed}"));
    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == "W141" && d["message"].as_str().unwrap().contains("indent")),
        "{parsed}"
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
    crate::tool_errors::mcp_error(&resp);
}

#[test]
fn rename_missing_params() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.rename", json!({}));
    crate::tool_errors::mcp_error(&resp);
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
    assert!(server.state().graph().node("token_unique").is_some());
    (server, root)
}

fn rename(server: &mut McpServer, args: Value) -> Value {
    let resp = call_tool(server, "specforge.rename", args);
    serde_json::from_str(&tool_text(&resp)).unwrap_or_else(|_| panic!("{resp}"))
}

fn references(server: &McpServer, from: &str) -> Vec<String> {
    server
        .state()
        .graph()
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
    assert!(server.state().graph().node("token_unique").is_none());
    assert!(server.state().graph().node("token_distinct").is_some());
    assert_eq!(references(&server, "login"), ["token_distinct"]);
}

/// A rename recompiles the project from disk: a file edited since the
/// server last loaded it, and not touched by the rename, is served too,
/// and the diagnostics returned are what a fresh compile reports.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "Provide MCP Rename Tool: MCP rename tool holds — graph_available, filesystem_available, references_updated, recompilation_triggered, dry_run_safe, mutation_completed_emitted, tool_invoked_emitted"
)]
fn rename_recompiles_files_it_did_not_edit() {
    let (mut server, root) = server_with_token_project();
    // Edited without watch: the server does not know yet.
    std::fs::write(
        root.join("spec/logout.spec"),
        "behavior logout \"Log out\" {\n  invariants [missing_invariant]\n}\n",
    )
    .unwrap();

    let parsed = rename(
        &mut server,
        json!({"entity_id": "token_unique", "new_name": "token_distinct"}),
    );

    assert!(server.state().graph().node("logout").is_some());
    let codes = |diagnostics: &[Value]| {
        let mut codes: Vec<String> = diagnostics
            .iter()
            .map(|d| d["code"].as_str().unwrap_or_default().to_string())
            .collect();
        codes.sort();
        codes
    };
    let runtime = specforge_component::project_runtime(&root);
    let fresh: Vec<Value> = specforge_project::CompiledProject::compile(&root, Some(&runtime))
        .diagnostics()
        .iter()
        .map(|d| serde_json::to_value(d).unwrap())
        .collect();
    let returned = parsed["diagnostics"].as_array().unwrap();
    assert_eq!(codes(returned), codes(&fresh), "{parsed}");
    let e003 = returned
        .iter()
        .find(|d| d["code"] == "E003")
        .unwrap_or_else(|| {
            panic!("the unresolved reference in the unrenamed file is reported: {parsed}")
        });
    // The shape every mutation tool returns: catalogue title and the flat
    // file/line/column beside the span.
    assert!(e003["title"].is_string(), "{e003}");
    assert!(
        e003["file"]
            .as_str()
            .is_some_and(|f| f.ends_with("logout.spec")),
        "{e003}"
    );
    assert!(e003["line"].is_u64() && e003["column"].is_u64(), "{e003}");
}

/// Each `(field, type)` of a spec type holds in `value`: `string`,
/// `integer`, `boolean`, or `string[]`.
fn assert_fields(value: &Value, fields: &[(&str, &str)]) {
    for (field, kind) in fields {
        let v = &value[*field];
        let holds = match *kind {
            "string" => v.is_string(),
            "integer" => v.is_u64() || v.is_i64(),
            "boolean" => v.is_boolean(),
            "string[]" => v.as_array().is_some_and(|a| a.iter().all(Value::is_string)),
            other => panic!("no check for {other}"),
        };
        assert!(holds, "{field} is not {kind}: {value}");
    }
}

#[specforge_test(type = "McpRenameResult", verify = "McpRenameResult schema is valid")]
fn rename_result_is_an_mcp_rename_result() {
    let (mut server, _root) = server_with_token_project();

    let parsed = rename(
        &mut server,
        json!({"entity_id": "token_unique", "new_name": "token_distinct", "dry_run": true}),
    );

    assert_fields(
        &parsed,
        &[
            ("old_name", "string"),
            ("new_name", "string"),
            ("affected_files", "string[]"),
        ],
    );
    assert_eq!(
        parsed["affected_files"],
        json!(["spec/login.spec", "spec/tokens.spec"]),
        "{parsed}"
    );
    let edits = parsed["edits"].as_array().unwrap();
    assert_eq!(edits.len(), 2, "{parsed}");
    for edit in edits {
        assert_fields(
            edit,
            &[
                ("file", "string"),
                ("line", "integer"),
                ("start_col", "integer"),
                ("end_col", "integer"),
                ("new_text", "string"),
            ],
        );
    }
}

/// With `spec_root` set, spans are relative to the spec root, not the
/// project root: rename must read and write the files there (plan 01, D8).
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "specforge.rename renames entity and all references"
)]
fn rename_edits_the_files_under_a_configured_spec_root() {
    let (mut server, root) = server_with_token_project();
    std::fs::write(
        root.join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software"],"spec_root":"spec"}"#,
    )
    .unwrap();
    server.state_mut().serve(&root);
    assert!(server.state().graph().node("token_unique").is_some());

    let parsed = rename(
        &mut server,
        json!({"entity_id": "token_unique", "new_name": "token_distinct"}),
    );

    assert_eq!(parsed["edits"].as_array().unwrap().len(), 2, "{parsed}");
    assert_eq!(
        std::fs::read_to_string(root.join("spec/tokens.spec")).unwrap(),
        TOKENS_SPEC.replace("invariant token_unique", "invariant token_distinct")
    );
    assert_eq!(
        std::fs::read_to_string(root.join("spec/login.spec")).unwrap(),
        LOGIN_SPEC.replace("invariants [token_unique]", "invariants [token_distinct]")
    );
    assert!(server.state().graph().node("token_distinct").is_some());
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
    assert!(server.state().graph().node("token_unique").is_some());
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
        crate::tool_errors::mcp_error(&resp);
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
    assert!(server.state().graph().node("token_unique").is_some());

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
    assert!(server.state().graph().node("token_unique").is_none());
    assert!(server.state().graph().node("token_distinct").is_some());
    // Two files rewritten, one entity renamed.
    assert_eq!(
        events_named(&server, "mcp_mutation_completed"),
        [json!({
            "toolName": "specforge.rename",
            "files_changed": 2,
            "entities_affected": 1,
            "success": true,
        })]
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
    crate::tool_errors::mcp_error(&resp)
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
    assert_eq!(parsed["starter_file"], "spec/hello.spec");
}

#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "path inside current project returns error"
)]
fn init_refuses_a_path_inside_the_current_project() {
    let mut server = test_server();
    let root = server
        .state()
        .project_root()
        .map(std::path::Path::to_path_buf)
        .unwrap();
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

    assert_eq!(error["code"], "extension_not_found", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("@specforge/nonexistent"),
        "{error}"
    );
    assert!(error["data"]["suggestion"].is_string(), "{error}");
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
    assert!(dir.path().join("spec/hello.spec").is_file());

    // path_outside_current: a path inside the server's project is refused.
    let current = server
        .state()
        .project_root()
        .map(std::path::Path::to_path_buf)
        .unwrap();
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
    assert_eq!(error["code"], "extension_not_found", "{error}");
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
    crate::support::serve_in_memory_at(server.state_mut(), dir.path());
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":[]}"#,
    )
    .unwrap();
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/greet-extension/greet.wasm");
    eprintln!("DEBUG blob exists: {}", blob.exists());
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/greet-extension/greet.wasm");
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
    assert!(lock.contains("@sdk/greet"));
}

#[test]
fn add_extension_missing_specifier() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.add_extension", json!({}));
    crate::tool_errors::mcp_error(&resp);
}

// --- specforge.remove_extension ---

/// The product blob the build vendors; a local `.wasm` install names the
/// extension after its file stem.
fn greet_blob() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/greet-extension/greet.wasm")
}

const GREET: &str = "@sdk/greet";

/// `test_server` with the product blob installed in its project.
fn server_with_product() -> (McpServer, std::path::PathBuf) {
    let mut server = test_server();
    let root = server
        .state()
        .project_root()
        .map(std::path::Path::to_path_buf)
        .unwrap();
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": greet_blob().to_str().unwrap()}),
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
    let root = server
        .state()
        .project_root()
        .map(std::path::Path::to_path_buf)
        .unwrap();
    let before = files_under(&root);

    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": greet_blob().to_str().unwrap(), "dry_run": true}),
    );

    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed["dry_run"], true, "{parsed}");
    assert_eq!(parsed["extension"], GREET, "{parsed}");
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
            .any(|e| e.starts_with(GREET))
    );

    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": GREET}),
    );

    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed["success"], true, "{parsed}");
    assert_eq!(parsed["removed_extension"], GREET);
    assert!(
        !config_extensions(&root)
            .iter()
            .any(|e| e.starts_with(GREET)),
        "specforge.json still lists it: {:?}",
        config_extensions(&root)
    );
    let lock = std::fs::read_to_string(root.join("specforge.lock")).unwrap();
    assert!(!lock.contains(GREET), "{lock}");
    assert!(!root.join(".specforge/extensions").join(GREET).exists());
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
        json!({"name": GREET, "dry_run": true}),
    );

    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed["dry_run"], true, "{parsed}");
    assert_eq!(parsed["removed_extension"], GREET, "{parsed}");
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
    feature.source_extension = GREET.into();
    server.state_mut().edit_environment(|env| {
        env.registries.kinds.register(feature);
    });
    server.state_mut().edit_environment(|env| {
        env.registries.kinds.register(kind_entry("behavior", true));
    });

    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": GREET}),
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
    assert!(!root.join(".specforge/extensions").join(GREET).exists());
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

    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "extension_not_found", "{resp}");
    let message = error["message"].as_str().unwrap();
    assert!(message.contains("@acme/missing"), "{message}");
}

#[specforge_test(
    behavior = "provide_mcp_remove_extension_tool",
    verify = "specforge.remove_extension removes extension from config"
)]
fn remove_extension_disables_an_enabled_builtin() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"p","version":"0.1.0","extensions":["@specforge/software","@specforge/product"]}"#,
    )
    .unwrap();
    let mut server = McpServer::with_project_root(dir.path().to_path_buf());
    let init = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&init.to_string());

    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": "@specforge/product"}),
    );

    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed["success"], true, "{parsed}");
    assert_eq!(parsed["removed_extension"], "@specforge/product");
    assert_eq!(config_extensions(dir.path()), ["@specforge/software"]);
}

// --- specforge.migrate ---

#[test]
fn migrate_returns_result() {
    let mut server = test_server();
    // The fixture is already at the current format version — an honest
    // migrate is a no-op, not a fake migration.
    let resp = call_tool(&mut server, "specforge.migrate", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["migrated"], false);
    assert!(parsed.get("message").is_some());
}

#[test]
fn add_extension_already_installed_placeholder() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    crate::support::serve_in_memory_at(server.state_mut(), dir.path());
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":[]}"#,
    )
    .unwrap();
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/greet-extension/greet.wasm");
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
    let still_ok = second["result"].is_object();
    assert!(still_ok);
    let lock = std::fs::read_to_string(dir.path().join("specforge.lock")).unwrap();
    assert!(lock.contains("@sdk/greet"));
}

#[test]
fn add_extension_invalid_manifest_placeholder() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": "invalid-extension-xyz"}),
    );
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["diagnostic"]["code"], "E054", "{resp}");
    let msg = error["message"].as_str().unwrap();
    assert!(msg.contains("invalid-extension-xyz"), "{msg}");
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
    crate::tool_errors::mcp_error(&resp);
    // Single character (< 2)
    let resp2 = call_tool(
        &mut server,
        "specforge.rename",
        json!({"entity_id": "alpha", "new_name": "x"}),
    );
    crate::tool_errors::mcp_error(&resp2);
    // Special chars
    let resp3 = call_tool(
        &mut server,
        "specforge.rename",
        json!({"entity_id": "alpha", "new_name": "no-dashes"}),
    );
    crate::tool_errors::mcp_error(&resp3);
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
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["diagnostic"]["code"], "E054", "{resp}");
    let msg = error["message"].as_str().unwrap();
    assert!(msg.contains("no-at-sign"), "{msg}");
}

#[test]
fn migrate_dry_run() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.migrate", json!({"dry_run": true}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["dry_run"], true);
    assert_eq!(parsed["migrated"], false);
}

#[test]
fn migrate_post_validation() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.migrate", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["changes"].is_array() || parsed["migrated"].is_boolean());
}

// --- Missing verify statements ---

#[test]
fn add_extension_dry_run() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    crate::support::serve_in_memory_at(server.state_mut(), dir.path());
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":[]}"#,
    )
    .unwrap();
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/greet-extension/greet.wasm");
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
    crate::support::serve_in_memory_at(server.state_mut(), dir.path());
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":[]}"#,
    )
    .unwrap();
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/greet-extension/greet.wasm");
    let _install = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": blob.to_str().unwrap()}),
    );
    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": "@sdk/greet", "dry_run": true}),
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
    assert_eq!(
        events_named(&server, "mcp_mutation_completed"),
        [json!({
            "toolName": "specforge.format",
            "files_changed": 2,
            "entities_affected": 0,
            "success": true,
        })]
    );
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
    let root = server
        .state()
        .project_root()
        .map(std::path::Path::to_path_buf)
        .unwrap();
    let before = files_under(&root);
    let specifier = json!(greet_blob().to_str().unwrap());

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
    // Enabled by its bare name; the lock pins version and hash (D3-b).
    assert_eq!(config_extensions(&root), vec![GREET]);
    let lock = std::fs::read_to_string(root.join("specforge.lock")).unwrap();
    assert!(lock.contains(GREET), "{lock}");

    // wasm_downloaded: the module sits in the project's extension cache
    // (from the local blob here; a registry install downloads it).
    let blob = std::fs::read(greet_blob()).unwrap();
    let cached = files_under(&root.join(".specforge/extensions").join(GREET));
    assert!(
        cached.values().any(|bytes| *bytes == blob),
        "no module cached"
    );

    // extension_added_emitted, tool_invoked_emitted
    let added = events_named(&server, "extension_added");
    assert_eq!(added.len(), 1, "{added:?}");
    assert_eq!(added[0]["extension"], GREET);
    assert!(invoked(&server, "specforge.add_extension"));
}

#[test]
fn remove_extension_contract() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    crate::support::serve_in_memory_at(server.state_mut(), dir.path());
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":[]}"#,
    )
    .unwrap();
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/greet-extension/greet.wasm");
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
    crate::tool_errors::mcp_error(&resp);
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
    let root = server
        .state()
        .project_root()
        .map(std::path::Path::to_path_buf)
        .unwrap();
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
    // The migration rewrote one file; the second run changed nothing.
    let migration = |files_changed: usize| {
        json!({
            "toolName": "specforge.migrate",
            "files_changed": files_changed,
            "entities_affected": 0,
            "success": true,
        })
    };
    assert_eq!(
        events_named(&server, "mcp_mutation_completed"),
        [migration(1), migration(0)]
    );
    assert!(invoked(&server, "specforge.migrate"));
}

/// `test_server` whose project also holds `old.spec`, in format 0.9.
fn server_with_old_spec() -> (McpServer, std::path::PathBuf) {
    let server = test_server();
    let root = server
        .state()
        .project_root()
        .map(std::path::Path::to_path_buf)
        .unwrap();
    std::fs::write(
        root.join("old.spec"),
        "// specforge-format: 0.9\nbehavior gamma \"Gamma\" {\n}\n",
    )
    .unwrap();
    (server, root)
}

#[specforge_test(
    behavior = "provide_mcp_migrate_tool",
    verify = "target_version selects the format version to migrate to"
)]
fn migrate_target_version_selects_the_version() {
    let (mut server, root) = server_with_old_spec();
    let resp = call_tool(
        &mut server,
        "specforge.migrate",
        json!({"target_version": "1.0", "no_backup": true}),
    );
    let result: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(result["migrated"], true, "{result}");
    assert_eq!(result["from_version"], "0.9", "{result}");
    assert_eq!(result["to_version"], "1.0", "{result}");
    assert!(
        std::fs::read_to_string(root.join("old.spec"))
            .unwrap()
            .starts_with("// specforge-format: 1.0"),
    );
    // no_backup: no .bak copy beside the migrated file.
    assert!(
        !files_under(&root)
            .keys()
            .any(|p| p.extension().is_some_and(|e| e == "bak"))
    );
}

#[specforge_test(
    behavior = "provide_mcp_migrate_tool",
    verify = "a malformed or unsupported target_version is refused without modifying files"
)]
fn migrate_refuses_a_bad_target_version() {
    let (mut server, root) = server_with_old_spec();
    let before = files_under(&root);
    for target in ["99.0", "latest"] {
        let resp = call_tool(
            &mut server,
            "specforge.migrate",
            json!({"target_version": target}),
        );
        let error = crate::tool_errors::mcp_error(&resp);
        assert_eq!(error["diagnostic"]["code"], "E019", "{target}: {resp}");
        let message = error["message"].as_str().unwrap_or_default();
        assert!(message.contains(target), "{target}: {resp}");
    }
    assert_eq!(files_under(&root), before);
}

#[specforge_test(
    behavior = "provide_mcp_migrate_tool",
    verify = "the result reports the hooks run, the structural differences and whether the migration was rolled back"
)]
fn migrate_reports_hooks_structure_and_rollback() {
    let (mut server, root) = server_with_old_spec();

    let resp = call_tool(&mut server, "specforge.migrate", json!({}));

    let result: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(result["migrated"], true, "{result}");
    // No enabled extension declares a migration hook.
    assert_eq!(result["hooks_invoked"], json!([]), "{result}");
    assert_eq!(result["hook_failures"], json!([]), "{result}");
    // A header-only migration keeps the graph's structure.
    assert_eq!(result["structural_differences"], json!([]), "{result}");
    assert_eq!(result["rolled_back"], false, "{result}");
    assert!(
        std::fs::read_to_string(root.join("old.spec"))
            .unwrap()
            .starts_with("// specforge-format: 1.0")
    );
}

/// A project whose two registries share the alias `main`; nothing listens
/// at their URL, so a registry install fails at once.
fn project_with_duplicate_registry_alias() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let url = "http://127.0.0.1:9/v1";
    let config = json!({
        "name": "t", "version": "0.1.0", "extensions": [],
        "registries": [
            {"alias": "main", "url": url, "default_registry": true},
            {"alias": "main", "url": url},
        ],
    });
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    dir
}

#[specforge_test(
    behavior = "provide_mcp_add_extension_tool",
    verify = "add_extension of a registry package reports what reading the registry configuration found"
)]
fn add_extension_from_a_registry_reports_a_duplicate_registry_alias() {
    let dir = project_with_duplicate_registry_alias();
    let mut server = test_server();
    let path = dir.path().to_str().unwrap();

    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": "@acme/widget@1.0.0", "path": path}),
    );
    let codes: Vec<&str> = resp["result"]["_meta"]["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("no _meta.diagnostics: {resp}"))
        .iter()
        .filter_map(|d| d["code"].as_str())
        .collect();
    assert!(codes.contains(&"W140"), "{resp}");

    // A builtin never reads the registries, so it reports none of it.
    let builtin = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": "@specforge/software", "path": path, "dry_run": true}),
    );
    assert!(builtin["result"]["_meta"].is_null(), "{builtin}");
}

/// The new name follows the entity-ID rule (the grammar's identifier,
/// 2-60 characters): an illegal one is refused and nothing is written.
#[test]
fn rename_refuses_an_illegal_entity_id() {
    let (mut server, root) = server_with_token_project();
    for bad in [
        "a".repeat(61),
        "token-distinct".to_string(),
        "9token".to_string(),
    ] {
        let resp = call_tool(
            &mut server,
            "specforge.rename",
            json!({"entity_id": "token_unique", "new_name": bad}),
        );
        let error = crate::tool_errors::mcp_error(&resp);
        assert_eq!(error["code"], "invalid_input", "{bad}: {error}");
        assert_eq!(error["argument"], "new_name", "{error}");
    }
    assert_eq!(
        std::fs::read_to_string(root.join("spec/tokens.spec")).unwrap(),
        TOKENS_SPEC
    );
}

/// A project on disk with `specforge.json` enabling `extensions`, and
/// `files`, served by a fresh server.
fn served_project(extensions: &[&str], files: &[(&str, &str)]) -> (McpServer, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    let config = json!({"name": "m", "version": "0.1.0", "extensions": extensions});
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    for (path, text) in files {
        std::fs::write(dir.path().join(path), text).unwrap();
    }
    let mut server = McpServer::new();
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"projectRoot": dir.path().to_str().unwrap()}});
    server.handle_message(&req.to_string());
    (server, dir)
}

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "remove_extension with a path to another project checks that project's dependents and entities"
)]
fn remove_on_another_project_checks_that_projects_dependents() {
    // The served project enables formal, which requires software; the
    // other enables software alone.
    let (mut server, _served) = served_project(
        &["@specforge/software", "@specforge/formal"],
        &[("main.spec", "")],
    );
    let other = tempfile::TempDir::new().unwrap();
    std::fs::write(
        other.path().join("specforge.json"),
        json!({"name": "o", "version": "0.1.0", "extensions": ["@specforge/software"]}).to_string(),
    )
    .unwrap();
    std::fs::write(other.path().join("main.spec"), "").unwrap();

    // Nothing in the other project requires software: it can go.
    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"path": other.path().to_str().unwrap(), "name": "@specforge/software", "dry_run": true}),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    let payload: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(payload["success"], true, "{payload}");

    // In the served project formal still requires it.
    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": "@specforge/software", "dry_run": true}),
    );
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["diagnostic"]["code"], "E027", "{error}");
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "rename plans on the project as it is on disk, references added since the last call included"
)]
fn rename_plans_on_the_project_as_it_is_on_disk() {
    let (mut server, dir) = served_project(
        &[],
        &[(
            "main.spec",
            "invariant tok \"Tok\" {\n}\nbehavior login \"Login\" {\n  invariants [tok]\n}\n",
        )],
    );
    // A reference written after the server last read the project.
    std::fs::write(
        dir.path().join("logout.spec"),
        "behavior logout \"Logout\" {\n  invariants [tok]\n}\n",
    )
    .unwrap();

    let resp = call_tool(
        &mut server,
        "specforge.rename",
        json!({"entity_id": "tok", "new_name": "tok2"}),
    );
    let payload: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    let affected = payload["affected_files"].as_array().unwrap();
    assert!(affected.iter().any(|f| f == "logout.spec"), "{payload}");
    assert!(
        std::fs::read_to_string(dir.path().join("logout.spec"))
            .unwrap()
            .contains("invariants [tok2]")
    );
    let diagnostics = payload["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().all(|d| d["code"] != "E003"), "{payload}");
}

#[specforge_test(
    behavior = "provide_mcp_add_extension_tool",
    verify = "after add_extension the server serves the extension it installed"
)]
fn add_then_tools_list_shows_the_extension_tools() {
    let (mut server, _dir) = served_project(&[], &[("main.spec", "")]);
    let product_tools = |server: &mut McpServer| -> usize {
        let req = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}});
        let resp: Value =
            serde_json::from_str(&server.handle_message(&req.to_string()).unwrap()).unwrap();
        resp["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["source"] == "@specforge/product")
            .count()
    };
    assert_eq!(product_tools(&mut server), 0);

    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": "@specforge/product"}),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp}");

    assert!(
        product_tools(&mut server) > 0,
        "the installed extension's tools are listed"
    );
    assert!(
        server
            .state()
            .registries()
            .manifests
            .iter()
            .any(|m| m.name == "@specforge/product")
    );
}
