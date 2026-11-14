//! `specforge search <text>`: the entity search (ADR 0355), the document `specforge.search` replies, over a copy of
//! the read-views fixture (`behavior alpha "Alpha"`, `beta`, `gamma`, `type Color`, `Red`, `Green`); and the
//! registry search it replaced, `specforge extensions search`.

use serde_json::{Value, json};
use specforge_test::prelude::*;
use std::path::Path;

use crate::support::{Cli, TestProject};

/// A copy of the read-views fixture, which configures no registry.
fn rv1() -> TestProject {
    TestProject::copy_of(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/read_views/rv1"),
    )
}

/// MCP's reply to `specforge.search` with `arguments` over `project`.
fn mcp(project: &TestProject, arguments: Value) -> Value {
    let mut server = crate::support::pairing::mcp_on(project.root());
    let reply = specforge_mcp::testing::rpc::call_tool(&mut server, "specforge.search", arguments);
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    serde_json::from_str(text).unwrap()
}

/// The ids of a search document's hits, in order.
fn ids(document: &Value) -> Vec<&str> {
    document["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| hit["entity_id"].as_str().unwrap())
        .collect()
}

#[specforge_test(
    behavior = "provide_search_cli",
    verify = "search prints the entities specforge.search returns, best first"
)]
fn search_prints_the_hits_specforge_search_returns() {
    let project = rv1();
    let cli = Cli::on(&project);
    let json = cli.run(&["search", "alpha", "--format", "json"]).exits(0);
    let reply = mcp(&project, json!({"query": "alpha"}));
    assert_eq!(json.json(), reply);
    assert_eq!(ids(&reply)[0], "alpha", "{reply}");

    let human = cli.run(&["search", "alpha"]).exits(0);
    let first = human.stdout.lines().next().unwrap_or_default();
    assert!(
        first.starts_with("behavior alpha  Alpha  ") && first.ends_with("main.spec:1"),
        "{}",
        human.stdout
    );
    // A match in a field's text shows the field and the text around it.
    let field = cli.run(&["search", "MUST g"]).exits(0);
    assert!(
        field.stdout.contains("\n  contract: MUST g\n"),
        "{}",
        field.stdout
    );
    let picked = cli.run(&["search", "alpha", "--pick", ".results[0].entity_id"]);
    assert_eq!(picked.exits(0).stdout, "alpha\n");
}

#[specforge_test(
    behavior = "provide_search_cli",
    verify = "search with --kinds keeps only those kinds"
)]
fn search_kinds_keeps_only_those_kinds() {
    let project = rv1();
    let run = Cli::on(&project).run(&["search", "", "--kinds", "type", "--format", "json"]);
    let document = run.exits(0).json();
    let mut found = ids(&document);
    found.sort_unstable();
    assert_eq!(found, ["Color", "Green", "Red"], "{document}");
}

#[specforge_test(
    behavior = "provide_search_cli",
    verify = "search with no hit says no entity matches"
)]
fn search_with_no_hit_says_no_entity_matches() {
    let project = rv1();
    let run = Cli::on(&project).run(&["search", "zzzzqqq"]).exits(0);
    assert_eq!(run.stdout, "no entity matches 'zzzzqqq'\n");
}

#[test]
fn extensions_search_with_no_registry_is_e063_on_extensions_search() {
    let project = rv1();
    let run = Cli::on(&project)
        .run(&["extensions", "search", "alpha"])
        .exits(1);
    assert!(run.stderr.contains("E063"), "{}", run.stderr);
    assert!(
        run.stderr.contains("`extensions search` needs one"),
        "{}",
        run.stderr
    );
}
