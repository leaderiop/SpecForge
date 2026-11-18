//! What a pass that declares the `delta` input is sent (ADR 0376 D6, D7). The guest is `@test/delta`: the kinds `item`
//! (whose `refs` are edges to a `note`) and `note`, and check passes that declare the delta of `item` (`items`,
//! `twin`), of `note` (`notes`), of no kind (`empty`), or nothing (`plain`). Each payload is read from the inputs the
//! runtime recorded.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;
use specforge_graph::Graph;
use specforge_project::CompiledProject;
use specforge_project::delta::Delta;
use specforge_project::testing::{TestProject, pass_input};
use specforge_test::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

const EXT: &str = "@test/delta";

/// Three items, two notes, and one edge: `a` refers to `n1`.
const SPEC: &str = "item a \"A\" {\n  refs [n1]\n}\n\nitem b \"B\" {\n}\n\nitem c \"C\" {\n}\n\n\
                    note n1 \"N1\" {\n}\n\nnote n2 \"N2\" {\n}\n";

fn nothing(_: &PassInput) -> Vec<PassDiagnostic> {
    Vec::new()
}

/// `@test/delta`'s kinds and its check passes.
fn delta_extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.kind("item", |k| {
        k.field("refs", |f| {
            f.field_type(FieldType::ReferenceList)
                .edge("Refs")
                .target_kind("note");
        });
    });
    c.kind("note", |_| {});
    c.edge("Refs", |e| {
        e.source_kind("item").target_kind("note");
    });
    for (name, kinds) in [
        ("items", &["item"][..]),
        ("twin", &["item"][..]),
        ("notes", &["note"][..]),
        ("empty", &[][..]),
    ] {
        c.pass(name, |p| {
            p.phase("check").input(Input::delta(kinds)).run(nothing);
        });
    }
    c.pass("plain", |p| {
        p.phase("check").run(nothing);
    });
    c
}

/// `files` compiled cold with `@test/delta` loaded, and the runtime it compiled in.
fn compiled(files: &[(&str, &str)]) -> (CompiledProject, Arc<InProcessRuntime>) {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .files(files);
    let runtime = Arc::new(InProcessRuntime::new().with(delta_extension));
    let compiled = project.compile_in(runtime.clone());
    (compiled, runtime)
}

/// The delta `pass` was sent.
fn delta_of(runtime: &InProcessRuntime, pass: &str) -> Value {
    pass_input(runtime, pass)["delta"].clone()
}

/// The delta from the empty graph to `compiled`'s, restricted to `kinds`, as the wire carries it.
fn cold(compiled: &CompiledProject, kinds: &[&str]) -> Value {
    let delta = Delta::between(
        &Graph::default(),
        compiled.graph(),
        compiled.environment().registries().field_forms(),
        DeltaBase::None,
    );
    serde_json::to_value(delta.restricted_to(kinds)).unwrap()
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_cold_compile_sends_every_node_of_the_named_kinds_added() {
    let (compiled, runtime) = compiled(&[("a.spec", SPEC)]);
    let delta = delta_of(&runtime, "items");
    assert_eq!(delta["base"], json!({"kind": "none"}));
    let nodes = delta["nodes"].as_array().unwrap();
    let named: Vec<(&str, &str, &str)> = nodes
        .iter()
        .map(|n| {
            (
                n["kind"].as_str().unwrap(),
                n["id"].as_str().unwrap(),
                n["change"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        named,
        [
            ("item", "a", "added"),
            ("item", "b", "added"),
            ("item", "c", "added")
        ]
    );
    for node in nodes {
        assert!(
            node["after"].as_str().is_some_and(|d| !d.is_empty()),
            "{node}"
        );
        assert!(node.get("before").is_none(), "{node}");
    }
    let edges = delta["added_edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1, "{delta}");
    assert_eq!(
        (&edges[0]["source"], &edges[0]["target"]),
        (&json!("a"), &json!("n1"))
    );
    assert!(delta.get("removed_edges").is_none(), "{delta}");
    // Exactly the delta the host takes of the graph, digests included.
    assert_eq!(delta, cold(&compiled, &["item"]));
    let held = compiled.delta().expect("a loaded pass declares the delta");
    assert_eq!(
        serde_json::to_value(held.restricted_to(&["item"])).unwrap(),
        delta
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_pass_is_sent_no_node_of_a_kind_it_did_not_name() {
    let (compiled, runtime) = compiled(&[("a.spec", SPEC)]);
    let items = delta_of(&runtime, "items");
    assert!(
        items["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["kind"] == "item"),
        "{items}"
    );
    let notes = delta_of(&runtime, "notes");
    let ids: Vec<&str> = notes["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["n1", "n2"]);
    assert!(
        notes["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["kind"] == "note"),
        "{notes}"
    );
    // The edge has an end of each kind: both readers are sent it.
    assert_eq!(notes["added_edges"], items["added_edges"]);
    assert_eq!(notes, cold(&compiled, &["note"]));
}

#[specforge_test(requirement = "FEATURE-142")]
fn two_passes_of_the_same_kinds_are_sent_the_same_delta() {
    let (_, runtime) = compiled(&[("a.spec", SPEC)]);
    let items = serde_json::to_string(&delta_of(&runtime, "items")).unwrap();
    let twin = serde_json::to_string(&delta_of(&runtime, "twin")).unwrap();
    assert_eq!(items, twin);
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_pass_that_declares_no_delta_is_sent_no_delta_key() {
    let (_, runtime) = compiled(&[("a.spec", SPEC)]);
    assert!(pass_input(&runtime, "plain").get("delta").is_none());
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_delta_input_naming_no_kind_is_sent_the_base_and_no_node() {
    let (_, runtime) = compiled(&[("a.spec", SPEC)]);
    assert_eq!(
        delta_of(&runtime, "empty"),
        json!({"base": {"kind": "none"}})
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_project_with_no_node_of_the_kind_sends_the_base_alone() {
    let (_, runtime) = compiled(&[("a.spec", "note n1 \"N1\" {\n}\n")]);
    assert_eq!(
        delta_of(&runtime, "items"),
        json!({"base": {"kind": "none"}})
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn no_delta_is_taken_when_no_pass_declares_it() {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .files(&[("a.spec", SPEC)]);
    let runtime = Arc::new(InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
        c.kind("item", |_| {});
        c.pass("plain", |p| {
            p.phase("check").run(nothing);
        });
        c
    }));
    let compiled = project.compile_in(runtime.clone());
    assert!(compiled.delta().is_none());
    assert!(pass_input(&runtime, "plain").get("delta").is_none());
}
