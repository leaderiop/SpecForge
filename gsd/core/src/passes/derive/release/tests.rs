use serde_json::{Value, json};

use super::*;

fn package(name: &str, version: Option<&str>) -> Value {
    let fields = version.map_or_else(
        || json!({ "name": name }),
        |v| json!({ "name": name, "version": v }),
    );
    json!({ "id": format!("pkg:cargo/{name}"), "kind": "package", "fields": fields })
}

/// v1.7 shipped, v1.8 open, v1.9 abandoned, `mvp` open, then `facts`.
fn input(facts: Vec<Value>) -> PassInput {
    let mut entities = vec![
        json!({ "id": "v1.7", "kind": "milestone", "fields": { "completion": "at 2026-02-01" } }),
        json!({ "id": "v1.8", "kind": "milestone" }),
        json!({ "id": "v1.9", "kind": "milestone", "fields": { "abandoned": "reason gone" } }),
        json!({ "id": "mvp", "kind": "milestone" }),
    ];
    entities.extend(facts);
    serde_json::from_value(json!({ "entities": entities, "edges": [] })).expect("a pass input")
}

fn released(found: &[PassValue]) -> Vec<(&str, Value)> {
    found
        .iter()
        .map(|v| (v.entity.as_str(), v.value.clone()))
        .collect()
}

fn packages() -> Vec<Value> {
    vec![
        package("a", Some("1.8.0")),
        package("b", Some("1.7.0")),
        package("c", None),
    ]
}

#[test]
fn release_of_e535_lists_the_package_not_at_the_open_milestones_version() {
    let found = release(&input(packages()), &[]);
    assert_eq!(
        released(&found),
        [(
            "v1.8",
            json!({ "packages": [{ "name": "b", "version": "1.7.0" }] })
        )]
    );
}

#[test]
fn release_of_e535_skips_an_exempt_package() {
    let found = release(&input(packages()), &["b"]);
    assert_eq!(released(&found), [("v1.8", json!({ "packages": [] }))]);
}

#[test]
fn release_of_e535_is_absent_without_a_versioned_package() {
    assert!(release(&input(vec![package("c", None)]), &[]).is_empty());
    assert!(release(&input(vec![]), &[]).is_empty());
}

#[test]
fn release_of_e535_caps_its_lines_and_keeps_them_raw() {
    let mut facts: Vec<Value> = (0..53)
        .map(|n| package(&format!("p{n:02}"), Some("0.1.0")))
        .collect();
    facts.push(package("z\u{1b}[2J", Some("1.0")));
    let found = release(&input(facts), &[]);
    let listed = found[0].value["packages"]
        .as_array()
        .expect("a list")
        .clone();
    assert_eq!(listed.len(), 50);
    assert_eq!(listed[0], json!({ "name": "p00", "version": "0.1.0" }));
    assert_eq!(found[0].value["more"], 4);
    let found = release(&input(vec![package("z\u{1b}", Some("1.\u{7}0"))]), &[]);
    assert_eq!(
        found[0].value,
        json!({ "packages": [{ "name": "z\u{1b}", "version": "1.\u{7}0" }] })
    );
}

#[test]
fn release_of_e535_holds_a_package_named_more_or_with_a_space_as_a_package() {
    let found = release(
        &input(vec![
            package("more", Some("5")),
            package("my crate", Some("0.1.0")),
        ]),
        &[],
    );
    assert_eq!(
        found[0].value,
        json!({ "packages": [
            { "name": "more", "version": "5" },
            { "name": "my crate", "version": "0.1.0" },
        ] })
    );
}

#[test]
fn release_of_e535_copies_a_shipped_milestones_recorded_tag() {
    let entities = vec![
        json!({ "id": "v1.6", "kind": "milestone", "fields": { "completion": "at 2026-01-01", "tag": "name, commit" },
                "values": { "tag": { "name": "v1.6", "commit": "abc" } } }),
        json!({ "id": "v1.7", "kind": "milestone", "fields": { "completion": "at 2026-02-01", "tag": "name" },
                "values": { "tag": { "name": "v1.7" } } }),
        json!({ "id": "v1.5", "kind": "milestone", "fields": { "completion": "at 2025-12-01" } }),
        json!({ "id": "v1.8", "kind": "milestone", "fields": { "tag": "name" },
                "values": { "tag": { "name": "v1.8" } } }),
    ];
    let input: PassInput =
        serde_json::from_value(json!({ "entities": entities, "edges": [] })).expect("a pass input");
    assert_eq!(
        released(&tagged(&input)),
        [
            ("v1.6", json!({ "tag": "v1.6", "commit": "abc" })),
            ("v1.7", json!({ "tag": "v1.7" })),
        ]
    );
}
