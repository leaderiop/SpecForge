use serde_json::{Value, json};

use super::*;
use crate::passes::gates::tests::{gate_of, input};

const HOW: &str = "bump the milestone's major version (Cargo's rule: for 0.x the minor), or restore the item, or \
    commit the updated accepted report when the change is accepted; then run `specforge collect` again: this check \
    reads the API facts of the last collect";

fn shipped(id: &str, at: &str) -> Value {
    json!({ "id": id, "kind": "milestone", "fields": { "completion": format!("at {at}"), "shipped": at } })
}

fn open(id: &str, breaks: &[&str]) -> Value {
    json!({ "id": id, "kind": "milestone", "values": { "api_breaks": breaks } })
}

fn close(subject: &str, entities: Vec<Value>) -> PassInput {
    input(
        gate_of("complete_milestone", Some(subject)),
        entities,
        vec![],
    )
}

#[test]
fn e533_refuses_a_removed_item_without_a_major_bump() {
    let found = e533(&close(
        "v1.8",
        vec![
            shipped("v1.7", "2026-02-01"),
            open("v1.8", &["removed api:rust:a::f"]),
        ],
    ));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, "E533");
    assert_eq!(found[0].severity, PassSeverity::Error);
    assert_eq!(found[0].entity.as_deref(), Some("v1.8"));
    assert_eq!(
        found[0].message,
        "API item 'api:rust:a::f' is removed without a major version bump: milestone 'v1.8' is compatible with the \
         last shipped 'v1.7'"
    );
    assert_eq!(found[0].suggestion.as_deref(), Some(HOW));
}

#[test]
fn e533_allows_a_break_at_a_major_bump() {
    let found = e533(&close(
        "v2.0",
        vec![
            shipped("v1.7", "2026-02-01"),
            open("v2.0", &["removed api:rust:a::f"]),
        ],
    ));
    assert!(found.is_empty());
}

#[test]
fn e533_is_silent_outside_a_gate() {
    let mut found = close(
        "v1.8",
        vec![
            shipped("v1.7", "2026-02-01"),
            open("v1.8", &["removed api:rust:a::f"]),
        ],
    );
    found.trigger = None;
    assert!(e533(&found).is_empty());
}
