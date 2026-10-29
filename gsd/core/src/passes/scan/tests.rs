use std::collections::BTreeMap;

use super::*;

/// One read's facts, as the host sends them.
fn fact(path: &str, exists: bool, text: Option<&str>, error: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "path": path,
        "exists": exists,
        "lines": exists.then(|| text.map_or(0, |t| t.lines().count())),
        "text": text,
        "error": error,
    })
}

fn entity(id: &str, kind: &str, fields: &[(&str, &str)]) -> serde_json::Value {
    serde_json::json!({ "id": id, "kind": kind, "fields": fields.iter().cloned().collect::<BTreeMap<_, _>>() })
}

fn input(
    read: &str,
    facts: Vec<serde_json::Value>,
    truncated: bool,
    entities: Vec<serde_json::Value>,
) -> PassInput {
    serde_json::from_value(serde_json::json!({
        "entities": entities,
        "files": { read: { "facts": facts, "truncated": truncated } },
    }))
    .expect("a pass input")
}

fn state_of(input: &PassInput) -> Vec<String> {
    compute(input)
        .values
        .iter()
        .filter(|v| v.field == "state")
        .map(|v| v.value.as_str().unwrap_or_default().to_string())
        .collect()
}

fn artifact() -> serde_json::Value {
    entity(
        "a/AR1",
        "artifact",
        &[("path", "src/a.js"), ("contains", "x")],
    )
}

#[test]
fn a_file_that_is_there_and_has_the_text_is_present() {
    let found = input(
        "artifacts",
        vec![fact("src/a.js", true, Some("x"), None)],
        false,
        vec![artifact()],
    );
    assert_eq!(state_of(&found), ["present"]);
}

#[test]
fn a_file_that_is_not_there_is_missing() {
    let found = input(
        "artifacts",
        vec![fact("src/a.js", false, None, None)],
        false,
        vec![artifact()],
    );
    assert_eq!(state_of(&found), ["missing"]);
}

#[test]
fn a_withheld_or_truncated_read_is_unchecked_never_missing() {
    for (facts, truncated) in [
        (vec![fact("src/a.js", false, None, Some("budget"))], false),
        (vec![fact("src/a.js", true, None, Some("denied"))], false),
        (vec![fact("src/a.js", false, None, None)], true),
        (Vec::new(), false),
        (vec![fact("src/a.js", true, None, None)], false),
    ] {
        let found = input("artifacts", facts, truncated, vec![artifact()]);
        assert_eq!(state_of(&found), ["unchecked"]);
    }
}

