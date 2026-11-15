use serde_json::{Value, json};

use super::*;

/// A pass input of `entities` (JSON), with the edges `edges` as `(source, label, target)`.
fn input(entities: Value, edges: &[(&str, &str, &str)]) -> PassInput {
    let edges: Vec<Value> = edges
        .iter()
        .map(
            |(source, label, target)| json!({ "source": source, "label": label, "target": target }),
        )
        .collect();
    serde_json::from_value(json!({ "entities": entities, "edges": edges })).expect("a pass input")
}

/// `field` of `entity` among `values`.
fn of<'v>(values: &'v [PassValue], entity: &str, field: &str) -> Option<&'v Value> {
    values
        .iter()
        .find(|v| v.entity == entity && v.field == field)
        .map(|v| &v.value)
}

fn glossary() -> Value {
    json!([
        { "id": "work_item", "kind": "term",
          "fields": { "name": "task", "definition": "a unit of work" },
          "values": { "aliases": ["to-do"], "avoid": ["ticket"] } },
        { "id": "R1", "kind": "requirement", "fields": { "text": "A user can add a Task" } },
        { "id": "R2", "kind": "requirement", "fields": { "text": "Every To-Do is kept" } },
        { "id": "R3", "kind": "requirement", "fields": { "text": "A ticket is closed" } },
        { "id": "D1", "kind": "decision", "fields": { "text": "Store on disk", "rationale": "a task survives" } },
        { "id": "add", "kind": "journey", "fields": { "persona": "owner" },
          "values": { "steps": [ { "action": "opens the board" }, { "action": "adds a task" } ] } },
        { "id": "T1", "kind": "task", "fields": { "action": "write the task form" } },
    ])
}

/// The name and the alias are uses, case-insensitively, in any prose field (a decision's rationale, a journey step's
/// action); an avoided word is none, and a task's action is no prose.
#[test]
fn a_term_is_used_where_its_name_or_an_alias_is_a_whole_word() {
    let values = values(&input(glossary(), &[]));
    assert_eq!(
        of(&values, "work_item", "used_in"),
        Some(&json!(["D1", "R1", "R2", "add"]))
    );
}

#[test]
fn a_term_inside_a_longer_word_is_unused() {
    let values = values(&input(
        json!([
            { "id": "work_item", "kind": "term", "fields": { "name": "task", "definition": "a unit" } },
            { "id": "R1", "kind": "requirement", "fields": { "text": "Tasks are saved; multitasking is not" } },
        ]),
        &[],
    ));
    assert_eq!(of(&values, "work_item", "used_in"), Some(&json!([])));
}

#[test]
fn an_unused_term_reads_an_empty_list() {
    let values = values(&input(
        json!([
            { "id": "board", "kind": "term", "fields": { "name": "board", "definition": "the tasks" } },
            { "id": "R1", "kind": "requirement", "fields": { "text": "A user can add a task" } },
        ]),
        &[],
    ));
    assert_eq!(of(&values, "board", "used_in"), Some(&json!([])));
}

/// Sessions `(id, started)` of phase `v1.0/01`, each with one test that names `journey` when it is not empty, and the
/// edges a host sends for the tests' `session` and `journey`.
fn sessions(of: &[(&str, Option<&str>, &str)]) -> PassInput {
    let mut entities = Vec::new();
    let mut edges = Vec::new();
    for (id, started, journey) in of {
        let mut fields = json!({ "phase": "v1.0/01" });
        if let Some(started) = started {
            fields["started"] = json!(started);
        }
        entities.push(json!({ "id": id, "kind": "uat_session", "fields": fields }));
        let test = format!("{id}/U1");
        entities.push(json!({ "id": test, "kind": "uat_test", "fields": { "session": id, "expected": "it works" } }));
        edges.push((test.clone(), "session".to_string(), id.to_string()));
        if !journey.is_empty() {
            edges.push((test, "journey".to_string(), journey.to_string()));
        }
    }
    entities.push(json!({ "id": "add", "kind": "journey", "fields": { "persona": "owner" } }));
    entities.push(json!({ "id": "share", "kind": "journey", "fields": { "persona": "owner" } }));
    let edges: Vec<(&str, &str, &str)> = edges
        .iter()
        .map(|(s, l, t)| (s.as_str(), l.as_str(), t.as_str()))
        .collect();
    input(Value::Array(entities), &edges)
}

#[test]
fn the_last_uat_is_the_latest_session_whose_test_names_the_journey() {
    let values = values(&sessions(&[
        ("v1.0/01/01-UAT", Some("2026-10-29T10:00:00Z"), "add"),
        ("v1.0/01/02-UAT", Some("2026-10-30T10:00:00Z"), "add"),
        ("v1.0/01/03-UAT", Some("2026-10-31T10:00:00Z"), "share"),
        ("v1.0/01/04-UAT", Some("2026-11-01T10:00:00Z"), ""),
    ]));
    assert_eq!(
        of(&values, "add", "last_uat"),
        Some(&json!("v1.0/01/02-UAT"))
    );
    assert_eq!(
        of(&values, "share", "last_uat"),
        Some(&json!("v1.0/01/03-UAT"))
    );
}

#[test]
fn a_journey_no_test_names_has_no_last_uat() {
    let values = values(&sessions(&[(
        "v1.0/01/01-UAT",
        Some("2026-10-29T10:00:00Z"),
        "share",
    )]));
    assert_eq!(of(&values, "add", "last_uat"), None);
}

#[test]
fn sessions_started_at_once_are_ordered_by_id() {
    let values = values(&sessions(&[
        ("v1.0/01/02-UAT", Some("2026-10-29T10:00:00Z"), "add"),
        ("v1.0/01/01-UAT", Some("2026-10-29T10:00:00Z"), "add"),
    ]));
    assert_eq!(
        of(&values, "add", "last_uat"),
        Some(&json!("v1.0/01/02-UAT"))
    );
}

#[test]
fn an_undated_session_is_older_than_a_dated_one() {
    let values = values(&sessions(&[
        ("v1.0/01/01-UAT", Some("2026-10-29T10:00:00Z"), "add"),
        ("v1.0/01/09-UAT", None, "add"),
    ]));
    assert_eq!(
        of(&values, "add", "last_uat"),
        Some(&json!("v1.0/01/01-UAT"))
    );
}
