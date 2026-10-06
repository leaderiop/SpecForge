//! What each mutation tool wrote, what it reports and emits, and how the
//! served project follows (architecture plan 04, ADR 0022).
//!
//! Every pin compares what a call left on disk (a snapshot diff of the
//! project) with what it reported.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use specforge_mcp::McpServer;

use crate::support::*;

/// test.spec: the behavior `alpha` and the feature `beta` that has it.
const TEST_SPEC: &str = concat!(
    "behavior alpha \"Alpha\" {\n",
    "}\n",
    "\n",
    "feature beta \"Beta\" {\n",
    "    behaviors [alpha]\n",
    "}\n",
);

/// A spec file the formatter rewrites.
const UNFORMATTED: &str = "behavior messy \"Messy\" {\ncontract \"The system MUST work\"\n}\n";

/// A spec file in format 0.9: `migrate` rewrites it (and backs it up).
const OLD: &str = "// specforge-format: 0.9\nbehavior gamma \"Gamma\" {\n}\n";

/// The extension blob the build vendors: `@sdk/greet`.
fn greet_blob() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/greet-extension/greet.wasm")
}

fn greet() -> String {
    greet_blob().to_str().expect("a UTF-8 path").to_string()
}

const GREET: &str = "@sdk/greet";
const PRODUCT: &str = "@specforge/product";

/// A project with no source, served with its own component runtime: what
/// an add or a removal changes is what the builtins and installs make of it.
fn empty_components() -> Served {
    TestProject::new().serve_components()
}

/// The project's files, relative to its root, as `files_written` lists them.
fn names(files: &[&str]) -> Vec<PathBuf> {
    files.iter().map(PathBuf::from).collect()
}

/// Call `tool` with `arguments` on `server`; its reply and the files under
/// `root` the call changed.
fn wrote(
    server: &mut McpServer,
    root: &Path,
    tool: &str,
    arguments: Value,
) -> (Value, Vec<PathBuf>) {
    let before = files_under(root);
    let reply = call_tool(server, tool, arguments);
    let changed = changed_files(root, &before, &files_under(root));
    (reply, changed)
}

/// The reply succeeded (no protocol error, no `isError`).
fn assert_ok(reply: &Value) {
    assert_eq!(reply["result"]["isError"], false, "{reply}");
}

/// The last `mcp_mutation_completed` event, if any.
fn last_completed(server: &McpServer) -> Option<Value> {
    events(server, "mcp_mutation_completed").pop()
}

/// The names of the events recorded from index `since` on, among `names`.
fn event_names_since(server: &McpServer, since: usize, names: &[&str]) -> Vec<String> {
    server.state().events[since..]
        .iter()
        .filter(|e| names.contains(&e.name.as_str()))
        .map(|e| e.name.clone())
        .collect()
}

// 1
#[test]
fn enabling_a_builtin_writes_one_file() {
    let mut server = empty_components();
    let root = server.root().to_path_buf();

    let (reply, changed) = wrote(
        &mut server,
        &root,
        "specforge.add_extension",
        json!({"specifier": PRODUCT}),
    );

    assert_ok(&reply);
    assert_eq!(changed, names(&["specforge.json"]));
    // BUG (04-T3 flips): one file written, three reported.
    assert_eq!(
        events(&server, "mcp_mutation_completed"),
        [completed("specforge.add_extension", 3, 0, true)]
    );
}

// 2
#[test]
fn disabling_a_builtin_writes_one_file() {
    let mut server = empty_components();
    let root = server.root().to_path_buf();
    assert_ok(&call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": PRODUCT}),
    ));

    let (reply, changed) = wrote(
        &mut server,
        &root,
        "specforge.remove_extension",
        json!({"name": PRODUCT}),
    );

    assert_ok(&reply);
    assert_eq!(changed, names(&["specforge.json"]));
    // BUG (04-T3 flips): one file written, three reported.
    assert_eq!(
        last_completed(&server),
        Some(completed("specforge.remove_extension", 3, 0, true))
    );
}

// 3
#[test]
fn enabling_an_enabled_builtin_writes_nothing() {
    let mut server = empty_components();
    let root = server.root().to_path_buf();
    assert_ok(&call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": PRODUCT}),
    ));
    assert_eq!(events(&server, "extension_added").len(), 1);

    let (reply, changed) = wrote(
        &mut server,
        &root,
        "specforge.add_extension",
        json!({"specifier": PRODUCT}),
    );

    assert_ok(&reply);
    assert_eq!(changed, Vec::<PathBuf>::new());
    assert_eq!(
        last_completed(&server),
        Some(completed("specforge.add_extension", 0, 0, true))
    );
    // BUG (04-T4 flips; spec: wasDuplicate): the duplicate add emits no
    // extension_added.
    assert_eq!(events(&server, "extension_added").len(), 1);
}