#[test]
fn a_wave0_item_is_done_when_its_file_exists_and_a_linked_test_ran() {
    let item = entity("v/W0-1", "wave0_item", &[("path", "test/a.test.js")]);
    let done = |tests: bool| {
        let mut found = input(
            "wave0",
            vec![fact("test/a.test.js", true, None, None)],
            false,
            vec![item.clone()],
        );
        if tests {
            found.test_results = serde_json::from_value(serde_json::json!({
                "results": { "v/W0-1": { "tests": [{ "name": "stub runs", "status": "passed" }] } }
            }))
            .expect("a report");
        }
        compute(&found)
            .values
            .iter()
            .map(|v| v.value.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(done(false), [serde_json::json!(false)]);
    assert_eq!(done(true), [serde_json::json!(true)]);
}

fn state_for(read: &str, facts: Vec<serde_json::Value>, e: serde_json::Value) -> String {
    state_of(&input(read, facts, false, vec![e])).remove(0)
}

fn exporting(names: &str) -> serde_json::Value {
    let fields = [("path", "src/a.rs"), ("exports", names)];
    let mut e = entity("a/AR1", "artifact", &fields);
    e["values"] = serde_json::json!({ "exports": [names] });
    e
}

#[test]
fn an_export_is_found_by_substring_whatever_the_language() {
    for text in ["pub fn add() {}", "export {\n  add,\n}"] {
        let facts = vec![fact("src/a.rs", true, Some(text), None)];
        assert_eq!(state_for("artifacts", facts, exporting("add")), "present");
    }
    let facts = vec![fact("src/a.rs", true, Some("pub fn sub() {}"), None)];
    assert_eq!(
        state_for("artifacts", facts, exporting("add")),
        "lacks_export"
    );
}

#[test]
fn min_lines_counts_as_upstream_does() {
    let at_least = |text: &str, min: &str| {
        let e = entity(
            "a/AR1",
            "artifact",
            &[("path", "src/a.js"), ("min_lines", min)],
        );
        state_for(
            "artifacts",
            vec![fact("src/a.js", true, Some(text), None)],
            e,
        )
    };
    // `split('\n')`: a file of 5 lines ending in a newline is 6, an empty file 1.
    assert_eq!(at_least("1\n2\n3\n4\n5\n", "6"), "present");
    assert_eq!(at_least("1\n2\n3\n4\n5\n", "7"), "too_short");
    assert_eq!(at_least("", "1"), "present");
    assert_eq!(at_least("", "2"), "too_short");
}

fn linked(from: Option<&str>, to: Option<&str>) -> String {
    let (mut links, mut targets) = (Vec::new(), Vec::new());
    links.extend(from.map(|t| fact("src/from.js", true, Some(t), None)));
    targets.extend(to.map(|t| fact("src/to.js", true, Some(t), None)));
    let e = entity(
        "a/KL1",
        "key_link",
        &[
            ("from", "src/from.js"),
            ("to", "src/to.js"),
            ("pattern", "function"),
        ],
    );
    let found: PassInput = serde_json::from_value(serde_json::json!({
        "entities": [e],
        "files": {
            "links": { "facts": links, "truncated": false },
            "link_targets": { "facts": targets, "truncated": false },
        },
    }))
    .expect("a pass input");
    state_of_links(&found)
}

fn state_of_links(found: &PassInput) -> String {
    compute(found).values[0]
        .value
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn a_key_link_pattern_found_only_in_the_target_is_wired() {
    assert_eq!(linked(Some(""), Some("function add() {}")), "wired");
    assert_eq!(linked(Some("function f() {}"), Some("")), "wired");
    assert_eq!(linked(Some(""), Some("")), "not_wired");
    assert_eq!(linked(Some(""), None), "unchecked");
}

fn with_plan_file(summarised: bool, facts: Vec<serde_json::Value>) -> PassInput {
    let mut entities = vec![
        serde_json::json!({ "id": "v1.0/01/01-01", "kind": "plan", "values": { "files_modified": ["src/a.js"] } }),
        serde_json::json!({ "id": "v1.0/01/01-VERIFICATION", "kind": "verification", "fields": { "phase": "v1.0/01" } }),
    ];
    if summarised {
        entities.push(serde_json::json!({ "id": "v1.0/01/01-01/01-01-SUMMARY", "kind": "summary", "fields": { "plan": "v1.0/01/01-01" } }));
    }
    serde_json::from_value(crate::passes::plans::with_field_edges(serde_json::json!({
        "entities": entities,
        "edges": [{ "source": "v1.0/01/01-01", "target": "v1.0/01", "label": "phase" }],
        "files": { "modified": { "facts": facts, "truncated": false } },
    })))
    .expect("a pass input")
}

#[test]
fn e505_judges_only_a_plan_that_has_a_summary() {
    let facts = || vec![fact("src/a.js", true, Some("// FIXME later"), None)];
    assert!(check(&with_plan_file(false, facts())).is_empty());
    assert_eq!(check(&with_plan_file(true, facts())).len(), 1);
}

#[test]
fn a_file_the_host_did_not_serve_makes_the_scan_partial() {
    let partial = |facts: Vec<serde_json::Value>| {
        verification_patterns(&with_plan_file(false, facts))
            .iter()
            .any(|v| v.field == "scan_partial")
    };
    assert!(partial(vec![fact("src/a.js", false, None, Some("budget"))]));
    assert!(partial(vec![fact("src/a.js", true, None, None)]));
    assert!(!partial(vec![fact("src/a.js", true, Some("ok"), None)]));
}
