//! A judged `test(..)` input over a proof (ADR 0331 D7): `test(self, self.proof)` reads the truth's report results and
//! its proof's `test` facts, each chain on its own; nothing linked is the value `none`; a proof no store holds facts of
//! is not read, and one a store lost is `absent`; the input is unjudged only when no chain can be read.

use serde_json::{Value, json};
use specforge_project::CompiledProject;
use specforge_project::judged::{Cited, Inputs};
use specforge_project::testing::{
    TestProject, gsd_judged_project, gsd_judged_tests_project, stored,
};
use specforge_registry::PlainDisk;
use specforge_test::prelude::*;

use crate::judged::{compile, judgement, record};

const E501: &str = "gsd/E501";
/// A truth whose proof cites one test.
const PROVED: &str = "v1.0/03/03-02/TR2";
const ADDS: &str = "js:a.test.js#adds";
const KEY: &str = "0123456789abcdef0123456789abcdef01234567";
const STORE: &str = ".specforge/facts/0123456789abcdef0123456789abcdef01234567/node.json";

fn proved() -> TestProject {
    gsd_judged_tests_project(&format!(
        "\ntruth TR2 \"Proved\" {{\n  plan 03-02\n  proof [\"{ADDS}\"]\n}}\n"
    ))
}

/// The store holding `tests` as `(name, status)` facts of `a.test.js`.
fn store(p: &TestProject, tests: &[(&str, &str)]) {
    let facts: Vec<Value> = tests
        .iter()
        .map(|(name, status)| {
            json!({"kind": "test", "id": format!("js:a.test.js#{name}"),
                   "fields": {"file": "a.test.js", "status": status}})
        })
        .collect();
    stored(p.root(), "node", KEY, Value::Array(facts));
}

/// A report holding `tests` for `entity`.
fn report(p: &TestProject, entity: &str, tests: &[(&str, &str)]) {
    let tests: Vec<Value> = tests
        .iter()
        .map(|(name, status)| json!({"name": name, "status": status, "duration_ms": 1}))
        .collect();
    p.write(
        "specforge-report.json",
        json!({"results": {entity: {"tests": tests}}}).to_string(),
    );
}

fn inputs(compiled: &CompiledProject, entity: &str) -> Inputs {
    let rule = compiled
        .environment()
        .registries()
        .judged_rule(E501)
        .expect("E501 is registered");
    specforge_project::judged::inputs(compiled, &PlainDisk, entity, rule, &Cited::default())
        .expect("E501 judges a truth")
}

fn tests_input(compiled: &CompiledProject, entity: &str) -> (Vec<String>, String) {
    let now = inputs(compiled, entity);
    let report = now
        .reports
        .iter()
        .find(|r| r.name == "tests")
        .expect("a tests input");
    (report.members.clone(), report.digest.clone())
}

