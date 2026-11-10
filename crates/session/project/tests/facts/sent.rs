//! What a pass and a validation rule are sent of the facts a session serves. The fixture is `Collected` (two `test`
//! facts of `@t/node`'s store, a truth citing one) beside `@t/probe`: a check pass `plain` that declares no input, and
//! a custom rule that names no kind, answered by `validate__none` (no violation). The payload is counted from the
//! inputs the runtime recorded, never timed.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;
use specforge_project::testing::{COLLECTED_EXTENSIONS, Collected, TestProject, stored};
use specforge_wasm::testing::InProcessRuntime;

use super::{KEY, two_tests};

const PROBE: &str = "@t/probe";
const FACTS: [&str; 2] = ["js:a.test.js#adds", "js:a.test.js#subtracts"];

/// `@t/probe`: the check pass `plain` (no input) and the kind-less custom rule `E990` naming `validate__none`.
fn probe() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(PROBE, "1.0.0"));
    c.pass("plain", |p| {
        p.phase("check").run(|_| Vec::new());
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

/// The runtime the probed project compiled in, after its compile.
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
    let runtime =
        Arc::new(Collected::new().over(InProcessRuntime::new().with_handler(probe, none)));
    p.compile_in(runtime.clone());
    runtime
}

/// The ids of `entities` (a JSON list of pass or validator entities) that are facts.
fn facts_of<'a>(ids: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    let mut found: Vec<&str> = ids.into_iter().filter(|id| FACTS.contains(id)).collect();
    found.sort();
    found
}

#[test]
fn every_pass_is_sent_every_fact() {
    let runtime = compiled_probe();
    let input = specforge_project::testing::pass_input(&runtime, "plain");
    let ids = input["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap());
    assert_eq!(facts_of(ids), FACTS);
}

#[test]
fn a_kind_less_custom_rule_is_called_on_every_fact() {
    let runtime = compiled_probe();
    let calls: Vec<Value> = runtime
        .calls()
        .into_iter()
        .filter(|call| call.export == "validate__none")
        .map(|call| call.input)
        .collect();
    let ids = calls.iter().map(|c| c["entity"]["id"].as_str().unwrap());
    assert_eq!(facts_of(ids), FACTS);
    assert!(
        calls.iter().any(|c| c["entity"]["id"] == "T1"),
        "the authored truth is judged too"
    );
}
