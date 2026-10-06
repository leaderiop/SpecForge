//! Which project an MCP call acts on, and whether it is current with disk
//! (architecture plan 01): a call's optional `path` and its tool's target
//! decide it before the handler runs.

use serde_json::{Value, json};
use specforge_mcp::McpServer;
use specforge_test::prelude::*;
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
    server.state().project_root().map(canonical)
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

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "a path while no project is served serves that project, for every tool that takes a path"
)]
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

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "a mutation on another project does not reload the served one"
)]
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

#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "validate with use_cached=true returns existing diagnostics without recompilation"
)]
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

#[specforge_test(
    behavior = "provide_mcp_doctor_tool",
    verify = "specforge.doctor compiles the project afresh unless use_cached is set"
)]
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
    use crate::fake_extension::{self, FakeExtension};

    // Serving nothing, no extension tool is listed: a call names an
    // unknown tool.
    let mut server = serving_nothing();
    assert!(server.state().surfaces().tools().is_empty());
    let resp = call_tool(&mut server, "specforge.cmds.check", json!({}));
    assert_eq!(resp["error"]["code"], -32602, "{resp}");

    // A project's extension tools stay listed when the server goes on to
    // serve a graph built in memory with no root: a call has no project to
    // run in.
    let (mut server, ext, _dir) = fake_extension::initialized(FakeExtension::new());
    server
        .state_mut()
        .serve_in_memory_at(None, specforge_graph::Graph::new(), Vec::new());
    let resp = call_tool(&mut server, "specforge.cmds.check", json!({}));
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "precondition_failed", "{error}");
    assert!(ext.calls().is_empty(), "{:?}", ext.calls());
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

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "a tool call serves files written since the last call, without watch"
)]
fn a_tool_call_serves_files_written_since_the_last_call() {
    let dir = project(&[], "behavior login \"Login\" {\n}\n");
    let mut server = McpServer::new();
    initialize(&mut server, dir.path());
    assert!(finds(&mut server, "login"));

    // Written after initialize, with no watch running.
    fs::write(
        dir.path().join("added.spec"),
        "feature fresh \"Fresh\" {\n  behaviors [login]\n}\n",
    )
    .unwrap();
    assert!(
        finds(&mut server, "fresh"),
        "the call serves what is on disk"
    );

    // A file deleted since: its entities go.
    fs::remove_file(dir.path().join("added.spec")).unwrap();
    assert!(!finds(&mut server, "fresh"), "a deleted file's entities go");
}

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "a prompt reads the project as it is on disk"
)]
fn a_prompt_reads_the_project_as_it_is_on_disk() {
    let dir = project(&[], "behavior login \"Login\" {\n}\n");
    let mut server = McpServer::new();
    initialize(&mut server, dir.path());

    fs::write(
        dir.path().join("added.spec"),
        "behavior fresh \"Fresh\" {\n}\n",
    )
    .unwrap();
    let resp = call(
        &mut server,
        "prompts/get",
        json!({"name": "specforge://prompts/context", "arguments": {"entity_id": "fresh"}}),
    );
    assert!(resp["error"].is_null(), "{resp}");
    assert!(resp["result"]["messages"].is_array(), "{resp}");
}

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "a resource read serves files written since the last call, without watch"
)]
fn a_resource_read_serves_files_written_since_the_last_call() {
    let dir = project(&[], "behavior login \"Login\" {\n}\n");
    let mut server = McpServer::new();
    initialize(&mut server, dir.path());

    fs::write(
        dir.path().join("added.spec"),
        "behavior fresh \"Fresh\" {\n}\n",
    )
    .unwrap();
    let resp = call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://graph/fresh"}),
    );
    assert!(resp["error"].is_null(), "{resp}");
    let text = resp["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(text.contains("\"fresh\""), "{text}");
}

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "an environment change on disk updates the extension tools listed"
)]
fn an_environment_change_on_disk_updates_the_extension_tools_listed() {
    use crate::fake_extension::{self, EXT, FakeExtension};

    let ext = std::sync::Arc::new(FakeExtension::new());
    let dir = project(&[], "");
    let mut server = fake_extension::server_with(&ext);
    initialize(&mut server, dir.path());
    let before = tool_names(&mut server);
    assert!(
        !before.iter().any(|t| t.starts_with("specforge.cmds.")),
        "{before:?}"
    );

    // The project enables the extension: listing the tools alone sees it.
    write_config(dir.path(), &[EXT]);
    let after = tool_names(&mut server);
    for tool in ["specforge.cmds.check", "specforge.cmds.report"] {
        assert!(
            after.iter().any(|t| t == tool),
            "{tool} is not listed after the change: {after:?}"
        );
    }
    // And the auto-promoted tool dispatches: the reload built its entry,
    // not only its descriptor.
    let entry = server.state().surfaces().tool("specforge.cmds.report");
    assert!(
        matches!(
            entry.map(|e| &e.kind),
            Some(specforge_mcp::surface_table::ToolKind::Command(_))
        ),
        "{entry:?}"
    );

    // Listing again changes nothing: each tool is listed once.
    let again = tool_names(&mut server);
    let reports = again
        .iter()
        .filter(|t| *t == "specforge.cmds.report")
        .count();
    assert_eq!(reports, 1, "{again:?}");
}

