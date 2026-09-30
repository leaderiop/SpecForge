use super::*;
use serde_json::{Value, json};
use specforge_test_macros::test as specforge_test;

/// A case's input, in the `coverage` pass's input shape (the SDK's
/// `PassInput`), so the extension's native tests read the same file.
#[derive(Deserialize)]
struct PassInput {
    entities: Vec<PassEntity>,
    #[serde(default)]
    test_results: Option<PassTestResults>,
    #[serde(default)]
    proved_claims: Option<BTreeSet<String>>,
}

#[derive(Deserialize)]
struct PassEntity {
    id: String,
    kind: String,
    #[serde(default)]
    fields: BTreeMap<String, String>,
    #[serde(default)]
    incoming_edge_count: usize,
    #[serde(default)]
    testable: bool,
    #[serde(default)]
    verify_kinds: Vec<String>,
    #[serde(default)]
    verify_texts: Vec<String>,
}

#[derive(Deserialize)]
struct PassTestResults {
    #[serde(default)]
    runner: Option<String>,
    #[serde(default)]
    results: BTreeMap<String, PassEntityResults>,
}

#[derive(Deserialize)]
struct PassEntityResults {
    #[serde(default)]
    tests: Vec<RecordedTest>,
}

fn assess_input(input: Value) -> (Vec<Entity>, Assessment) {
    let input: PassInput = serde_json::from_value(input).unwrap();
    let entities: Vec<Entity> = input
        .entities
        .into_iter()
        .map(|e| Entity {
            risk: e.fields.get("risk").cloned(),
            referenced: e.incoming_edge_count > 0,
            id: e.id,
            kind: e.kind,
            testable: e.testable,
            verify_kinds: e.verify_kinds,
            verify_texts: e.verify_texts,
        })
        .collect();
    let results = input.test_results.map(|r| TestResults {
        runner: r.runner,
        entities: r.results.into_iter().map(|(id, e)| (id, e.tests)).collect(),
    });
    let assessment = assess(&entities, results.as_ref(), input.proved_claims.as_ref());
    (entities, assessment)
}

fn codes(assessment: &Assessment) -> Vec<&str> {
    assessment.findings.iter().map(|f| f.code).collect()
}

fn create_user(tests: Value) -> Value {
    json!({
        "entities": [{
            "id": "create_user", "kind": "behavior", "testable": true,
            "verify_kinds": ["unit", "unit"],
            "verify_texts": ["rejects a duplicate email", "stores a hashed password"]
        }],
        "test_results": {"runner": "cargo-test", "results": {"create_user": {"tests": tests}}}
    })
}

#[specforge_test(
    behavior = "te_coverage_pass",
    verify = "recorded test results prove entities and failing tests are A014"
)]
fn an_obligation_is_proven_by_a_passing_test_that_names_it() {
    let (_, out) = assess_input(create_user(json!([
        {"name": "dup", "status": "pass", "verify": "rejects a duplicate email"},
        {"name": "hash", "status": "pass", "verify": "stores a hashed password"}
    ])));
    assert!(out.findings.is_empty(), "{:?}", codes(&out));
    assert_eq!(out.summary.discharge_funnel.entities_proven, 1);
    assert_eq!(out.summary.test_results.unwrap().obligations_proven, 2);
    assert!(out.verdicts["create_user"].is_proven());
}

#[specforge_test(
    behavior = "te_coverage_pass",
    verify = "recorded test results prove entities and failing tests are A014"
)]
fn unnamed_and_failing_tests_leave_obligations_unproven() {
    let (_, out) = assess_input(create_user(json!([
        {"name": "any", "status": "pass"},
        {"name": "hash", "status": "fail", "verify": "stores a hashed password"}
    ])));
    assert_eq!(codes(&out), vec!["A015", "A014"]);
    assert!(out.findings[0].message.contains(
        "2 obligation(s) no passing test proves: \"rejects a duplicate email\", \"stores a hashed password\""
    ));
    assert_eq!(out.findings[1].severity, Severity::Error);
    assert_eq!(out.summary.discharge_funnel.entities_proven, 0);
}

