//! What a pass and a validation rule are sent of the facts a session serves (ADR 0353). The fixture is `Collected`
//! (two `test` facts of `@t/node`'s store, a truth citing one) beside `@t/probe`: the collected kind `note` with one
//! stored fact, a check pass `plain` that declares no input, a check pass `scoped` that declares the facts of `test`,
//! and a custom rule that names no kind, answered by `validate__none` (no violation). The payload is counted from the
//! inputs the runtime recorded, never timed.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;
use specforge_project::testing::{COLLECTED_EXTENSIONS, Collected, TestProject, stored};
use specforge_test::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

use super::{KEY, two_tests};

const PROBE: &str = "@t/probe";
const TESTS: [&str; 2] = ["js:a.test.js#adds", "js:a.test.js#subtracts"];
const NOTE: &str = "note:one";

/// `@t/probe`: the collected kind `note`, the check passes `plain` (no input) and `scoped` (the facts of `test`), and
/// the kind-less custom rule `E990` naming `validate__none`.
fn probe() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(PROBE, "1.0.0"));
    c.kind("note", |k| {
        k.collected(&["note"]);
    });
    c.pass("plain", |p| {
        p.phase("check").run(|_| Vec::new());
    });
    c.pass("scoped", |p| {
        p.phase("check")
            .input(Input::facts(&["test"]))
            .run(|_| Vec::new());
    });
    specforge_project::testing::declare_rules(
        &mut c,
        json!([{
            "code": "E990", "severity": "error", "check": "custom",
            "message_template": "'{id}' failed", "wasm_function": "validate__none"
        }]),
    );
    c
}

/// `validate__none`: every entity passes.
fn none(export: &str, input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    (export == "validate__none").then(|| {
        specforge_extension_sdk::answer_export(export, input, |_: &ValidatorContext| {
            ValidatorVerdict::Pass
        })
    })
}

/// The runtime the probed project compiled in, after its compile: the two tests and the note stored.
fn compiled_probe() -> Arc<InProcessRuntime> {
    let mut extensions = COLLECTED_EXTENSIONS.to_vec();
    extensions.push(PROBE);
    let p = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&extensions)
        .installing_enabled()
        .file(
            "truths.spec",
            "truth T1 \"One\" {\n  proof [\"js:a.test.js#adds\"]\n}\n",
        );
    stored(p.root(), "node", KEY, two_tests());
    stored(
        p.root(),
        "unit",
        KEY,
        json!([{"kind": "note", "id": NOTE, "fields": {}}]),
    );
    let runtime =
        Arc::new(Collected::new().over(InProcessRuntime::new().with_handler(probe, none)));
    let compiled = p.compile_in(runtime.clone());
    assert!(
        compiled.graph().nodes().iter().any(|n| n.id.raw == NOTE),
        "the note is served: {:?}",
        compiled.diagnostics()
    );
    runtime
}

/// The ids `ids` holds that are facts, sorted.
fn facts_of<'a>(ids: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    let mut found: Vec<&str> = ids
        .into_iter()
        .filter(|id| TESTS.contains(id) || *id == NOTE)
        .collect();
    found.sort();
    found
}

/// The ids of the entities the last call of `pass` was sent, and its edges as `source -> target`.
fn sent(runtime: &InProcessRuntime, pass: &str) -> (Vec<String>, Vec<String>) {
    let input = specforge_project::testing::pass_input(runtime, pass);
    let ids = input["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect();
    let edges = input["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            format!(
                "{} -> {}",
                e["source"].as_str().unwrap(),
                e["target"].as_str().unwrap()
            )
        })
        .collect();
    (ids, edges)
}

#[specforge_test(
    behavior = "run_check_phase_passes",
    verify = "a pass that declares no facts input is sent no fact"
)]
fn a_pass_that_declares_no_facts_is_sent_no_fact() {
    let runtime = compiled_probe();
    let (ids, edges) = sent(&runtime, "plain");
    assert!(ids.iter().any(|id| id == "T1"), "{ids:?}");
    assert_eq!(facts_of(ids.iter().map(String::as_str)), Vec::<&str>::new());
    assert!(
        edges
            .iter()
            .all(|e| !e.contains(".test.js") && !e.contains(NOTE)),
        "{edges:?}"
    );
}

#[specforge_test(
    behavior = "run_check_phase_passes",
    verify = "a pass is sent exactly the facts of the kinds it declares"
)]
fn a_pass_is_sent_exactly_the_facts_of_the_kinds_it_declares() {
    let runtime = compiled_probe();
    let (ids, edges) = sent(&runtime, "scoped");
    assert!(ids.iter().any(|id| id == "T1"), "{ids:?}");
    assert_eq!(facts_of(ids.iter().map(String::as_str)), TESTS);
    assert!(
        edges.contains(&"T1 -> js:a.test.js#adds".to_string()),
        "the proof edge, both ends sent: {edges:?}"
    );
    let (plain, _) = sent(&runtime, "plain");
    assert_eq!(ids.len(), plain.len() + TESTS.len());
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "a validation rule that names no kind judges no fact"
)]
fn a_kind_less_custom_rule_is_called_on_no_fact() {
    let runtime = compiled_probe();
    let calls: Vec<Value> = runtime
        .calls()
        .into_iter()
        .filter(|call| call.export == "validate__none")
        .map(|call| call.input)
        .collect();
    let ids: Vec<&str> = calls
        .iter()
        .map(|c| c["entity"]["id"].as_str().unwrap())
        .collect();
    assert_eq!(facts_of(ids.iter().copied()), Vec::<&str>::new());
    assert!(ids.contains(&"T1"), "the authored truth is judged: {ids:?}");
}
