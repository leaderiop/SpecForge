//! The `checks` collector and its `check_outcome` facts: the declaration, and the rows mapped one by one.

use specforge_protocol_types::{
    CheckOutcome, CheckStatus, CollectInput, Input, OutcomeClass, Unreachable,
};
use specforge_test::prelude::*;

use super::declaration;
use crate::facts::{collect, rfc3339};

#[specforge_test(
    behavior = "ck_outcome_facts",
    verify = "the checks collector declares the check_outcomes input and no detection, command or report"
)]
fn declares_the_checks_collector_with_no_detection_and_the_check_outcomes_input() {
    let declaration = declaration();
    let collector = declaration
        .collectors
        .iter()
        .find(|c| c.name == "checks")
        .expect("the checks collector");
    assert_eq!(collector.inputs, [Input::CheckOutcomes]);
    assert!(collector.auto_detect.is_none(), "{collector:?}");
    assert!(collector.capability.is_none() && collector.report.is_none());
    assert!(!collector.writes);
}

#[test]
fn check_outcome_is_collected_in_scheme_check_with_status_as_its_result() {
    let declaration = declaration();
    let kind = declaration
        .entities
        .iter()
        .find(|k| k.name == "check_outcome")
        .expect("the check_outcome kind");
    assert_eq!(kind.writer.as_deref(), Some("collector"));
    assert_eq!(kind.schemes, ["check"]);
    assert_eq!(kind.result.as_deref(), Some("status"));
    assert!(kind.complete_when.is_none());
}

fn row(
    item: u32,
    status: CheckStatus,
    outcome: Option<OutcomeClass>,
    codes: &[&str],
) -> CheckOutcome {
    CheckOutcome {
        entity: "f".into(),
        field: "refs".into(),
        item,
        value: "https://a.example/".into(),
        checker: "@specforge/checks:url".into(),
        status,
        outcome,
        observed_at: Some(1_767_225_600_000),
        from_cache: false,
        codes: codes.iter().map(ToString::to_string).collect(),
    }
}

#[specforge_test(
    behavior = "ck_outcome_facts",
    verify = "each served row becomes one check_outcome fact, a finding failed and a check not performed skipped"
)]
fn each_outcome_row_becomes_one_fact_and_a_finding_is_failed() {
    let input: CollectInput = CollectInput {
        check_outcomes: vec![
            row(0, CheckStatus::Passed, Some(OutcomeClass::Ok), &[]),
            row(
                1,
                CheckStatus::Failed,
                Some(OutcomeClass::Status(404)),
                &["E093"],
            ),
            row(2, CheckStatus::NotRun, None, &[]),
        ],
        ..CollectInput::default()
    };
    let facts = collect(&input).unwrap().facts;
    assert_eq!(facts.len(), 3);
    let ids: Vec<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "check:f/refs#0@@specforge/checks:url",
            "check:f/refs#1@@specforge/checks:url",
            "check:f/refs#2@@specforge/checks:url"
        ]
    );
    assert!(facts.iter().all(|f| f.kind == "check_outcome"));
    let field = |at: usize, name: &str| facts[at].fields.get(name).cloned();
    assert_eq!(field(0, "status"), Some("passed".into()));
    assert_eq!(field(1, "status"), Some("failed".into()));
    assert_eq!(field(1, "outcome"), Some("status:404".into()));
    assert_eq!(field(1, "codes"), Some(serde_json::json!(["E093"])));
    assert_eq!(field(1, "observed_at"), Some("2026-01-01T00:00:00Z".into()));
    assert_eq!(field(2, "status"), Some("skipped".into()));
    assert_eq!(field(2, "outcome"), None);
    assert_eq!(field(2, "item"), Some(2.into()));

    let unreachable = row(
        0,
        CheckStatus::Failed,
        Some(OutcomeClass::Unreachable(Unreachable::Dns)),
        &["W172"],
    );
    let one = crate::facts::fact(&unreachable);
    assert_eq!(one.fields.get("outcome"), Some(&"unreachable:dns".into()));
}

#[test]
fn facts_rfc3339_reads_unix_milliseconds_in_utc() {
    assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
    assert_eq!(rfc3339(951_782_400_000), "2000-02-29T00:00:00Z");
    assert_eq!(rfc3339(1_767_225_599_999), "2025-12-31T23:59:59Z");
}