// 4
#[test]
fn installing_and_removing_a_local_extension_write_three_files() {
    let mut server = empty_components();
    let root = server.root().to_path_buf();
    let three = names(&[
        ".specforge/extensions/@sdk/greet/extension.wasm",
        "specforge.json",
        "specforge.lock",
    ]);

    let (reply, installed) = wrote(
        &mut server,
        &root,
        "specforge.add_extension",
        json!({"specifier": greet()}),
    );
    assert_ok(&reply);
    assert_eq!(installed, three);
    assert_eq!(
        last_completed(&server),
        Some(completed("specforge.add_extension", 3, 0, true))
    );

    let (reply, removed) = wrote(
        &mut server,
        &root,
        "specforge.remove_extension",
        json!({"name": GREET}),
    );
    assert_ok(&reply);
    assert_eq!(removed, three);
    assert_eq!(
        last_completed(&server),
        Some(completed("specforge.remove_extension", 3, 0, true))
    );
}

// 5
#[test]
fn init_with_a_local_extension_writes_five_files() {
    let mut server = empty_components();
    let scratch = tempfile::TempDir::new().unwrap();
    let dir = scratch.path().join("n1");

    let (reply, changed) = wrote(
        &mut server,
        &dir,
        "specforge.init",
        json!({"path": dir.to_str().unwrap(), "name": "newone", "extensions": [greet()]}),
    );

    assert_ok(&reply);
    assert_eq!(
        changed,
        names(&[
            ".gitignore",
            ".specforge/extensions/@sdk/greet/extension.wasm",
            "spec/hello.spec",
            "specforge.json",
            "specforge.lock",
        ])
    );
    // BUG (04-T3 flips): five files written, three reported.
    assert_eq!(
        last_completed(&server),
        Some(completed("specforge.init", 3, 0, true))
    );
}

// 6
#[test]
fn init_beside_a_complete_gitignore_writes_two_files() {
    let mut server = empty_components();
    let scratch = tempfile::TempDir::new().unwrap();
    let dir = scratch.path().join("n2");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(".gitignore"),
        "specforge-infer.json\nspecforge-report.json\n.specforge/\n",
    )
    .unwrap();

    let (reply, changed) = wrote(
        &mut server,
        &dir,
        "specforge.init",
        json!({"path": dir.to_str().unwrap(), "name": "newtwo"}),
    );

    assert_ok(&reply);
    assert_eq!(changed, names(&["spec/hello.spec", "specforge.json"]));
    // BUG (04-T3 flips): two files written, three reported.
    assert_eq!(
        last_completed(&server),
        Some(completed("specforge.init", 3, 0, true))
    );
}

// 7
#[cfg(unix)]
#[test]
fn a_format_that_fails_on_one_file_reports_what_it_wrote() {
    use std::os::unix::fs::PermissionsExt;
    let mut server = TestProject::new()
        .file("a.spec", UNFORMATTED)
        .file("b.spec", &UNFORMATTED.replace("messy", "other"))
        .serve(&[TestExtension::software()]);
    let root = server.root().to_path_buf();
    let locked = root.join("a.spec");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o444)).unwrap();
    let generation = server.state().session_generation();

    let (reply, changed) = wrote(&mut server, &root, "specforge.format", json!({}));
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();

    let error = crate::tool_errors::mcp_error(&reply);
    assert_eq!(error["code"], "internal_error", "{error}");
    assert_eq!(changed, names(&["b.spec"]));
    // BUG (04-T3 flips): one file written, none reported.
    assert_eq!(
        last_completed(&server),
        Some(completed("specforge.format", 0, 0, false))
    );
    // What was written is served.
    assert!(server.state().session_generation() > generation);
}

// 8
#[test]
fn a_migration_writes_the_file_and_its_backup() {
    let mut server = TestProject::new()
        .file("old.spec", OLD)
        .serve(&[TestExtension::software()]);
    let root = server.root().to_path_buf();

    let (reply, changed) = wrote(&mut server, &root, "specforge.migrate", json!({}));

    assert_ok(&reply);
    assert_eq!(changed, names(&["old.spec", "old.spec.bak"]));
    // BUG (04-T3 flips): the file and its backup written, one reported.
    assert_eq!(
        last_completed(&server),
        Some(completed("specforge.migrate", 1, 0, true))
    );
}

