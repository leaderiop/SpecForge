use serde_json::json;

use super::*;
use crate::passes::thread::fixture::add_test;

const TR: &str = "v1.0/01/01-01/TR1";

/// Test facts `(name, status, whether it names the truth)`.
type Facts = Vec<(&'static str, TestStatus, bool)>;

fn input(verification: &str, cited: &[&str], facts: Option<Facts>) -> PassInput {
    let judgement = json!({ "rule": "gsd/E501", "state": "fresh", "passed": true, "outcome": "verified",
                            "cited_tests": cited });
    let value = json!({
        "entities": [
            { "id": TR, "kind": "truth", "fields": { "verification": verification }, "judgements": [judgement] },
        ],
        "edges": [],
    });
    let mut input: PassInput =
        serde_json::from_value(crate::passes::plans::with_field_edges(value))
            .expect("a pass input");
    for (name, status, names) in facts.into_iter().flatten() {
        let named: &[&str] = if names { &[TR] } else { &[] };
        let id = format!("js:taskboard/test/tasks.test.js#{name}");
        add_test(&mut input, &id, name, "test/tasks.test.js", status, named);
    }
    input
}

/// Facts: `linked` name the truth, `others` name nothing.
fn report(linked: &[(&'static str, TestStatus)], others: &[(&'static str, TestStatus)]) -> Facts {
    linked
        .iter()
        .map(|(name, status)| (*name, *status, true))
        .chain(others.iter().map(|(name, status)| (*name, *status, false)))
        .collect()
}

#[test]
fn a_cited_failing_test_contradicts_verified() {
    let found = e502(&input(
        "explicit",
        &["cited_one"],
        Some(report(
            &[("linked", TestStatus::Passed)],
            &[("cited_one", TestStatus::Failed)],
        )),
    ));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, "E502");
    assert!(found[0].message.contains("a test recorded for it failed"));
}

#[test]
fn a_passing_cited_test_backs_the_verdict_and_an_uncited_failure_is_not_counted() {
    let passing = report(
        &[],
        &[
            ("cited_one", TestStatus::Passed),
            ("stray", TestStatus::Failed),
        ],
    );
    assert!(e502(&input("explicit", &["cited_one"], Some(passing))).is_empty());
}

#[test]
fn a_backstop_or_unreported_truth_is_never_contradicted() {
    let failing = report(&[("linked", TestStatus::Failed)], &[]);
    assert!(e502(&input("backstop", &[], Some(failing))).is_empty());
    assert!(e502(&input("explicit", &[], None)).is_empty());
}

/// A truth with no `verification` names no tier to contradict. A lifted backstop edge is not one: the planner writes
/// it `verification backstop`, as above.
#[test]
fn an_untiered_truth_is_never_contradicted() {
    let failing = report(&[("linked", TestStatus::Failed)], &[]);
    let mut untiered = input("explicit", &[], Some(failing));
    untiered.entities[0].fields.remove("verification");
    assert!(e502(&untiered).is_empty());
}

#[test]
fn a_report_with_no_result_is_no_run_for_e502() {
    // What `collect --runner gsd-git` leaves: no test fact.
    assert!(e502(&input("explicit", &[], Some(Vec::new()))).is_empty());
    // A run that links nothing to the truth is "no passing test".
    let other = report(&[], &[("stray", TestStatus::Passed)]);
    let found = e502(&input("explicit", &[], Some(other)));
    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .message
            .contains("no passing test is recorded for it")
    );
}

/// E502 reads the facts that name the truth and those its verdict cites (ADR 0371 D5): a failed one that names it
/// contradicts the verdict, and a fact a `proof` edge points at, which no truth writes now, plays no part.
#[test]
fn a_failed_fact_that_names_the_truth_contradicts_it_and_a_proof_edge_plays_no_part() {
    let found = e502(&input(
        "explicit",
        &[],
        Some(report(&[("linked", TestStatus::Failed)], &[])),
    ));
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("a test recorded for it failed"));
    assert_eq!(
        found[0].suggestion.as_deref(),
        Some("record `failed` for the truth, or tag a passing test with it ([gsd:<id>])")
    );
    let mut with_edge = input(
        "explicit",
        &[],
        Some(report(
            &[("linked", TestStatus::Passed)],
            &[("adds", TestStatus::Failed)],
        )),
    );
    with_edge.edges.push(PassEdge {
        source: TR.to_string(),
        target: "js:taskboard/test/tasks.test.js#adds".to_string(),
        label: "proof".to_string(),
    });
    assert!(e502(&with_edge).is_empty());
}
