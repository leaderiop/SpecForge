//! What `@gsd/core` declares for the risk (`spec/governance.md`), pinned through the SDK's runtime-free `MockHost`.
//! The model doc's table is read here, each row's field, type and flags, so the doc cannot drift from the declaration.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{describe, field_of, fields_of, model_doc};

const GOVERNANCE_MD: &str = include_str!("../spec/governance.md");

const RATINGS: [&str; 3] = ["severity", "occurrence", "detection"];

fn member_of(field: &serde_json::Value, member: &str) -> serde_json::Value {
    field["members"]
        .as_array()
        .and_then(|m| m.iter().find(|m| m["name"] == member))
        .cloned()
        .unwrap_or_else(|| panic!("no member {member} in {field}"))
}

#[test]
fn the_risk_declares_the_fields_of_the_model() {
    assert_eq!(
        fields_of("risk"),
        [
            "cause",
            "effect",
            "severity",
            "occurrence",
            "detection",
            "post",
            "mitigated_by",
            "mitigation",
            "affects",
            "milestone",
            "tags",
            "description",
            "refs",
            "rpn",
            "post_rpn",
        ]
    );
    model_doc::assert_declared(GOVERNANCE_MD, &["risk"]);
}

#[test]
fn a_risk_is_in_the_project_scope() {
    let risk = describe("entities")["items"]
        .as_array()
        .and_then(|k| k.iter().find(|k| k["name"] == "risk"))
        .cloned()
        .expect("the risk kind");
    assert!(risk["scoped_by"].is_null(), "{risk}");
}

#[test]
fn a_risk_rates_severity_occurrence_and_detection_one_to_ten() {
    let post = field_of("risk", "post");
    assert_eq!(post["field_type"], "block", "{post}");
    assert_ne!(post["required"], true, "{post}");
    for rating in RATINGS {
        for (declared, required) in [
            (field_of("risk", rating), true),
            (member_of(&post, rating), false),
        ] {
            assert_eq!(declared["field_type"], "integer", "{declared}");
            assert_eq!(declared["minimum"], 1, "{declared}");
            assert_eq!(declared["maximum"], 10, "{declared}");
            assert_eq!(declared["required"] == true, required, "{declared}");
        }
    }
    for field in ["cause", "effect"] {
        assert_eq!(field_of("risk", field)["required"], true, "{field}");
    }
}

/// `mitigated_by` names no reverse: the kernel reads a reverse name through one target kind, and it has two.
#[test]
fn a_risk_names_its_mitigations_and_milestone() {
    let mitigated_by = field_of("risk", "mitigated_by");
    assert_eq!(
        mitigated_by["field_type"], "reference_list",
        "{mitigated_by}"
    );
    assert_eq!(
        mitigated_by["target_kinds"],
        serde_json::json!(["task", "truth"]),
        "{mitigated_by}"
    );
    assert!(mitigated_by["inverse_of"].is_null(), "{mitigated_by}");
    let affects = field_of("risk", "affects");
    assert_eq!(affects["target_kind"], "requirement", "{affects}");
    assert_eq!(affects["inverse_of"], "risks", "{affects}");
    let milestone = field_of("risk", "milestone");
    assert_eq!(milestone["field_type"], "reference", "{milestone}");
    assert_eq!(milestone["target_kind"], "milestone", "{milestone}");
    assert_eq!(milestone["inverse_of"], "risks", "{milestone}");
    assert!(
        !fields_of("risk").contains(&"invariants".to_string()),
        "the old failure mode's invariant link has one home, mitigated_by"
    );
}

#[test]
fn rpn_and_post_rpn_are_computed() {
    for field in ["rpn", "post_rpn"] {
        let declared = field_of("risk", field);
        assert_eq!(declared["writer"], "computed", "{declared}");
        assert_eq!(declared["field_type"], "integer", "{declared}");
    }
}
