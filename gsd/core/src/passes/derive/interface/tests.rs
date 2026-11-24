use serde_json::{Value, json};

use super::*;

fn item(id: &str, change: Option<&str>) -> Value {
    let fields = change.map_or_else(|| json!({ "item": id }), |c| json!({ "change": c }));
    json!({ "id": id, "kind": "api_item", "fields": fields })
}

/// v1.7 shipped, v1.8 open, v1.9 abandoned, then `items`.
fn input(items: Vec<Value>) -> PassInput {
    let mut entities = vec![
        json!({ "id": "v1.7", "kind": "milestone", "fields": { "completion": "at 2026-02-01" },
                "values": { "completion": { "at": "2026-02-01" } } }),
        json!({ "id": "v1.8", "kind": "milestone" }),
        json!({ "id": "v1.9", "kind": "milestone", "fields": { "abandoned": "reason gone" } }),
    ];
    entities.extend(items);
    serde_json::from_value(json!({ "entities": entities, "edges": [] })).expect("a pass input")
}

fn breaks(found: &[PassValue]) -> Vec<(&str, Value)> {
    found
        .iter()
        .map(|v| (v.entity.as_str(), v.value.clone()))
        .collect()
}

#[test]
fn api_breaks_of_e533_lists_a_removed_item_on_the_open_milestone_only() {
    let found = values(&input(vec![
        item("api:rust:a::f", Some("removed")),
        item("api:rust:a::g", Some("unchanged")),
        item("api:rust:a::h", Some("added")),
    ]));
    assert_eq!(breaks(&found), [("v1.8", json!(["removed api:rust:a::f"]))]);
}

#[test]
fn api_breaks_of_e533_is_empty_when_every_judged_item_is_clean() {
    let found = values(&input(vec![
        item("api:rust:a::f", Some("unchanged")),
        item("api:rust:a::g", None),
    ]));
    assert_eq!(breaks(&found), [("v1.8", json!([]))]);
}

#[test]
fn api_breaks_of_e533_is_absent_without_an_accepted_side() {
    assert!(values(&input(vec![item("api:rust:a::f", None)])).is_empty());
    assert!(values(&input(vec![])).is_empty());
}

#[test]
fn api_breaks_of_e533_sorts_by_id_and_keeps_the_three_breaking_changes() {
    let found = values(&input(vec![
        item("api:rust:a::z", Some("narrowed")),
        item("api:rust:a::b", Some("changed")),
        item("api:rust:a::m", Some("removed")),
        item("api:rust:a::q", Some("renamed")),
    ]));
    assert_eq!(
        breaks(&found),
        [(
            "v1.8",
            json!([
                "changed api:rust:a::b",
                "removed api:rust:a::m",
                "narrowed api:rust:a::z"
            ])
        )]
    );
}

#[test]
fn api_breaks_of_e533_caps_at_50_lines_then_one_more_line() {
    let items = (0..60)
        .map(|i| item(&format!("api:rust:a::f{i:02}"), Some("removed")))
        .collect();
    let found = values(&input(items));
    let lines = found[0].value.as_array().expect("a list");
    assert_eq!(lines.len(), 51);
    assert_eq!(lines[49], "removed api:rust:a::f49");
    assert_eq!(lines[50], "more 10");
}

#[test]
fn api_breaks_of_e533_escapes_each_id() {
    let found = values(&input(vec![item("api:rust:a::f\nb`'", Some("changed"))]));
    assert_eq!(
        breaks(&found),
        [("v1.8", json!(["changed api:rust:a::f\\nb`\\'"]))]
    );
}
