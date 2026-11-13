//! `specforge refs <entity>`: the references of an entity (ADR 0355), the document `specforge.find_references`
//! replies, over a copy of the read-views fixture (`type Color "Color" = Red | Green`).

use serde_json::{Value, json};
use specforge_test::prelude::*;
use std::path::Path;

use crate::support::{Cli, TestProject};

/// A copy of the read-views fixture.
fn rv1() -> TestProject {
    TestProject::copy_of(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/read_views/rv1"),
    )
}

/// MCP's reply to `specforge.find_references` with `arguments` over `project`.
fn mcp(project: &TestProject, arguments: Value) -> Value {
    let mut server = crate::support::pairing::mcp_on(project.root());
    let reply =
        specforge_mcp::testing::rpc::call_tool(&mut server, "specforge.find_references", arguments);
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    serde_json::from_str(text).unwrap()
}

#[specforge_test(
    behavior = "provide_refs_cli",
    verify = "refs prints the locations find_references returns"
)]
fn refs_prints_the_locations_find_references_returns() {
    let project = rv1();
    let cli = Cli::on(&project);
    let json = cli.run(&["refs", "Red", "--format", "json"]).exits(0);
    let reply = mcp(&project, json!({"entity_id": "Red"}));
    assert_eq!(json.json(), reply);
    assert_eq!(reply["locations"].as_array().unwrap().len(), 1, "{reply}");

    let human = cli.run(&["refs", "Red"]).exits(0);
    assert!(
        human
            .stdout
            .ends_with("spec/main.spec:10:22  Color.composed_types → Red (reference)\n"),
        "{}",
        human.stdout
    );
    let both = cli.run(&[
        "refs",
        "Red",
        "--direction",
        "both",
        "--include-declaration",
    ]);
    let both = both.exits(0).stdout;
    assert!(both.contains("  Red (declaration)\n"), "{both}");
    let none = cli.run(&["refs", "alpha"]).exits(0);
    assert_eq!(none.stdout, "no reference to 'alpha'\n");
    let picked = cli.run(&[
        "refs",
        "Red",
        "--pick",
        ".locations[].referencing_entity_id",
    ]);
    assert_eq!(picked.exits(0).stdout, "Color\n");
}

#[specforge_test(
    behavior = "provide_refs_cli",
    verify = "refs of an unknown entity is E003"
)]
fn refs_of_an_unknown_entity_is_e003() {
    let project = rv1();
    let run = Cli::on(&project).run(&["refs", "ghost"]).exits(1);
    assert!(run.stderr.contains("E003"), "{}", run.stderr);
}