#[specforge_test(
    invariant = "mcp_tool_idempotency",
    verify = "read-only tools return equivalent results for identical inputs"
)]
fn an_unchanged_project_is_not_rebuilt_by_a_call() {
    let dir = project(&[], "behavior login \"Login\" {\n}\n");
    let mut server = McpServer::new();
    initialize(&mut server, dir.path());
    let generation = server.state().session_generation();

    let first = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "login"}),
    );
    let second = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "login"}),
    );
    assert_eq!(first, second);
    assert_eq!(
        server.state().session_generation(),
        generation,
        "nothing changed on disk: nothing was rebuilt"
    );
}

/// What each tool that takes a `path` is called with to act on a project
/// with `behavior alpha` (its path added by the caller).
fn path_tool_calls() -> Vec<(&'static str, Value)> {
    vec![
        ("specforge.validate", json!({})),
        ("specforge.analyze", json!({"pass": "contracts"})),
        ("specforge.collect", json!({})),
        ("specforge.format", json!({"check": true})),
        (
            "specforge.rename",
            json!({"entity_id": "alpha", "new_name": "gamma"}),
        ),
        (
            "specforge.add_extension",
            json!({"specifier": "@specforge/software", "dry_run": true}),
        ),
        (
            "specforge.remove_extension",
            json!({"name": "@specforge/software", "dry_run": true}),
        ),
        ("specforge.migrate", json!({"dry_run": true})),
    ]
}

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "a path while no project is served serves that project, for every tool that takes a path"
)]
fn a_path_while_nothing_is_served_serves_it_for_every_tool() {
    // Every core tool whose schema takes a project path is covered, init
    // (which creates its project) below.
    let mut covered: Vec<&str> = path_tool_calls().iter().map(|(name, _)| *name).collect();
    covered.push("specforge.init");
    covered.sort_unstable();
    let mut with_path: Vec<&str> = specforge_mcp::tools::CORE_TOOLS
        .iter()
        .filter(|tool| (tool.schema)()["properties"].get("path").is_some())
        .map(|tool| tool.name)
        .collect();
    with_path.sort_unstable();
    assert_eq!(covered, with_path);

    for (tool, mut arguments) in path_tool_calls() {
        let other = project(&["@specforge/software"], "behavior alpha \"Alpha\" {\n}\n");
        arguments["path"] = Value::from(other.path().to_str().unwrap());
        let mut server = serving_nothing();

        call_tool(&mut server, tool, arguments);

        assert_eq!(
            served_root(&server),
            Some(canonical(other.path())),
            "{tool} did not serve the project its path names"
        );
        if tool == "specforge.rename" {
            // R1b: the rename acted on the project the path names.
            assert!(server.state().graph().node("gamma").is_some(), "{tool}");
            let text = fs::read_to_string(other.path().join("main.spec")).unwrap();
            assert!(text.contains("behavior gamma"), "{text}");
        } else {
            assert!(server.state().graph().node("alpha").is_some(), "{tool}");
        }
    }

    // init with nothing served serves the project it created.
    let parent = TempDir::new().unwrap();
    let created = parent.path().join("created");
    let mut server = serving_nothing();
    let resp = call_tool(
        &mut server,
        "specforge.init",
        json!({"path": created.to_str().unwrap(), "name": "created"}),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    assert_eq!(served_root(&server), Some(canonical(&created)));
}

#[specforge_test(
    invariant = "mcp_structured_error_responses",
    verify = "a path that does not exist is a file_not_found error on path"
)]
fn a_path_that_does_not_exist_is_file_not_found() {
    let served = project(&[], "behavior login \"Login\" {\n}\n");
    let missing = served.path().join("no/such/dir");
    for (tool, mut arguments) in path_tool_calls() {
        let mut server = McpServer::new();
        initialize(&mut server, served.path());
        arguments["path"] = Value::from(missing.to_str().unwrap());

        let resp = call_tool(&mut server, tool, arguments);

        let error = crate::tool_errors::mcp_error(&resp);
        assert_eq!(error["code"], "file_not_found", "{tool}: {error}");
        assert_eq!(error["argument"], "path", "{tool}: {error}");
        assert_eq!(error["tool"], tool, "{error}");
    }
}

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "a mutation on another project does not reload the served one"
)]
fn a_mutation_on_another_project_does_not_reload_the_served_one() {
    let served = project(&[], "behavior login \"Login\" {\n}\n");
    let calls: Vec<(&str, Value)> = vec![
        (
            "specforge.rename",
            json!({"entity_id": "alpha", "new_name": "omega"}),
        ),
        ("specforge.format", json!({})),
        (
            "specforge.add_extension",
            json!({"specifier": "@specforge/software"}),
        ),
        (
            "specforge.remove_extension",
            json!({"name": "@specforge/software"}),
        ),
        ("specforge.migrate", json!({})),
    ];
    for (tool, mut arguments) in calls {
        let other = project(
            &["@specforge/software"],
            "behavior alpha \"Alpha\" {\n\n\n}\n",
        );
        let mut server = McpServer::new();
        initialize(&mut server, served.path());
        // An edit the served project has not seen: reloading it would
        // serve it.
        fs::write(
            served.path().join("added.spec"),
            "behavior unseen \"U\" {\n}\n",
        )
        .unwrap();
        let generation = server.state().session_generation();
        arguments["path"] = Value::from(other.path().to_str().unwrap());

        let resp = call_tool(&mut server, tool, arguments);
        assert!(resp["error"].is_null(), "{tool}: {resp}");

        assert_eq!(
            server.state().session_generation(),
            generation,
            "{tool} reloaded the served project"
        );
        assert!(server.state().graph().node("unseen").is_none(), "{tool}");
        assert_eq!(served_root(&server), Some(canonical(served.path())));
        fs::remove_file(served.path().join("added.spec")).unwrap();
    }

    // R6: init of a new project elsewhere.
    let mut server = McpServer::new();
    initialize(&mut server, served.path());
    fs::write(
        served.path().join("added.spec"),
        "behavior unseen \"U\" {\n}\n",
    )
    .unwrap();
    let generation = server.state().session_generation();
    let elsewhere = TempDir::new().unwrap();
    let dir = elsewhere.path().join("fresh");
    let resp = call_tool(
        &mut server,
        "specforge.init",
        json!({"path": dir.to_str().unwrap(), "name": "fresh"}),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    assert_eq!(server.state().session_generation(), generation);
    assert!(server.state().graph().node("unseen").is_none());
}

/// A tool of the served project only takes no path to another project.
#[test]
fn a_tool_of_the_served_project_refuses_another_projects_path() {
    let served = project(&[], "behavior login \"Login\" {\n}\n");
    let other = project(&[], "behavior other \"Other\" {\n}\n");
    let mut server = McpServer::new();
    initialize(&mut server, served.path());

    let resp = call_tool(
        &mut server,
        "specforge.doctor",
        json!({"path": other.path().to_str().unwrap()}),
    );
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "invalid_input", "{error}");
    assert_eq!(error["argument"], "path", "{error}");

    // Its own project's path is the served project.
    let resp = call_tool(
        &mut server,
        "specforge.doctor",
        json!({"path": served.path().to_str().unwrap()}),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp}");
}

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "a path inside the served project names the served project"
)]
fn a_path_inside_the_served_project_names_the_served_project() {
    let dir = TempDir::new().unwrap();
    let config = json!({"name": "a", "version": "0.1.0", "spec_root": "spec",
        "extensions": ["@specforge/software"]});
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    fs::create_dir_all(dir.path().join("spec")).unwrap();
    fs::write(
        dir.path().join("spec/main.spec"),
        "behavior login \"Login\" {\n  category command\n  invariants [missing]\n}\n",
    )
    .unwrap();
    let mut server = McpServer::new();
    initialize(&mut server, dir.path());
    let generation = server.state().session_generation();

    let inside = validate_codes(
        &mut server,
        json!({"path": dir.path().join("spec").to_str().unwrap()}),
    );
    let served = validate_codes(&mut server, json!({}));
    assert_eq!(inside, served);
    // Compiled as a project of its own, spec/ would load no extension.
    assert!(!inside.contains(&"I002".to_string()), "{inside:?}");
    assert!(inside.contains(&"E003".to_string()), "{inside:?}");
    assert_eq!(server.state().session_generation(), generation);
}

