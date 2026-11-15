//! What `@gsd/core` declares for the invariant (`spec/governance.md`), pinned through the SDK's runtime-free
//! `MockHost`. The model doc's table is read here, each row's field, type and flags, so the doc cannot drift from the
//! declaration.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{describe, field_of, fields_of, model_doc};

const GOVERNANCE_MD: &str = include_str!("../spec/governance.md");

#[test]
fn the_invariant_declares_the_fields_of_the_model() {
    assert_eq!(
        fields_of("invariant"),
        [
            "statement",
            "description",
            "scope",
            "holds_from",
            "proof",
            "items",
            "criticality",
            "tags",
            "refs",
        ]
    );
    model_doc::assert_declared(GOVERNANCE_MD, &["invariant"]);
    let statement = field_of("invariant", "statement");
    assert_eq!(statement["field_type"], "string", "{statement}");
    assert_eq!(statement["required"], true, "{statement}");
    for (field, words) in [
        ("scope", &["project", "milestone"][..]),
        ("criticality", &["critical", "high", "medium", "low"][..]),
    ] {
        let declared = field_of("invariant", field);
        assert_eq!(declared["field_type"], "enum", "{declared}");
        assert_eq!(
            declared["enum_values"],
            serde_json::json!(words),
            "{declared}"
        );
    }
    let holds_from = field_of("invariant", "holds_from");
    assert_eq!(holds_from["field_type"], "reference", "{holds_from}");
    assert_eq!(holds_from["target_kind"], "phase", "{holds_from}");
    for (field, kind) in [("proof", "test"), ("items", "item")] {
        let declared = field_of("invariant", field);
        assert_eq!(declared["field_type"], "reference_list", "{declared}");
        assert_eq!(declared["target_kind"], kind, "{declared}");
        assert!(declared["inverse_of"].is_null(), "{declared}");
    }
    for field in ["tags", "refs"] {
        assert_eq!(field_of("invariant", field)["field_type"], "string_list");
    }
    for field in fields_of("invariant") {
        let declared = field_of("invariant", &field);
        assert!(
            declared["writer"].is_null() || declared["writer"] == "author",
            "every field is authored: {declared}"
        );
        assert_ne!(
            declared["required"] == true,
            field != "statement",
            "only the statement is required: {declared}"
        );
    }
}

#[test]
fn an_invariant_is_in_the_project_scope() {
    let invariant = describe("entities")["items"]
        .as_array()
        .and_then(|k| k.iter().find(|k| k["name"] == "invariant"))
        .cloned()
        .expect("the invariant kind");
    assert!(invariant["scoped_by"].is_null(), "{invariant}");
}
