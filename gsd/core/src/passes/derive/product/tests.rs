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
