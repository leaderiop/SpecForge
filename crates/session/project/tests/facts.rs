//! The fact stores a session serves (ADR 0327 D5, D9): each one `specforge-harness` records is read-only entities of
//! the graph, gated off when no collector-written kind is loaded; a store or the ledger that changes is a source
//! change of the stores' keys. The fixture is `Collected` (`@t/code`'s `test`, `@t/node`'s `node`, `@t/spec`'s
//! `truth`).

use std::fs;
use std::sync::Arc;

use serde_json::{Value, json};
use specforge_project::testing::{COLLECTED_EXTENSIONS, Collected, TestProject, stored};
use specforge_project::{CompiledProject, ProjectSession};
use specforge_test::prelude::*;

const KEY: &str = "0123456789abcdef0123456789abcdef01234567";
const STORE: &str = ".specforge/facts/0123456789abcdef0123456789abcdef01234567/node.json";

/// `node`'s facts: two tests of `a.test.js`.
fn two_tests() -> Value {
    json!([
        {"kind": "test", "id": "js:a.test.js#adds", "title": "adds", "fields": {"file": "a.test.js", "status": "passed"}},
        {"kind": "test", "id": "js:a.test.js#subtracts", "fields": {"file": "a.test.js", "status": "failed"}},
    ])
}

/// A project enabling the fixture, whose truth cites one of the tests.
fn collected() -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&COLLECTED_EXTENSIONS)
        .installing_enabled()
        .file(
            "truths.spec",
            "truth T1 \"One\" {\n  proof [\"js:a.test.js#adds\"]\n}\n",
        )
}

fn compiled(p: &TestProject) -> CompiledProject {
    p.compile_in(Arc::new(Collected::new().runtime()))
}

fn session(p: &TestProject) -> ProjectSession {
    p.session_in(Arc::new(Collected::new().runtime()))
}

fn codes(project: &CompiledProject) -> Vec<String> {
    project
        .diagnostics()
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

fn export(project: &CompiledProject) -> String {
    specforge_emitter::emit(project.graph(), &specforge_emitter::EmitOptions::default()).unwrap()
}

#[specforge_test(
    behavior = "build_in_memory_graph",
    verify = "a project that loads no collector-written kind reads no fact store and compiles byte-identically"
)]
fn with_no_collected_kind_a_fact_store_on_disk_changes_no_output() {
    let p = TestProject::new().config(|c| c["name"] = "p".into()).file(
        "a.spec",
        "thing alpha \"A\" {\n  uses beta\n}\n\nthing beta \"B\" {}\n",
    );
    let before = p.compile();

    stored(p.root(), "node", KEY, two_tests());
    let after = p.compile();

    assert_eq!(
        format!("{:?}", before.diagnostics()),
        format!("{:?}", after.diagnostics())
    );
    assert_eq!(export(&before), export(&after));
    assert_eq!(before.source_keys(), after.source_keys());
}

#[specforge_test(
    behavior = "build_in_memory_graph",
    verify = "a fact store specforge-harness records is served as read-only entities; one it does not is W191, one that does not read W192"
)]
fn a_served_fact_is_an_entity_every_query_reads() {
    let p = collected();
    stored(p.root(), "node", KEY, two_tests());

    let project = compiled(&p);

    let adds = project
        .graph()
        .node("js:a.test.js#adds")
        .expect("the fact is a node");
    assert_eq!(adds.kind.raw.as_str(), "test");
    let cited: Vec<&str> = project
        .graph()
        .edges_from("T1")
        .iter()
        .map(|e| e.target.as_str())
        .collect();
    assert_eq!(cited, ["js:a.test.js#adds"]);
    assert!(export(&project).contains("js:a.test.js#subtracts"));
    assert!(project.source_keys().contains(STORE));
    assert!(codes(&project).is_empty(), "{:?}", project.diagnostics());
}