#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "validate with use_cached=true returns existing diagnostics without recompilation"
)]
fn use_cached_is_honoured_when_the_project_has_no_diagnostics() {
    let dir = project(&["@specforge/software"], "");
    let mut server = McpServer::new();
    initialize(&mut server, dir.path());
    assert!(validate_codes(&mut server, json!({"use_cached": true})).is_empty());

    // R5: a project with no diagnostics was recompiled whatever use_cached
    // said.
    fs::write(
        dir.path().join("added.spec"),
        "behavior added \"Added\" {\n  category command\n  invariants [nope]\n}\n",
    )
    .unwrap();
    let cached = validate_codes(&mut server, json!({"use_cached": true}));
    assert!(cached.is_empty(), "{cached:?}");
    assert!(server.state().graph().node("added").is_none());

    let fresh = validate_codes(&mut server, json!({}));
    assert!(fresh.contains(&"E003".to_string()), "{fresh:?}");
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "analyzing another project leaves the served project untouched"
)]
fn analyze_of_another_project_runs_its_extensions_in_one_runtime() {
    use crate::fake_extension::{self, EXT, FakeExtension};

    let ext = FakeExtension::new().with_passes(&["audit"]).with_output(
        "__pass_audit",
        json!({"diagnostics": [{"code": "W900", "severity": "Warning", "message": "careful"}]}),
    );
    let (mut server, ext, _served) = fake_extension::initialized(ext);
    let generation = server.state().session_generation();
    let other = project(&[EXT], "");
    let loads = ext.handshakes();

    let resp = call_tool(
        &mut server,
        "specforge.analyze",
        json!({"path": other.path().to_str().unwrap(), "pass": "@test/cmds:audit"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(
        parsed["passes"][0]["findings"][0]["code"], "W900",
        "{parsed}"
    );

    // The other project's environment loaded once, and its pass ran in the
    // runtime it loaded in; the served project was not touched.
    assert_eq!(ext.handshakes() - loads, 1);
    assert!(
        ext.calls()
            .iter()
            .any(|(_, export, _)| export == "__pass_audit"),
        "{:?}",
        ext.calls()
    );
    assert_eq!(server.state().session_generation(), generation);
}

#[specforge_test(
    behavior = "provide_mcp_outline_tool",
    verify = "a file under the spec root with no entities has an empty outline"
)]
fn outline_finds_an_empty_file_under_the_spec_root() {
    let dir = TempDir::new().unwrap();
    let config = json!({"name": "o", "version": "0.1.0", "spec_root": "spec"});
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    fs::create_dir_all(dir.path().join("spec")).unwrap();
    fs::write(
        dir.path().join("spec/main.spec"),
        "term alpha \"Alpha\" {\n}\n",
    )
    .unwrap();
    fs::write(dir.path().join("spec/empty.spec"), "// nothing yet\n").unwrap();
    let mut server = McpServer::new();
    initialize(&mut server, dir.path());

    let resp = call_tool(
        &mut server,
        "specforge.outline",
        json!({"file": "main.spec"}),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    let resp = call_tool(
        &mut server,
        "specforge.outline",
        json!({"file": "empty.spec"}),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    assert_eq!(tool_text(&resp), "[]");
    // A file that is not there is still not found.
    let resp = call_tool(
        &mut server,
        "specforge.outline",
        json!({"file": "gone.spec"}),
    );
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "file_not_found", "{error}");
}

/// Pin (architecture plan 10, T0): with nothing served, outline looks a
/// file up relative to the server's working directory. Nextest runs this
/// binary in the package root, where `Cargo.toml` exists: outline answers
/// it with an empty outline, though no project is served. A file that is
/// nowhere is `file_not_found`.
#[test]
fn outline_with_nothing_served_reads_the_working_directory() {
    assert!(
        Path::new("Cargo.toml").is_file(),
        "run from the package root"
    );
    let mut server = serving_nothing();

    let resp = call_tool(
        &mut server,
        "specforge.outline",
        json!({"file": "Cargo.toml"}),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    assert_eq!(tool_text(&resp), "[]");

    let resp = call_tool(
        &mut server,
        "specforge.outline",
        json!({"file": "absent.spec"}),
    );
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "file_not_found", "{error}");
    assert_eq!(error["argument"], "file", "{error}");
}

/// Pin (architecture plan 10, T0): with nothing served, inspecting an
/// entity answers that the entity is not found, not that no project is
/// served.
#[test]
fn inspect_with_nothing_served_is_entity_not_found() {
    let mut server = serving_nothing();

    let resp = call_tool(&mut server, "specforge.inspect", json!({"entity_id": "x"}));
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "entity_not_found", "{error}");
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "analyze with no project served and no path is a no-project error"
)]
fn analyze_without_a_project_is_refused() {
    let mut server = serving_nothing();
    let resp = call_tool(&mut server, "specforge.analyze", json!({}));
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "precondition_failed", "{error}");
    assert_eq!(error["tool"], "specforge.analyze", "{error}");
}

#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "rename with a path to another project edits that project only and keeps serving this one"
)]
fn rename_on_another_project_edits_that_project_only() {
    let served = project(&[], "behavior alpha \"A\" {\n}\n");
    let other = project(
        &[],
        "behavior alpha \"B\" {\n}\nbehavior beta \"Beta\" {\n}\n",
    );
    let mut server = McpServer::new();
    initialize(&mut server, served.path());

    let resp = call_tool(
        &mut server,
        "specforge.rename",
        json!({"path": other.path().to_str().unwrap(), "entity_id": "alpha", "new_name": "gamma"}),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    // The returned diagnostics are the other project's, compiled after the
    // edit: no dangling reference.
    let payload: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert!(
        payload["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["code"] != "E003")
    );

    assert!(
        fs::read_to_string(other.path().join("main.spec"))
            .unwrap()
            .contains("behavior gamma")
    );
    assert!(
        fs::read_to_string(served.path().join("main.spec"))
            .unwrap()
            .contains("behavior alpha")
    );
    assert_eq!(served_root(&server), Some(canonical(served.path())));
    assert!(server.state().graph().node("beta").is_none());
    assert!(server.state().graph().node("alpha").is_some());
}
