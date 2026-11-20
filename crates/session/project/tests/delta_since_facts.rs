//! The facts in the delta of a check since a revision (ADR 0376 D4, D7): a fact store is never in git, so a collector
//! served at the merge-base or in the working tree only is left out of the delta a pass is sent, never read as every
//! fact of it removed or added; one served on both sides changes as its facts do. The guest `@t/reader` peers the
//! collected fixture's `@t/code` and declares the delta of its kind `test`.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;
use specforge_git::testing::Repo;
use specforge_project::CompiledProject;
use specforge_project::harness::LEDGER_FILE;
use specforge_project::testing::{
    CODE, COLLECTED_EXTENSIONS, Collected, TestProject, pass_input, stored,
};
use specforge_test::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

const READER: &str = "@t/reader";
const FIRST: &str = "thing x \"X\" {\n  note \"first\"\n}\n";
const SECOND: &str = "thing x \"X\" {\n  note \"second\"\n}\n";
const KEY: &str = "0123456789abcdef0123456789abcdef01234567";
const NEXT: &str = "89abcdef0123456789abcdef0123456789abcdef";
const STORE: &str = ".specforge/facts/0123456789abcdef0123456789abcdef01234567/node.json";
const ARCHIVED: &str =
    ".specforge/facts-history/000001-0123456789abcdef0123456789abcdef01234567/node.json";

/// `node`'s facts: two tests of `a.test.js`.
fn two_tests() -> Value {
    json!([
        {"kind": "test", "id": "js:a.test.js#adds", "fields": {"file": "a.test.js", "status": "passed"}},
        {"kind": "test", "id": "js:a.test.js#subtracts", "fields": {"file": "a.test.js", "status": "failed"}},
    ])
}

fn nothing(_: &PassInput) -> Vec<PassDiagnostic> {
    Vec::new()
}

/// `@t/reader`: the check pass `facts`, reading the delta of `test`.
fn reader() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(READER, "1.0.0"));
    c.peer(CODE, "^1.0");
    c.pass("facts", |p| {
        p.phase("check").input(Input::delta(&["test"])).run(nothing);
    });
    c
}

/// A project of [`FIRST`] enabling the collected fixture and [`READER`], in a fresh repository.
fn project() -> (TestProject, Repo) {
    let mut enabled = COLLECTED_EXTENSIONS.to_vec();
    enabled.push(READER);
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&enabled)
        .installing_enabled()
        .file("a.spec", FIRST);
    let repo = Repo::init(project.root());
    (project, repo)
}

/// `project` checked since `revision` with the real git port, and the runtime its passes ran in.
fn since(project: &TestProject, revision: &str) -> (CompiledProject, Arc<InProcessRuntime>) {
    let runtime = Arc::new(Collected::new().over(InProcessRuntime::new().with(reader)));
    let git: specforge_effects::SharedGit = Arc::new(specforge_git::Gix::default());
    let env = project.environment_in(runtime.clone()).with_git(git);
    let compiled = CompiledProject::of_since(env, revision).expect("a revision");
    (compiled, runtime)
}

/// The id and change of each node of the delta `facts` was sent, and the delta.
fn sent(runtime: &InProcessRuntime) -> (Vec<(String, String)>, Value) {
    let delta = pass_input(runtime, "facts")["delta"].clone();
    let nodes = delta["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|n| {
            let text = |key: &str| n[key].as_str().unwrap().to_string();
            (text("id"), text("change"))
        })
        .collect();
    (nodes, delta)
}

/// Whether the delta `compiled` took names the node `id` as `change`.
fn names(compiled: &CompiledProject, id: &str, change: &str) -> bool {
    let delta = serde_json::to_value(compiled.delta().expect("a pass reads the delta").payload());
    delta.unwrap()["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["id"] == id && n["change"] == change)
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_collector_served_at_the_merge_base_only_is_no_removal() {
    let (project, repo) = project();
    stored(project.root(), "node", KEY, two_tests());
    let collected = project.read(STORE);
    let base = repo.commit_all("collected");
    // the ledger now records another store, and the disk still holds the one the merge-base records (W191 here)
    project.write("a.spec", SECOND);
    project.remove(LEDGER_FILE);
    stored(project.root(), "node", KEY, json!([two_tests()[0]]));
    repo.commit_all("collected again");
    project.write(STORE, &collected);
    let (compiled, runtime) = since(&project, &base);
    assert!(
        compiled.diagnostics().iter().any(|d| d.code == "W191"),
        "the working tree does not serve the store: {:?}",
        compiled.diagnostics()
    );
    let (nodes, delta) = sent(&runtime);
    assert!(nodes.is_empty(), "no fact is removed: {delta}");
    assert_eq!(delta["base"], json!({"kind": "revision", "id": base}));
    assert!(names(&compiled, "x", "modified"), "the specs still compare");
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_collector_served_in_the_working_tree_only_is_no_addition() {
    let (project, repo) = project();
    let base = repo.commit_all("no facts");
    project.write("a.spec", SECOND);
    stored(project.root(), "node", KEY, two_tests());
    repo.commit_all("collected");
    let (compiled, runtime) = since(&project, &base);
    assert_eq!(
        compiled
            .graph()
            .nodes()
            .iter()
            .filter(|n| n.kind.raw.as_str() == "test")
            .count(),
        2,
        "the working tree serves the store"
    );
    let (nodes, delta) = sent(&runtime);
    assert!(nodes.is_empty(), "no fact is added: {delta}");
    assert!(names(&compiled, "x", "modified"), "the specs still compare");
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_collector_served_on_both_sides_sends_what_its_facts_changed() {
    let (project, repo) = project();
    stored(project.root(), "node", KEY, two_tests());
    let base = repo.commit_all("collected");
    // a later collect on another key: the merge-base's store archived, one test stored
    project.write(ARCHIVED, project.read(STORE));
    project.remove(STORE);
    std::fs::remove_dir(project.root().join(format!(".specforge/facts/{KEY}"))).unwrap();
    project.remove(LEDGER_FILE);
    stored(project.root(), "node", NEXT, json!([two_tests()[0]]));
    repo.commit_all("collected again");
    let (_, runtime) = since(&project, &base);
    let (nodes, delta) = sent(&runtime);
    assert_eq!(
        nodes,
        [("js:a.test.js#subtracts".to_string(), "removed".to_string())],
        "{delta}"
    );
}