/// [`TEST_SPEC`] plus other.spec, whose feature also has `alpha`: renaming
/// `alpha` edits both files.
fn rename_server() -> Served {
    TestProject::new()
        .file("test.spec", TEST_SPEC)
        .file(
            "other.spec",
            "feature delta \"Delta\" {\n    behaviors [alpha]\n}\n",
        )
        .serve(&[TestExtension::software()])
}

// 9
#[test]
fn a_rename_reports_its_files_and_one_entity() {
    let mut server = rename_server();
    let root = server.root().to_path_buf();

    let (reply, changed) = wrote(
        &mut server,
        &root,
        "specforge.rename",
        json!({"entity_id": "alpha", "new_name": "omega", "dry_run": true}),
    );
    assert_ok(&reply);
    assert_eq!(changed, Vec::<PathBuf>::new(), "a dry run writes nothing");
    assert_eq!(last_completed(&server), None, "a dry run is no mutation");

    let (reply, changed) = wrote(
        &mut server,
        &root,
        "specforge.rename",
        json!({"entity_id": "alpha", "new_name": "omega"}),
    );
    assert_ok(&reply);
    assert_eq!(changed, names(&["other.spec", "test.spec"]));
    assert_eq!(
        last_completed(&server),
        Some(completed("specforge.rename", 2, 1, true))
    );
    assert!(tool_json(&reply)["diagnostics"].is_array(), "{reply}");
}

// 10
#[test]
fn infer_session_writes_its_manifest() {
    let mut server = TestProject::new()
        .file("src/main.rs", "fn main() {}\n")
        .serve(&[TestExtension::software()]);
    let root = server.root().to_path_buf();
    let tool = "specforge.infer_session";

    let (reply, changed) = wrote(&mut server, &root, tool, json!({"action": "start"}));
    assert_ok(&reply);
    assert_eq!(changed, names(&["specforge-infer.json"]));
    assert_eq!(last_completed(&server), Some(completed(tool, 1, 0, true)));

    let (reply, changed) = wrote(
        &mut server,
        &root,
        tool,
        json!({
            "action": "mark_analyzed",
            "source_file": "src/main.rs",
            "entities_produced": ["alpha", "beta"],
        }),
    );
    assert_ok(&reply);
    assert_eq!(changed, names(&["specforge-infer.json"]));
    assert_eq!(last_completed(&server), Some(completed(tool, 1, 2, true)));

    // A second session while one is active: refused, nothing written.
    let (reply, changed) = wrote(&mut server, &root, tool, json!({"action": "start"}));
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    assert_eq!(changed, Vec::<PathBuf>::new());
    assert_eq!(last_completed(&server), Some(completed(tool, 0, 0, false)));
}

// 11
#[test]
fn previews_are_no_mutation() {
    let mut server = TestProject::new()
        .enabling(&["@specforge/software"])
        .file("test.spec", TEST_SPEC)
        .file("a.spec", UNFORMATTED)
        .file("old.spec", OLD)
        .serve_components();
    let root = server.root().to_path_buf();
    let before = files_under(&root);
    let generation = server.state().session_generation();

    for (tool, arguments) in [
        ("specforge.format", json!({"check": true})),
        ("specforge.format", json!({"diff": true})),
        ("specforge.format", json!({"write": false})),
        (
            "specforge.rename",
            json!({"entity_id": "alpha", "new_name": "omega", "dry_run": true}),
        ),
        (
            "specforge.add_extension",
            json!({"specifier": PRODUCT, "dry_run": true}),
        ),
        (
            "specforge.add_extension",
            json!({"specifier": greet(), "dry_run": true}),
        ),
        (
            "specforge.remove_extension",
            json!({"name": "@specforge/software", "dry_run": true}),
        ),
        ("specforge.migrate", json!({"dry_run": true})),
    ] {
        let reply = call_tool(&mut server, tool, arguments.clone());
        assert!(reply["result"].is_object(), "{tool} {arguments}: {reply}");
        assert_eq!(
            last_completed(&server),
            None,
            "{tool} {arguments} previewed: {reply}"
        );
    }
    assert_eq!(files_under(&root), before, "a preview writes nothing");
    assert_eq!(server.state().session_generation(), generation);
}

