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
            "status",
            "last_verified",
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
        let computed = ["status", "last_verified"].contains(&field.as_str());
        assert_eq!(declared["writer"] == "computed", computed, "{declared}");
        assert_eq!(
            declared["required"] == true,
            field == "statement",
            "only the statement is required: {declared}"
        );
    }
}

/// The status a phase's ladder and the verdict compute: pending while its start phase is not complete, which is the
/// kind's own pending state, so a pending invariant's `proof` and `items` are not checked.
#[test]
fn an_invariant_is_pending_until_its_phase_completes() {
    let status = field_of("invariant", "status");
    assert_eq!(
        status["enum_values"],
        serde_json::json!([
            "pending",
            "unverified",
            "stale",
            "holds",
            "violated",
            "uncertain"
        ]),
        "{status}"
    );
    let invariant = kind("invariant");
    assert_eq!(invariant["complete_when"]["field"], "status", "{invariant}");
    assert_eq!(
        invariant["complete_when"]["values"],
        serde_json::json!(["unverified", "stale", "holds", "violated", "uncertain"]),
        "{invariant}"
    );
    assert_eq!(
        kind("phase")["complete_when"]["values"],
        serde_json::json!(["executed", "needs_review", "complete"]),
        "an invariant is in force once its phase is complete in the phase's own sense"
    );
}

fn kind(name: &str) -> serde_json::Value {
    describe("entities")["items"]
        .as_array()
        .and_then(|k| k.iter().find(|k| k["name"] == name))
        .cloned()
        .unwrap_or_else(|| panic!("no kind {name}"))
}

#[test]
fn an_invariant_is_in_the_project_scope() {
    let invariant = kind("invariant");
    assert!(invariant["scoped_by"].is_null(), "{invariant}");
}
