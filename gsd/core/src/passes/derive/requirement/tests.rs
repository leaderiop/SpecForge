use serde_json::json;

use super::*;

/// R1, which no plan lists (so it reads its tests alone, and no test names it), and R2, which plan 01-01 lists, whose
/// task T1 a commit names and a passing test names.
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
        BTreeMap::from([("R1", Proof::NoTest), ("R2", Proof::Proven)])
    );
    let written: Vec<(String, String, serde_json::Value)> = values(&input, &commits, &proofs)
        .into_iter()
        .map(|v| (v.entity, v.field, v.value))
        .collect();
    assert_eq!(
        written,
        [
            ("R1".to_string(), "proof".to_string(), json!("no_test")),
            ("R2".to_string(), "proof".to_string(), json!("proven")),
        ]
    );
}

/// Plans 01-01 and 02-01 list R2, plan 03-01 lists R3; each task is named by one recorded commit, whose
/// `items_changed` is written as `items` gives it (`None`: unwritten).
fn itemized(items: &[(&str, &str, Option<Vec<&str>>)]) -> PassInput {
    let mut entities = vec![
        json!({ "id": "R2", "kind": "requirement", "fields": { "text": "a task is added" } }),
        json!({ "id": "R3", "kind": "requirement", "fields": { "text": "a board is shared" } }),
    ];
    for (plan, requirement) in [("01-01", "R2"), ("02-01", "R2"), ("03-01", "R3")] {
        entities.push(json!({ "id": format!("v1.0/01/{plan}"), "kind": "plan",
                              "values": { "requirements": [requirement] } }));
    }
    for (n, (task, task_type, changed)) in items.iter().enumerate() {
        let (plan, _) = task.split_once('/').expect("a plan");
        let task = format!("v1.0/01/{task}");
        entities.push(json!({ "id": task, "kind": "task",
                              "fields": { "plan": format!("v1.0/01/{plan}"), "type": task_type } }));
        let mut commit = json!({ "id": format!("abcdef12345{n}"), "kind": "commit",
                                 "fields": { "task": task, "at": format!("2026-10-29T10:00:0{n}Z") } });
        if let Some(changed) = changed {
            commit["fields"]["items_changed"] = json!(changed.join(", "));
            commit["values"] = json!({ "items_changed": changed });
        }
        entities.push(commit);
    }
    serde_json::from_value(crate::passes::plans::with_field_edges(
        json!({ "entities": entities, "edges": [] }),
    ))
    .expect("a pass input")
}

/// The `satisfied_by` value `values` writes for `requirement`, if any.
fn satisfied_by(input: &PassInput, requirement: &str) -> Option<serde_json::Value> {
    let commits = crate::passes::commits::commits(input);
    let proofs = proofs(input, &commits);
    values(input, &commits, &proofs)
        .into_iter()
        .find(|v| v.entity == requirement && v.field == "satisfied_by")
        .map(|v| v.value)
}

#[test]
fn satisfied_by_is_each_item_the_commits_of_its_tasks_changed_once() {
    let input = itemized(&[
        (
            "01-01/T1",
            "auto",
            Some(vec!["js:app/src/b.js::add", "js:app/src/a.js::board"]),
        ),
        ("01-01/T2", "tdd", Some(vec!["js:app/src/c.js::check"])),
        (
            "02-01/T1",
            "auto",
            Some(vec!["js:app/src/a.js::board", "js:app/src/d.js::done"]),
        ),
        ("03-01/T1", "auto", Some(vec!["js:app/src/e.js::share"])),
    ]);
    // Both plans' tasks, whatever their type, sorted and each once; the task's own `items` are not read, and the
    // commit of R3's task counts for R3 alone.
    assert_eq!(
        satisfied_by(&input, "R2"),
        Some(json!([
            "js:app/src/a.js::board",
            "js:app/src/b.js::add",
            "js:app/src/c.js::check",
            "js:app/src/d.js::done"
        ]))
    );
    assert_eq!(
        satisfied_by(&input, "R3"),
        Some(json!(["js:app/src/e.js::share"]))
    );
}

#[test]
fn an_unwritten_items_changed_adds_nothing() {
    let input = itemized(&[
        ("01-01/T1", "auto", None),
        ("02-01/T1", "auto", Some(vec!["js:app/src/d.js::done"])),
        ("03-01/T1", "auto", None),
    ]);
    assert_eq!(
        satisfied_by(&input, "R2"),
        Some(json!(["js:app/src/d.js::done"]))
    );
    assert_eq!(satisfied_by(&input, "R3"), None);
}

#[test]
fn a_requirement_with_no_commit_that_changed_an_item_has_no_satisfied_by() {
    // A commit that changed no item, and the commit of another requirement's task.
    let input = itemized(&[
        ("01-01/T1", "auto", Some(vec![])),
        ("03-01/T1", "auto", Some(vec!["js:app/src/e.js::share"])),
    ]);
    assert_eq!(satisfied_by(&input, "R2"), None);
    // One with no task at all has none either, and one whose task no commit names (its `items` written) none.
    assert_eq!(satisfied_by(&project(), "R1"), None);
    let mut uncommitted = itemized(&[("01-01/T1", "auto", Some(vec!["js:app/src/b.js::add"]))]);
    uncommitted.entities.retain(|e| e.kind != "commit");
    assert_eq!(satisfied_by(&uncommitted, "R2"), None);
}
