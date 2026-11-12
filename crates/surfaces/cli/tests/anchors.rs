//! `specforge anchors <ENTITY> | --file <PATH>`: the anchors read view (ADR 0349) on the command line, the documents
//! `specforge.find_implementation` and `specforge.find_spec_for_source` reply, over a project with a hand-written
//! `rust-source` store whose item `rust:anc::alpha` names `behavior alpha`.

use serde_json::{Value, json};
use specforge_project::testing::{WORKTREE, item, naming, stored_with};
use specforge_test::prelude::*;

use crate::support::{Cli, TestProject};

const SOURCE: &str = "pub fn alpha() {}\npub fn beta() {}\n";

/// A project enabling `@code/core`, `behavior alpha` and `behavior beta`, `src/lib.rs` as collected: its digest in
/// the store.
fn anchored() -> TestProject {
    let project = TestProject::new()
        .file(
            "specforge.json",
            r#"{"name":"anc","spec_root":"spec","extensions":["@specforge/software","@code/core"]}"#,
        )
        .file(
            "spec/main.spec",
            "behavior alpha \"Alpha\" {\n  contract \"c\"\n}\n\nbehavior beta \"Beta\" {\n  contract \"c\"\n}\n",
        )
        .file("src/lib.rs", SOURCE);
    let sources = [(
        "src/lib.rs".to_string(),
        specforge_ops::revision::of_bytes(SOURCE.as_bytes()),
    )];
    stored_with(
        project.root(),
        "rust-source",
        WORKTREE,
        json!([
            naming(
                item(
                    "rust:anc::alpha",
                    "src/lib.rs",
                    1,
                    "function",
                    "public",
                    "alpha"
                ),
                &["alpha"]
            ),
            item(
                "rust:anc::beta",
                "src/lib.rs",
                2,
                "function",
                "public",
                "beta"
            ),
        ]),
        sources.into_iter().collect(),
    );
    project
}

/// MCP's reply to `tool` with `arguments` over `project`.
fn mcp(project: &TestProject, tool: &str, arguments: Value) -> Value {
    let mut server = crate::support::pairing::mcp_on(project.root());
    let reply = specforge_mcp::testing::rpc::call_tool(&mut server, tool, arguments);
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    serde_json::from_str(text).unwrap()
}

#[specforge_test(
    behavior = "provide_anchors_cli",
    verify = "anchors <entity> prints the rows find_implementation returns"
)]
fn anchors_of_an_entity_are_find_implementations_rows() {
    let project = anchored();
    let cli = Cli::on(&project);
    let json = cli.run(&["anchors", "alpha", "--format", "json"]);
    assert_eq!(json.code, 0, "{}{}", json.stdout, json.stderr);
    let reply = mcp(
        &project,
        "specforge.find_implementation",
        json!({"entity_id": "alpha"}),
    );
    assert_eq!(json.json(), reply);
    assert_eq!(reply["count"], 1, "{reply}");

    let human = cli.run(&["anchors", "alpha"]);
    assert_eq!(human.code, 0, "{}", human.stderr);
    assert_eq!(
        human.stdout,
        "src/lib.rs:1-1  function rust:anc::alpha  via item.names\n"
    );
    // The file is as the store recorded it: no W198; facts are served: no I212.
    assert!(!human.stderr.contains("W198"), "{}", human.stderr);
    assert!(!human.stderr.contains("I212"), "{}", human.stderr);
    // An entity with no anchor says so and passes.
    let none = cli.run(&["anchors", "beta"]);
    assert_eq!(
        (none.code, none.stdout.as_str()),
        (0, "no anchor for 'beta'\n")
    );
}

#[specforge_test(
    behavior = "provide_anchors_cli",
    verify = "anchors --file prints the rows find_spec_for_source returns"
)]
fn anchors_of_a_file_are_find_spec_for_sources_rows() {
    let project = anchored();
    let cli = Cli::on(&project);
    let json = cli.run(&["anchors", "--file", "src/lib.rs", "--format", "json"]);
    assert_eq!(json.code, 0, "{}{}", json.stdout, json.stderr);
    let reply = mcp(
        &project,
        "specforge.find_spec_for_source",
        json!({"file_path": "src/lib.rs"}),
    );
    assert_eq!(json.json(), reply);

    let human = cli.run(&["anchors", "--file", "src/lib.rs"]);
    assert_eq!(
        human.stdout,
        "src/lib.rs:1-1  function rust:anc::alpha  via item.names  [behavior alpha]\n"
    );

    // An edit since the collect: the rows stay, W198 on stderr.
    std::fs::write(project.root().join("src/lib.rs"), "pub fn alpha() {}\n").unwrap();
    let stale = cli.run(&["anchors", "--file", "src/lib.rs"]);
    assert_eq!(stale.code, 0, "{}", stale.stderr);
    assert_eq!(stale.stdout, human.stdout);
    assert!(stale.stderr.contains("warning[W198]"), "{}", stale.stderr);
}

#[specforge_test(
    behavior = "provide_anchors_cli",
    verify = "an entity and --file together are refused"
)]
fn an_entity_and_a_file_together_are_refused() {
    let project = anchored();
    let cli = Cli::on(&project);
    let both = cli.run(&["anchors", "alpha", "--file", "src/lib.rs"]);
    assert_eq!(both.code, 2, "{}", both.stderr);
    assert!(
        both.stderr.contains("cannot be used with"),
        "{}",
        both.stderr
    );
    let neither = cli.run(&["anchors"]);
    assert_eq!(neither.code, 2, "{}", neither.stderr);
}

#[specforge_test(
    behavior = "provide_anchors_cli",
    verify = "anchors answers a Rust test's file and line after a real collect"
)]
fn anchors_answers_a_collected_rust_item() {
    // The Rust packs' fixture crate, collected for real: `rust-cargo` reads `Cargo.toml`, `rust-source` the crate's
    // files through the host's reader; no command runs.
    let fixture = specforge_repo_root::repo_root().join("crates/app/ops/tests/fixtures/rust-packs");
    let packs = [
        "@specforge/software",
        "@code/core",
        "@rust/cargo",
        "@rust/source",
    ];
    let project = TestProject::copy_of(fixture).enabling(&packs);
    let cli = Cli::on(&project);
    let collect = cli.run(&["collect", "--yes"]);
    assert_eq!(collect.code, 0, "{}{}", collect.stdout, collect.stderr);

    let human = cli.run(&["anchors", "rs_mark_tests"]);
    assert_eq!(
        (human.code, human.stdout.as_str()),
        (
            0,
            "src/lib.rs:14-16  function rust:adder::tests::adds  via item.names\n"
        ),
        "{}",
        human.stderr
    );
    assert!(
        !human.stderr.contains("I212") && !human.stderr.contains("W198"),
        "{}",
        human.stderr
    );
    let json = cli.run(&["anchors", "rs_mark_tests", "--format", "json"]);
    let reply = mcp(
        &project,
        "specforge.find_implementation",
        json!({"entity_id": "rs_mark_tests"}),
    );
    assert_eq!(json.json(), reply);
    let file = cli.run(&["anchors", "--file", "src/lib.rs"]);
    assert_eq!(
        file.stdout,
        "src/lib.rs:14-16  function rust:adder::tests::adds  via item.names  [behavior rs_mark_tests]\n"
    );
}