#[specforge_test(
    behavior = "build_in_memory_graph",
    verify = "a fact's single-value reference to another fact links like a written one"
)]
fn a_facts_single_value_reference_links_like_a_written_one() {
    use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta, FieldType};
    use specforge_project::testing::CODE;
    use specforge_wasm::testing::InProcessRuntime;

    // `@t/run`'s `run` names the `test` it ran in one reference, as a runner pack's `test_run` does.
    let run = || {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/run", "1.0.0"));
        c.peer(CODE, "^1.0");
        c.kind("run", |k| {
            k.collected(&["run"]).field("test", |f| {
                f.field_type(FieldType::Reference).target_kind("test");
            });
        });
        c
    };
    let mut enabled = COLLECTED_EXTENSIONS.to_vec();
    enabled.push("@t/run");
    let p = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&enabled)
        .installing_enabled();
    let mut facts = two_tests();
    facts.as_array_mut().unwrap().push(
        json!({"kind": "run", "id": "run:js:a.test.js#adds", "fields": {"test": "js:a.test.js#adds"}}),
    );
    stored(p.root(), "node", KEY, facts);

    // Before its value kept a span, the edge was held by no written token: the debug build's edge check panicked.
    let project = p.compile_in(Arc::new(
        Collected::new().over(InProcessRuntime::new().with(run)),
    ));

    let ran: Vec<&str> = project
        .graph()
        .edges_from("run:js:a.test.js#adds")
        .iter()
        .map(|e| e.target.as_str())
        .collect();
    assert_eq!(ran, ["js:a.test.js#adds"]);
    assert!(codes(&project).is_empty(), "{:?}", project.diagnostics());
}

#[test]
fn a_store_the_ledger_does_not_record_is_w191_and_serves_nothing() {
    let p = collected();
    stored(p.root(), "node", KEY, two_tests());
    fs::remove_file(p.root().join("specforge-harness")).unwrap();

    let project = compiled(&p);

    assert!(project.graph().node("js:a.test.js#adds").is_none());
    assert!(
        codes(&project).contains(&"W191".to_string()),
        "{:?}",
        project.diagnostics()
    );
}

#[test]
fn a_rewritten_fact_store_is_a_source_change_of_its_key() {
    let p = collected();
    stored(p.root(), "node", KEY, two_tests());
    let mut session = session(&p);
    assert!(
        session
            .project()
            .graph()
            .node("js:a.test.js#adds")
            .is_some()
    );

    stored(
        p.root(),
        "node",
        KEY,
        json!([{"kind": "test", "id": "js:a.test.js#subtracts"}]),
    );
    let root = fs::canonicalize(p.root()).unwrap();
    let changes = session.changes([root.join(STORE).as_path()]);

    assert_eq!(changes.sources(), [STORE]);
    assert!(session.apply(&changes).is_some());
    let graph = session.project().graph();
    assert!(graph.node("js:a.test.js#adds").is_none());
    assert!(graph.node("js:a.test.js#subtracts").is_some());
    assert!(session.stale().is_empty(), "{:?}", session.stale());
}

#[specforge_test(
    behavior = "watch_file_system_for_changes",
    verify = "a fact store and the harness ledger are watched inputs when a collector-written kind is loaded"
)]
fn a_changed_harness_ledger_reserves_the_fact_keys() {
    let p = collected();
    stored(p.root(), "node", KEY, two_tests());
    let mut session = session(&p);
    assert!(
        session
            .project()
            .graph()
            .node("js:a.test.js#adds")
            .is_some()
    );

    // Another branch's ledger: it records no store.
    fs::write(
        p.root().join("specforge-harness"),
        "# specforge-harness 1\n",
    )
    .unwrap();
    let root = fs::canonicalize(p.root()).unwrap();
    let changes = session.changes([root.join("specforge-harness").as_path()]);

    assert_eq!(changes.sources(), [STORE]);
    assert!(session.apply(&changes).is_some());
    assert!(
        session
            .project()
            .graph()
            .node("js:a.test.js#adds")
            .is_none()
    );
    assert!(codes(session.project()).contains(&"W191".to_string()));
    assert!(session.stale().is_empty(), "{:?}", session.stale());
}
