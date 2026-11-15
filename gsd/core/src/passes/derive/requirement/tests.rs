use serde_json::json;

use super::*;

/// R1, which no plan lists, and R2, which plan 01-01 lists, whose task T1 a commit names and a passing test names.
fn project() -> PassInput {
    let value = json!({
        "entities": [
            { "id": "R1", "kind": "requirement", "fields": { "text": "a board is shared" } },
            { "id": "R2", "kind": "requirement", "fields": { "text": "a task is added" } },
            { "id": "v1.0/01/01-01", "kind": "plan", "fields": { "phase": "v1.0/01" },
              "values": { "requirements": ["R2"] } },
            { "id": "v1.0/01/01-01/T1", "kind": "task",
              "fields": { "plan": "v1.0/01/01-01", "type": "auto" } },
            { "id": "abcdef123456", "kind": "commit",
              "fields": { "task": "v1.0/01/01-01/T1", "at": "2026-10-29T10:00:00Z" } },
        ],
        "edges": [],
    });
    let mut input: PassInput =
        serde_json::from_value(crate::passes::plans::with_field_edges(value))
            .expect("a pass input");
    crate::passes::thread::fixture::add_test(
        &mut input,
        "js:app/test/t.test.js#adds",
        "adds",
        "test/t.test.js",
        TestStatus::Passed,
        &["v1.0/01/01-01/T1"],
    );
    input
}

#[test]
fn every_requirement_gets_its_proof() {
    let input = project();
    let commits = crate::passes::commits::commits(&input);
    let proofs = proofs(&input, &commits);
    assert_eq!(
        proofs,
        BTreeMap::from([("R1", Proof::NoTask), ("R2", Proof::Proven)])
    );
    let written: Vec<(String, String, serde_json::Value)> = values(&proofs)
        .into_iter()
        .map(|v| (v.entity, v.field, v.value))
        .collect();
    assert_eq!(
        written,
        [
            ("R1".to_string(), "proof".to_string(), json!("no_task")),
            ("R2".to_string(), "proof".to_string(), json!("proven")),
        ]
    );
}
