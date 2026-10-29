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