// 12
#[test]
fn a_mutation_refused_for_its_arguments_is_a_failed_mutation() {
    let mut server = rename_server();

    let reply = call_tool(&mut server, "specforge.rename", json!({"entity_id": 3}));
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    assert_eq!(
        events(&server, "mcp_mutation_completed"),
        [completed("specforge.rename", 0, 0, false)]
    );

    let reply = call_tool(
        &mut server,
        "specforge.rename",
        json!({"entity_id": 3, "dry_run": true}),
    );
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    // 04-T3 flips (D6): arguments that do not parse cannot say they asked
    // for a preview: a failed mutation.
    assert_eq!(
        events(&server, "mcp_mutation_completed"),
        [completed("specforge.rename", 0, 0, false)]
    );
}

// 13
#[test]
fn the_domain_event_precedes_the_mutation_event() {
    let mut server = empty_components();

    let since = server.state().events.len();
    assert_ok(&call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": PRODUCT}),
    ));
    assert_eq!(
        event_names_since(
            &server,
            since,
            &[
                "mcp_tool_invoked",
                "extension_added",
                "mcp_mutation_completed"
            ]
        ),
        [
            "mcp_tool_invoked",
            "extension_added",
            "mcp_mutation_completed"
        ]
    );

    let scratch = tempfile::TempDir::new().unwrap();
    let dir = scratch.path().join("fresh");
    let since = server.state().events.len();
    assert_ok(&call_tool(
        &mut server,
        "specforge.init",
        json!({"path": dir.to_str().unwrap(), "name": "fresh"}),
    ));
    assert_eq!(
        event_names_since(
            &server,
            since,
            &[
                "mcp_tool_invoked",
                "project_initialized",
                "mcp_mutation_completed"
            ]
        ),
        [
            "mcp_tool_invoked",
            "project_initialized",
            "mcp_mutation_completed"
        ]
    );
}

// 14
#[test]
fn domain_event_payloads() {
    let mut server = empty_components();
    assert_ok(&call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": PRODUCT}),
    ));
    // BUG (04-T4 flips; spec: extensionSpecifier, totalExtensions,
    // wasDuplicate).
    assert_eq!(
        events(&server, "extension_added"),
        [json!({"extension": PRODUCT, "version": null})]
    );

    let scratch = tempfile::TempDir::new().unwrap();
    let dir = scratch.path().join("newone");
    assert_ok(&call_tool(
        &mut server,
        "specforge.init",
        json!({"path": dir.to_str().unwrap(), "name": "newone"}),
    ));
    // BUG (04-T4 flips; spec: projectName, extensionCount, specFilePath).
    assert_eq!(
        events(&server, "project_initialized"),
        [json!({"name": "newone", "path": dir.display().to_string()})]
    );
}

// 15
#[test]
fn a_write_brings_the_served_project_up_to_date() {
    let mut server = TestProject::new()
        .enabling(&["@specforge/software"])
        .file("test.spec", TEST_SPEC)
        .file("a.spec", UNFORMATTED)
        .file("old.spec", OLD)
        .serve_components();

    for (tool, arguments) in [
        ("specforge.format", json!({})),
        (
            "specforge.rename",
            json!({"entity_id": "alpha", "new_name": "omega"}),
        ),
        ("specforge.add_extension", json!({"specifier": PRODUCT})),
        ("specforge.remove_extension", json!({"name": PRODUCT})),
        ("specforge.migrate", json!({})),
    ] {
        let generation = server.state().session_generation();
        let reply = call_tool(&mut server, tool, arguments.clone());
        assert_ok(&reply);
        assert!(
            server.state().session_generation() > generation,
            "{tool} {arguments} wrote the served project, which was not brought up to date"
        );
    }
    // What is served is what is on disk.
    assert!(server.state().graph().node("omega").is_some());
    assert!(server.state().graph().node("alpha").is_none());
}

// 16
#[test]
fn infer_session_leaves_the_served_project_as_it_is() {
    let mut server = TestProject::new()
        .file("test.spec", TEST_SPEC)
        .file("src/main.rs", "fn main() {}\n")
        .serve(&[TestExtension::software()]);
    let generation = server.state().session_generation();

    assert_ok(&call_tool(
        &mut server,
        "specforge.infer_session",
        json!({"action": "start"}),
    ));

    // specforge-infer.json is no project input.
    assert_eq!(server.state().session_generation(), generation);
}
