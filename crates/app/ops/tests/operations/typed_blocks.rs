//! Typed edits address a block member by its path (ADR 0285, plan K3): `set`, `unset`, `append` and `remove`.

use serde_json::json;
use specforge_ops::authoring::edit::{self, Edit};
use specforge_ops::testing::Viewed;
use specforge_project::testing::Assembled;
use specforge_test::prelude::*;

const SOURCE: &str = "\
milestone v1.0 \"One\" {
  completion { at 2026-10-27 audit v1.0-audit }
}

audit v1.0-audit \"Audit\" {
  milestone v1.0
}

phase 02 \"Two\" {
  milestone v1.0
}

phase 03 \"Three\" {
  milestone v1.0
}

plan 03-02 \"Plan\" {
  phase v1.0/03
  files_modified [\"src/a.ts\"]
}

summary 03-02-S \"S\" {
  plan v1.0/03/03-02
  requires [
    { phase 02 provides \"x\" }
  ]
}

refactor_proposal r1 \"R\" {
  reasons [threshold]
}

discussion d1 \"D\" {
  scores { goal 10 }
}
";

fn edit(wire: serde_json::Value) -> Edit {
    serde_json::from_value(wire).expect("an edit")
}

/// The text of `a.spec` once `wire` is planned, or the refusal's code and message.
fn run(wire: serde_json::Value) -> Result<String, (String, String)> {
    let project = Assembled::typed().sources(&[("a.spec", SOURCE)]);
    project.write("src/a.ts", "");
    match edit::plan(&project.view(), &[edit(wire)]) {
        Ok(planned) => Ok(planned
            .change()
            .after()
            .remove("a.spec")
            .unwrap_or_else(|| SOURCE.to_string())),
        Err(refused) => Err((refused.error.code.to_string(), refused.error.message)),
    }
}

#[specforge_test(
    behavior = "edit_entity_field",
    verify = "a typed edit sets a block member, appends and removes a block-list item, and refuses the other ways to change one"
)]
fn a_typed_edit_sets_appends_and_removes_block_items() {
    let set = run(
        json!({"op": "set", "entity_id": "03-02-S", "field": "requires[0].phase", "value": "03"}),
    )
    .unwrap();
    assert!(set.contains("{ phase 03 provides \"x\" }"), "{set}");

    let appended = run(
        json!({"op": "append", "entity_id": "03-02-S", "field": "requires",
        "value": {"phase": "03", "provides": "y"}}),
    )
    .unwrap();
    assert!(
        appended.contains("{ phase 02 provides \"x\" }")
            && appended.contains("{ phase 03 provides \"y\" }"),
        "{appended}"
    );

    let removed =
        run(json!({"op": "remove", "entity_id": "03-02-S", "field": "requires", "index": 0}))
            .unwrap();
    assert!(!removed.contains("provides \"x\""), "{removed}");

    let unset =
        run(json!({"op": "unset", "entity_id": "v1.0", "field": "completion.audit"})).unwrap();
    assert!(
        unset.contains("completion {\n    at 2026-10-27\n  }")
            && !unset.contains("audit v1.0-audit\n  }"),
        "{unset}"
    );

    let linked =
        run(json!({"op": "link", "entity_id": "r1", "field": "reasons", "target": "jump"}))
            .unwrap();
    assert!(linked.contains("reasons [threshold, jump]"), "{linked}");

    for (wire, code, fragment) in [
        (
            json!({"op": "unset", "entity_id": "03-02-S", "field": "requires[0]"}),
            "not_editable",
            "use remove",
        ),
        (
            json!({"op": "set", "entity_id": "03-02-S", "field": "requires[5]", "value": {"phase": "03", "provides": "z"}}),
            "not_editable",
            "use append",
        ),
        (
            json!({"op": "append", "entity_id": "r1", "field": "reasons", "value": "jump"}),
            "not_editable",
            "use link",
        ),
        (
            json!({"op": "set", "entity_id": "03-02-S", "field": "requires[01].phase", "value": "03"}),
            "not_editable",
            "not a field",
        ),
    ] {
        let (got, message) = run(wire.clone()).unwrap_err();
        assert_eq!(
            (got.as_str(), message.contains(fragment)),
            (code, true),
            "{wire}: {message}"
        );
    }
}

#[specforge_test(
    behavior = "create_entity",
    verify = "a skeleton writes a block list and a typed block empty, and a list of words with a slot"
)]
fn a_skeleton_writes_typed_fields_empty() {
    use specforge_ops::authoring::edit::Skeletons;
    use specforge_ops::authoring::format::Place;
    let project = Assembled::typed().sources(&[("a.spec", SOURCE)]);
    let view = project.view();
    let path = view.env().spec_root().join("new.spec");
    let skeletons = Skeletons::at(&view, Place::File(&path));
    for (kind, field, text) in [
        ("summary", "requires", "requires []"),
        ("milestone", "completion", "completion {}"),
    ] {
        let skeleton = skeletons.member(kind, field).expect("a skeleton");
        assert_eq!(skeleton.text, text, "{kind}.{field}");
    }
    let words = skeletons.member("refactor_proposal", "reasons").unwrap();
    assert!(words.text.starts_with("reasons ["), "{}", words.text);
}

#[specforge_test(
    behavior = "edit_entity_field",
    verify = "a typed edit sets a block member, appends and removes a block-list item, and refuses the other ways to change one"
)]
fn a_json_number_or_boolean_is_a_leaf_of_an_integer_or_bool_member() {
    let set = run(json!({"op": "set", "entity_id": "d1", "field": "scores", "value": {"goal": 50, "ambiguity": 7}}))
        .unwrap();
    let squeezed = set.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(squeezed.contains("scores { goal 50 ambiguity 7 }"), "{set}");
    let member =
        run(json!({"op": "set", "entity_id": "d1", "field": "scores.goal", "value": 60})).unwrap();
    assert!(
        member
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("goal 60"),
        "{member}"
    );
    // The member's own form judges the number: out of range and not an integer are both E061.
    for bad in [json!(500), json!(1.5)] {
        let (_, message) =
            run(json!({"op": "set", "entity_id": "d1", "field": "scores.goal", "value": bad}))
                .unwrap_err();
        assert!(message.contains("E061"), "{bad}: {message}");
    }
    // A text member and a bool given the wrong leaf are refused by name, not as "is a block".
    for (field, value, fragment) in [
        ("requires[0].provides", json!(5), "takes text"),
        ("scores.goal", json!(true), "an integer, not a boolean"),
    ] {
        let (code, message) = run(json!({"op": "set", "entity_id": if field.starts_with("scores") { "d1" } else { "03-02-S" }, "field": field, "value": value})).unwrap_err();
        assert_eq!(code, "not_editable", "{message}");
        assert!(message.contains(fragment), "{field}: {message}");
    }
}

#[specforge_test(
    behavior = "edit_entity_field",
    verify = "set reads a JSON object or an array of objects as a typed block or a block list, and any other text as one value"
)]
fn a_value_is_json_exactly_where_the_declaration_is_a_block() {
    let project = Assembled::typed().sources(&[("a.spec", SOURCE)]);
    let view = project.view();
    for (entity, path, json) in [
        ("d1", "scores", true),
        ("03-02-S", "requires", true),
        ("03-02-S", "requires[0]", true),
        ("03-02-S", "requires[0].provides", false),
        ("v1.0", "completion", true),
        ("v1.0", "completion.at", false),
        ("r1", "reasons", false),
        ("r1", "nope", false),
    ] {
        assert_eq!(
            edit::takes_json(&view, entity, path),
            json,
            "{entity} {path}"
        );
    }
}
