use serde_json::{Value, json};

use super::*;

const TR: &str = "v1.0/01/01-01/TR1";

fn input(verification: &str, cited: &[&str], report: Option<Value>) -> PassInput {
    let judgement = json!({ "rule": "gsd/E501", "state": "fresh", "passed": true, "outcome": "verified",
                            "cited_tests": cited });
    let mut value = json!({
        "entities": [
            { "id": TR, "kind": "truth", "fields": { "verification": verification }, "judgements": [judgement] },
        ],
        "edges": [],
    });
    if let Some(report) = report {
        value["test_results"] = report;
    }
    serde_json::from_value(crate::passes::plans::with_field_edges(value)).expect("a pass input")
}

fn report(linked: &[(&str, &str)], others: &[(&str, &str)]) -> Value {
    let tests = |rows: &[(&str, &str)]| {
        rows.iter()
            .map(|(name, status)| json!({ "name": name, "status": status }))
            .collect::<Vec<_>>()
    };
    json!({ "results": { TR: { "tests": tests(linked) }, "other": { "tests": tests(others) } } })
}

#[test]
fn a_cited_failing_test_contradicts_verified() {
    let found = e502(&input(
        "explicit",
        &["cited_one"],
        Some(report(&[("linked", "passed")], &[("cited_one", "failed")])),
    ));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, "E502");
    assert!(found[0].message.contains("a test recorded for it failed"));
}

#[test]
fn a_passing_cited_test_backs_the_verdict_and_an_uncited_failure_is_not_counted() {
    let passing = report(&[], &[("cited_one", "passed"), ("stray", "failed")]);
    assert!(e502(&input("explicit", &["cited_one"], Some(passing))).is_empty());
}

#[test]
fn a_backstop_or_unreported_truth_is_never_contradicted() {
    let failing = report(&[("linked", "failed")], &[]);
    assert!(e502(&input("backstop", &[], Some(failing))).is_empty());
    assert!(e502(&input("explicit", &[], None)).is_empty());
}
