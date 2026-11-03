//! The taskboard twin's tests as the thread's test facts: `@javascript/node-test` (a [`TWIN_BUILTINS`] builtin)
//! answers a `test` and a `test_run` fact per test of node's JUnit report, from the recorded golden or from a live
//! `node --test`, and both name the tests alike (R-1). A live run skips where node cannot write the JUnit report.
//!
//! [`TWIN_BUILTINS`]: super::harness::TWIN_BUILTINS

use super::harness::{Project, taskboard};
use super::node::node_test_runs;

/// The twin's three tests, in `test/tasks.test.js` of the package `taskboard`.
const TESTS: [&str; 3] = [
    "js:taskboard/test/tasks.test.js#a task is added with the next id",
    "js:taskboard/test/tasks.test.js#a task is completed by its id",
    "js:taskboard/test/tasks.test.js#completing an unknown id throws",
];

/// The `collect --format json` answer of `run`.
fn answer(run: &super::harness::Run) -> serde_json::Value {
    assert_eq!(run.code, 0, "collect: {}", run.all());
    serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("collect prints JSON ({e}): {}", run.all()))
}

/// `(id, status)` of every fact of `kind`, sorted.
fn facts(p: &Project, kind: &str) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = p
        .nodes_of(kind)
        .iter()
        .map(|n| {
            (
                n["id"].as_str().expect("an id").to_string(),
                n["fields"]["status"]
                    .as_str()
                    .expect("a status")
                    .to_string(),
            )
        })
        .collect();
    found.sort();
    found
}

/// `ids`, each with `status`.
fn all(ids: &[&str], status: &str) -> Vec<(String, String)> {
    ids.iter()
        .map(|id| (id.to_string(), status.to_string()))
        .collect()
}

/// `ids` as their runs, each with `status`.
fn runs(ids: &[&str], status: &str) -> Vec<(String, String)> {
    ids.iter()
        .map(|id| (format!("run:{id}"), status.to_string()))
        .collect()
}

#[test]
fn the_twins_junit_golden_collects_as_the_threads_test_facts() {
    let p = taskboard();
    let collected = answer(&p.collect_node_test_golden());

    let runner = &collected["runners"][0];
    assert_eq!(runner["name"], "node-test", "{collected:#}");
    assert_eq!(runner["ran"], false, "{collected:#}");
    let tests = facts(&p, "test");
    assert_eq!(
        tests[0].0,
        "js:taskboard/test/tasks.test.js#a task is added with the next id"
    );
    assert_eq!(tests, all(&TESTS, "passed"));
    assert_eq!(facts(&p, "test_run"), runs(&TESTS, "passed"));
}

#[test]
fn node_test_runs_the_twins_own_tests() {
    if !node_test_runs() {
        eprintln!("node --test with the JUnit reporter does not run here: nothing to check");
        return;
    }
    let p = taskboard();
    let collected = answer(&p.collect(&["--runner", "node-test"]));

    let runner = &collected["runners"][0];
    assert_eq!(
        (&runner["ran"], &runner["exit_code"]),
        (&true.into(), &0.into()),
        "{collected:#}"
    );
    assert!(p.exists(".specforge/reports/node-test.xml"));
    assert_eq!(
        facts(&p, "test"),
        all(&TESTS, "passed"),
        "the live run names the tests as the golden does"
    );
    assert_eq!(facts(&p, "test_run"), runs(&TESTS, "passed"));
}

#[test]
fn a_failing_twin_test_exits_1_and_is_still_recorded() {
    if !node_test_runs() {
        eprintln!("node --test with the JUnit reporter does not run here: nothing to check");
        return;
    }
    let p = taskboard();
    p.write(
        "test/broken.test.js",
        "import { test } from 'node:test';\nimport assert from 'node:assert/strict';\n\ntest('one is two', () => {\n  assert.equal(1, 2);\n});\n",
    );
    let collected = answer(&p.collect(&["--runner", "node-test"]));

    assert_eq!(collected["runners"][0]["exit_code"], 1, "{collected:#}");
    let broken = "js:taskboard/test/broken.test.js#one is two";
    let mut expected = all(&[broken], "failed");
    expected.extend(all(&TESTS, "passed"));
    assert_eq!(facts(&p, "test"), expected);
    let mut expected = runs(&[broken], "failed");
    expected.extend(runs(&TESTS, "passed"));
    assert_eq!(facts(&p, "test_run"), expected);
}
