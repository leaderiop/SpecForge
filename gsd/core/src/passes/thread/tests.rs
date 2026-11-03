use serde_json::{Value, json};

use super::*;

fn input(report: Option<Value>) -> PassInput {
    let mut value = json!({ "entities": [] });
    if let Some(report) = report {
        value["test_results"] = report;
    }
    serde_json::from_value(value).expect("a pass input")
}

fn tests(rows: &[(&str, &str)]) -> Value {
    json!({ "tests": rows.iter().map(|(name, status)| json!({ "name": name, "status": status })).collect::<Vec<_>>() })
}

#[test]
fn no_report_is_not_run() {
    let input = input(None);
    let thread = Thread::of(&input);
    assert!(!thread.ran());
    assert_eq!(thread.of_entity("T1"), Evidence::NotRun);
}

#[test]
fn a_report_with_no_result_and_no_test_fact_is_not_run() {
    let input = input(Some(json!({ "results": { "T1": { "tests": [] } } })));
    let thread = Thread::of(&input);
    assert!(!thread.ran());
    assert_eq!(thread.of_entity("T1"), Evidence::NotRun);
}

#[test]
fn a_run_with_nothing_linked_is_no_test() {
    let input = input(Some(
        json!({ "results": { "T2": tests(&[("adds", "passed")]) } }),
    ));
    let thread = Thread::of(&input);
    assert!(thread.ran());
    assert_eq!(thread.of_entity("T1"), Evidence::NoTest);
    // A skipped test is no evidence either way.
    let skipped = self::input(Some(
        json!({ "results": { "T1": tests(&[("adds", "skipped")]) } }),
    ));
    assert_eq!(Thread::of(&skipped).of_entity("T1"), Evidence::NoTest);
}

#[test]
fn a_failed_result_outweighs_a_passed_one() {
    let input = input(Some(
        json!({ "results": { "T1": tests(&[("adds", "passed"), ("lists", "failed")]) } }),
    ));
    let thread = Thread::of(&input);
    assert_eq!(thread.of_entity("T1"), Evidence::Failed);
    assert_eq!(
        thread.linked("T1"),
        [("adds", TestStatus::Passed), ("lists", TestStatus::Failed)]
    );
}

#[test]
fn cited_results_count_anywhere_in_the_report() {
    let input = input(Some(json!({ "results": {
        "T1": tests(&[("adds", "passed")]),
        "T2": tests(&[("lists", "failed"), ("other", "passed")]),
    } })));
    let thread = Thread::of(&input);
    assert_eq!(
        thread.cited(&["lists".to_string(), "adds".to_string()]),
        [TestStatus::Passed, TestStatus::Failed]
    );
    assert_eq!(
        thread.evidence(thread.cited(&["lists".to_string()])),
        Evidence::Failed
    );
}

#[test]
fn a_checks_ref_names_its_results() {
    let input = input(Some(json!({ "results": {
        "D1": tests(&[("test/a.test.js#adds", "passed"), ("test/a.test.js#lists", "failed")]),
    } })));
    let thread = Thread::of(&input);
    assert_eq!(
        thread.named("D1", "test/a.test.js#adds"),
        [TestStatus::Passed]
    );
    assert_eq!(
        thread.evidence(thread.named("D1", "test/a.test.js#gone")),
        Evidence::NoTest
    );
}

/// `input` holding `facts` as `@code/core` test facts `(id, name, file, status)` and `proofs` as `(truth, fact)`
/// `proof` edges.
fn with_facts(
    report: Option<Value>,
    facts: &[(&str, &str, &str, &str)],
    proofs: &[(&str, &str)],
) -> PassInput {
    let mut input = input(report);
    for (id, name, file, status) in facts {
        input.entities.push(
            serde_json::from_value(json!({
                "id": id, "kind": "test",
                "fields": { "name": name, "file": file, "status": status },
            }))
            .expect("a test fact"),
        );
    }
    for (truth, fact) in proofs {
        input.edges.push(PassEdge {
            source: truth.to_string(),
            target: fact.to_string(),
            label: "proof".to_string(),
        });
    }
    input
}

const ADDS: &str = "js:taskboard/test/tasks.test.js#adds";
const LISTS: &str = "js:taskboard/test/tasks.test.js#lists";

#[test]
fn a_truths_proof_results_are_linked_to_it() {
    let input = with_facts(
        Some(json!({ "results": { "TR1": tests(&[("tagged", "passed")]) } })),
        &[
            (ADDS, "adds", "test/tasks.test.js", "failed"),
            (LISTS, "lists", "test/tasks.test.js", "passed"),
        ],
        &[("TR1", ADDS)],
    );
    let thread = Thread::of(&input);
    assert_eq!(
        thread.linked("TR1"),
        [("tagged", TestStatus::Passed), (ADDS, TestStatus::Failed)]
    );
    assert_eq!(thread.proof("TR1"), [(ADDS, TestStatus::Failed)]);
    assert_eq!(thread.of_entity("TR1"), Evidence::Failed);
}

#[test]
fn a_test_fact_is_a_test_run() {
    let input = with_facts(None, &[(ADDS, "adds", "test/tasks.test.js", "passed")], &[]);
    let thread = Thread::of(&input);
    assert!(thread.ran());
    assert_eq!(thread.of_entity("TR1"), Evidence::NoTest);
}

#[test]
fn a_checks_ref_names_a_fact_by_its_name_and_file() {
    let input = with_facts(
        None,
        &[
            (ADDS, "adds", "test/tasks.test.js", "passed"),
            (
                "js:other/test/other.test.js#adds",
                "adds",
                "test/other.test.js",
                "failed",
            ),
        ],
        &[],
    );
    let thread = Thread::of(&input);
    assert_eq!(
        thread.named("D1", "test/tasks.test.js#adds"),
        [TestStatus::Passed]
    );
    assert_eq!(thread.named("D1", "test/missing.test.js#adds"), []);
    // A ref with no file names the test in every file.
    assert_eq!(
        thread.named("D1", "adds"),
        [TestStatus::Passed, TestStatus::Failed]
    );
}
