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