fn codes(compiled: &CompiledProject) -> Vec<String> {
    compiled
        .diagnostics()
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a test input of several chains digests their union"
)]
fn a_test_input_of_several_chains_digests_their_union() {
    let p = proved();
    store(&p, &[("adds", "passed"), ("lists", "passed")]);
    report(&p, PROVED, &[("survives", "passed")]);
    let compiled = compile(&p);
    let (members, digest) = tests_input(&compiled, PROVED);
    assert_eq!(members, [ADDS, PROVED]);
    assert!(digest.starts_with("sha256:"), "{digest}");
    assert!(inputs(&compiled, PROVED).unjudged.is_empty());
    // Each chain's results count: the tagged one's change stales the verdict as the fact's does (below).
    record(&p, PROVED, E501, "passed");
    report(&p, PROVED, &[("survives", "failed")]);
    assert_eq!(judgement(&compile(&p), PROVED, E501).1, ["tests"]);
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a fact's own result is its test input, and a status change stales it"
)]
fn with_no_report_a_failing_proof_stales_a_verified_truth() {
    // A facts-only collect writes no report: the truth's own chain is not read, its proof's is.
    let p = proved();
    store(&p, &[("adds", "passed")]);
    record(&p, PROVED, E501, "passed");
    let compiled = compile(&p);
    assert!(inputs(&compiled, PROVED).unjudged.is_empty());
    assert_eq!(judgement(&compiled, PROVED, E501).0, "fresh");
    store(&p, &[("adds", "failed")]);
    let (state, changed) = judgement(&compile(&p), PROVED, E501);
    assert_eq!(
        (state.as_str(), changed.as_slice()),
        ("stale", ["tests".to_string()].as_slice())
    );
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "each chain of a test input counts on its own, and the input is unjudged only when no chain is read"
)]
fn with_no_report_a_passing_proof_keeps_it_fresh() {
    let p = proved();
    store(&p, &[("adds", "passed")]);
    record(&p, PROVED, E501, "passed");
    // Another collect stores the same result, beside a test the proof does not cite.
    store(&p, &[("adds", "passed"), ("lists", "failed")]);
    let compiled = compile(&p);
    assert_eq!(judgement(&compiled, PROVED, E501).0, "fresh");
    assert!(!codes(&compiled).contains(&"I205".to_string()));
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a proof with no stored facts is unjudged, as a missing report is"
)]
fn with_no_report_and_no_stored_facts_every_chain_is_unjudged_and_i205() {
    let p = proved();
    store(&p, &[("adds", "passed")]);
    record(&p, PROVED, E501, "passed");
    // A fresh clone: the store is git-ignored, so it is not there.
    p.remove(STORE);
    let compiled = compile(&p);
    assert_eq!(inputs(&compiled, PROVED).unjudged, ["tests"]);
    assert_eq!(judgement(&compiled, PROVED, E501).0, "fresh");
    let i205: Vec<_> = compiled
        .diagnostics()
        .iter()
        .filter(|d| d.code == "I205")
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(i205.len(), 1, "{i205:?}");
    assert!(
        i205[0].contains("no collected facts for the references they read"),
        "{i205:?}"
    );
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a proof the store does not hold leaves the input unjudged, and one the store lost stales it"
)]
fn a_proof_no_store_holds_is_unjudged_and_one_the_store_lost_is_absent() {
    let p = proved();
    // No store holds `js` facts: the cited test may simply not be collected yet.
    assert_eq!(inputs(&compile(&p), PROVED).unjudged, ["tests"]);
    store(&p, &[("adds", "passed")]);
    record(&p, PROVED, E501, "passed");
    // The store holds `js` facts, but not the cited one any more: the test is gone.
    store(&p, &[("lists", "passed")]);
    let compiled = compile(&p);
    assert!(inputs(&compiled, PROVED).unjudged.is_empty());
    assert_eq!(tests_input(&compiled, PROVED).0, [ADDS, PROVED]);
    assert_eq!(judgement(&compiled, PROVED, E501).1, ["tests"]);
}

#[specforge_test(
    behavior = "declare_judged_rule",
    verify = "a chain to a kind of an unloaded optional peer is inert, not W112"
)]
fn a_chain_to_a_kind_of_an_unloaded_optional_peer_is_inert_not_w112() {
    // `@gsd/core`'s E501 reads `test(self, self.proof)`, and `proof` names `test`, which the builtin vocabulary
    // credits to `@code/core`, an optional peer the project does not load.
    let p = gsd_judged_project("");
    let compiled = compile(&p);
    assert!(!codes(&compiled).contains(&"W112".to_string()));
    assert_eq!(judgement(&compiled, "v1.0/03/03-02/TR1", E501).0, "missing");
    report(&p, "v1.0/03/03-02/TR1", &[("survives", "passed")]);
    let (members, digest) = tests_input(&compile(&p), "v1.0/03/03-02/TR1");
    assert_eq!(members, ["v1.0/03/03-02/TR1"]);
    assert!(digest.starts_with("sha256:"), "{digest}");
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a linked test that vanishes stales the verdict"
)]
fn a_linked_test_that_vanishes_stales_the_verdict() {
    let p = gsd_judged_project("");
    let truth = "v1.0/03/03-02/TR1";
    report(&p, truth, &[("survives", "passed")]);
    record(&p, truth, E501, "passed");
    assert_eq!(judgement(&compile(&p), truth, E501).0, "fresh");
    // The run no longer links a test to the truth: `none` now, judged, against the results it was recorded over.
    report(&p, "v1.0/03/03-02", &[("plans", "passed")]);
    let (state, changed) = judgement(&compile(&p), truth, E501);
    assert_eq!(
        (state.as_str(), changed.as_slice()),
        ("stale", ["tests".to_string()].as_slice())
    );
}
