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

#[test]
fn a_failing_proof_contradicts_a_verified_explicit_truth() {
    let fact = "js:taskboard/test/tasks.test.js#adds";
    let with_proof = |status: &str| {
        // The proof's fact is the test run (`collect --runner node-test`).
        let mut proved = input("explicit", &[], None);
        proved.entities.push(
            serde_json::from_value(
                json!({ "id": fact, "kind": "test", "fields": { "name": "adds", "status": status } }),
            )
            .expect("a test fact"),
        );
        proved.edges.push(PassEdge {
            source: TR.to_string(),
            target: fact.to_string(),
            label: "proof".to_string(),
        });
        e502(&proved)
    };
    let found = with_proof("failed");
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("a test recorded for it failed"));
    assert!(with_proof("passed").is_empty());
}
