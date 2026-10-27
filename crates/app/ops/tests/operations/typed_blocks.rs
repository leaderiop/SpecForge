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
";

fn edit(wire: serde_json::Value) -> Edit {
    serde_json::from_value(wire).expect("an edit")
}

/// The text of `a.spec` once `wire` is planned, or the refusal's code and message.
fn run(wire: serde_json::Value) -> Result<String, (String, String)> {
    let project = Assembled::typed().sources(&[("a.spec", SOURCE)]);
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
#[ignore = "K3 T10"]
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
    assert!(unset.contains("completion { at 2026-10-27 }"), "{unset}");

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
