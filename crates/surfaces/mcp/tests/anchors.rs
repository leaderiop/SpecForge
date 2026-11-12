//! `specforge.find_implementation` and `specforge.find_spec_for_source` over a served project: the anchors read view
//! over the graph (ADR 0349), an anchor an edge between an authored entity and a located fact.

use crate::support::*;
use serde_json::{Value, json};
use specforge_project::testing::{item, naming, stored_items};
use specforge_test::prelude::*;

/// `behavior alpha` and `behavior beta`, and a `task wire` citing `rust:p::net::Wire`.
const SPEC: &str = concat!(
    "behavior alpha \"A\" {\n    contract \"c\"\n}\n",
    "behavior beta \"B\" {\n    contract \"c\"\n}\n",
    "task wire \"W\" {\n    items [\"rust:p::net::Wire\"]\n}\n",
);

/// A server over [`SPEC`] serving the software kinds and the item fixtures, its `rust-source` store holding `facts`.
fn served(facts: Vec<Value>) -> Served {
    let project = TestProject::new().file("test.spec", SPEC);
    if !facts.is_empty() {
        stored_items(project.root(), "rust-source", facts);
    }
    project.serve(&[
        TestExtension::software(),
        TestExtension::items(),
        TestExtension::cites(),
    ])
}

/// The JSON reply of `tool` called with `args`.
fn result(server: &mut Served, tool: &str, args: Value) -> Value {
    let reply = call_tool(server, tool, args);
    serde_json::from_str(&tool_text(&reply)).unwrap_or_else(|e| panic!("{e}: {reply}"))
}

/// The public function item `id` at `line` of `file`.
fn at(id: &str, file: &str, line: usize) -> Value {
    item(
        id,
        file,
        line,
        "function",
        "public",
        id.rsplit("::").next().unwrap(),
    )
}

#[specforge_test(
    behavior = "provide_mcp_find_implementation_tool",
    verify = "find_implementation lists every anchor of the entity, by file and line"
)]
fn find_implementation_lists_every_anchor_of_the_entity() {
    let mut server = served(vec![
        naming(at("rust:p::net::serve", "src/net.rs", 4), &["alpha"]),
        naming(at("rust:p::run", "src/lib.rs", 1), &["alpha", "beta"]),
    ]);
    let found = result(
        &mut server,
        "specforge.find_implementation",
        json!({"entity_id": "alpha"}),
    );
    let row = |fact: &str, file: &str, line: usize| {
        json!({"fact_id": fact, "file": file, "line": line, "end_line": line, "item_kind": "function",
               "collector": "rust-source", "via": "item.names"})
    };
    assert_eq!(
        found,
        json!({
            "entity_id": "alpha",
            "count": 2,
            "implementations": [row("rust:p::run", "src/lib.rs", 1), row("rust:p::net::serve", "src/net.rs", 4)],
        })
    );

    let refused = call_tool(
        &mut server,
        "specforge.find_implementation",
        json!({"entity_id": "nope"}),
    );
    let error = crate::tool_errors::mcp_error(&refused);
    assert_eq!(error["code"], "entity_not_found", "{error}");
    assert_eq!(error["argument"], "entity_id", "{error}");
    assert_eq!(error["diagnostic"]["code"], "E003", "{error}");
}

#[specforge_test(
    behavior = "provide_mcp_find_spec_for_source_tool",
    verify = "find_spec_for_source lists the anchors of every located fact the path names"
)]
fn find_spec_for_source_lists_the_entities_anchored_in_the_file() {
    let mut server = served(vec![
        naming(at("rust:p::net::serve", "src/net.rs", 4), &["beta"]),
        at("rust:p::net::Wire", "src/net.rs", 9),
    ]);
    let found = result(
        &mut server,
        "specforge.find_spec_for_source",
        json!({"file_path": "src/net.rs"}),
    );
    let entities: Vec<(&str, &str, &str)> = found["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["entity_id"].as_str().unwrap(),
                e["kind"].as_str().unwrap(),
                e["via"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        entities,
        [
            ("beta", "behavior", "item.names"),
            ("wire", "task", "task.items")
        ]
    );
    assert_eq!(found["match_mode"], "exact", "{found}");
}

#[specforge_test(
    behavior = "provide_mcp_find_implementation_tool",
    verify = "an answer with no located fact served carries I212"
)]
fn find_implementation_says_when_nothing_was_collected() {
    let mut server = served(vec![]);
    let reply = call_tool(
        &mut server,
        "specforge.find_implementation",
        json!({"entity_id": "alpha"}),
    );
    let found: Value = serde_json::from_str(&tool_text(&reply)).unwrap();
    assert_eq!(found["count"], 0, "{reply}");
    let codes: Vec<&str> = reply["result"]["_meta"]["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("no notices: {reply}"))
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["I212"], "{reply}");
}

/// The Rust packs' fixture crate, `@code/core`, `@rust/cargo` and `@rust/source` enabled, served and collected for
/// real through `specforge.collect`: `rust-cargo` reads `Cargo.toml`, `rust-source` the crate's files; no command runs.
fn collected_crate() -> (TestProject, specforge_mcp::McpServer) {
    let fixture = specforge_repo_root::repo_root().join("crates/app/ops/tests/fixtures/rust-packs");
    let project = TestProject::copy_of(fixture).enabling(&[
        "@specforge/software",
        "@code/core",
        "@rust/cargo",
        "@rust/source",
    ]);
    let mut server = specforge_mcp::McpServer::with_project_root(project.root().to_path_buf());
    let reply = call(&mut server, "initialize", json!({}));
    assert!(reply["error"].is_null(), "{reply}");
    let collected = call_tool(&mut server, "specforge.collect", json!({}));
    assert!(collected["result"]["isError"] != true, "{collected}");
    (project, server)
}

#[specforge_test(
    behavior = "provide_mcp_find_implementation_tool",
    verify = "find_implementation answers a Rust test's file and line after a real collect"
)]
fn find_implementation_answers_a_collected_rust_item() {
    let (_project, mut server) = collected_crate();
    let reply = call_tool(
        &mut server,
        "specforge.find_implementation",
        json!({"entity_id": "rs_mark_tests"}),
    );
    let found: Value = serde_json::from_str(&tool_text(&reply)).unwrap();
    // `tests::adds` in `src/lib.rs`, its `#[specforge_test(behavior = "rs_mark_tests")]` at line 14.
    assert_eq!(
        found["implementations"],
        json!([{"fact_id": "rust:adder::tests::adds", "file": "src/lib.rs", "line": 14, "end_line": 16,
                "item_kind": "function", "collector": "rust-source", "via": "item.names"}]),
        "{reply}"
    );
    // The files are as collected and facts are served: no notice.
    assert!(reply["result"]["_meta"]["diagnostics"].is_null(), "{reply}");

    let source = call_tool(
        &mut server,
        "specforge.find_spec_for_source",
        json!({"file_path": "src/lib.rs"}),
    );
    let found: Value = serde_json::from_str(&tool_text(&source)).unwrap();
    let entities: Vec<&str> = found["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["entity_id"].as_str().unwrap())
        .collect();
    assert_eq!(entities, ["rs_mark_tests"], "{found}");
}