#[specforge_test(
    behavior = "te_coverage_pass",
    verify = "an obligation no passing test names is A015 and a test naming an undeclared obligation is A016"
)]
fn a_test_naming_an_undeclared_obligation_is_reported() {
    let (_, out) = assess_input(create_user(json!([
        {"name": "dup", "status": "pass", "verify": "rejects a duplicate email"},
        {"name": "hash", "status": "pass", "verify": "stores a hashed pasword"}
    ])));
    assert_eq!(codes(&out), vec!["A015", "A016"]);
    assert!(
        out.findings[1]
            .message
            .contains("\"stores a hashed pasword\"")
    );
}

#[specforge_test(
    behavior = "te_coverage_pass",
    verify = "a proved formal claim discharges verify property obligations"
)]
fn an_entailed_formal_claim_discharges_property_obligations() {
    let (_, out) = assess_input(json!({
        "entities": [{
            "id": "unique_ids", "kind": "invariant", "testable": true, "incoming_edge_count": 1,
            "verify_kinds": ["property", "unit"],
            "verify_texts": ["ids never collide", "a second insert fails"]
        }],
        "test_results": {"results": {}},
        "proved_claims": ["unique_ids"]
    }));
    assert_eq!(codes(&out), vec!["A015"]);
    assert!(out.findings[0].message.contains("1 obligation(s)"));
    assert!(
        out.findings[0]
            .message
            .contains("\"a second insert fails\"")
    );
    assert_eq!(out.summary.discharge_funnel.formally_discharged, 1);
}

#[test]
fn without_recorded_results_obligations_are_not_scored() {
    let mut input = create_user(json!([]));
    input.as_object_mut().unwrap().remove("test_results");
    let (_, out) = assess_input(input);
    assert!(out.findings.is_empty(), "{:?}", codes(&out));
    assert!(out.summary.test_results.is_none());
}

#[test]
fn the_gate_percentage_is_proven_over_testable_and_vacuous_without_testables() {
    let mut summary = Summary::default();
    assert_eq!(summary.proof_pct(), 100.0);
    summary.testable_total = 4;
    summary.discharge_funnel.entities_proven = 1;
    assert_eq!(summary.proof_pct(), 25.0);
}

/// `tests/fixtures/coverage-cases.json`: the rule's golden vectors. The
/// testing extension's native tests hold its Wasm pass to the same file.
#[test]
fn the_rule_matches_the_shared_golden_vectors() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../tests/fixtures/coverage-cases.json")).unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let expect = &case["expect"];
        let (entities, out) = assess_input(case["input"].clone());

        let summary = serde_json::to_value(&out.summary).unwrap();
        assert_eq!(summary, expect["summary"], "summary of {name:?}");
        let back: Summary = serde_json::from_value(summary).unwrap();
        assert_eq!(back, out.summary, "summary of {name:?} round-trips");

        let findings: Vec<Value> = out
            .findings
            .iter()
            .map(|f| {
                json!({
                    "code": f.code,
                    "entity": entities[f.entity].id,
                    "severity": f.severity,
                    "message": f.message,
                    "suggestion": f.suggestion,
                })
            })
            .collect();
        assert_eq!(
            Value::from(findings),
            expect["findings"],
            "findings of {name:?}"
        );

        let verdicts: serde_json::Map<String, Value> = out
            .verdicts
            .iter()
            .map(|(id, v)| {
                let mut view = serde_json::to_value(v).unwrap();
                view["is_proven"] = json!(v.is_proven());
                view["status"] = serde_json::to_value(v.status()).unwrap();
                (id.clone(), view)
            })
            .collect();
        assert_eq!(
            Value::from(verdicts),
            expect["verdicts"],
            "verdicts of {name:?}"
        );
    }
}
