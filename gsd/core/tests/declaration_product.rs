//! What `@gsd/core` declares for the product kinds (`spec/product.md`), pinned through the SDK's runtime-free
//! `MockHost`. The model doc's tables are read here, each row's field, type and flags, so the doc cannot drift from the
//! declaration.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{describe, field_of, fields_of, model_doc};

const PRODUCT_MD: &str = include_str!("../spec/product.md");

fn kind_of(kind: &str) -> serde_json::Value {
    describe("entities")["items"]
        .as_array()
        .and_then(|k| k.iter().find(|k| k["name"] == kind))
        .cloned()
        .unwrap_or_else(|| panic!("no kind {kind}"))
}

fn member_of(field: &serde_json::Value, member: &str) -> serde_json::Value {
    field["members"]
        .as_array()
        .and_then(|m| m.iter().find(|m| m["name"] == member))
        .cloned()
        .unwrap_or_else(|| panic!("no member {member} in {field}"))
}

#[test]
fn the_product_kinds_declare_the_fields_of_product_md() {
    model_doc::assert_declared(PRODUCT_MD, &["persona", "journey", "term"]);
}

#[test]
fn a_journey_lists_its_milestones_and_is_not_scoped() {
    assert!(
        kind_of("journey")["scoped_by"].is_null(),
        "{}",
        kind_of("journey")
    );
    let milestones = field_of("journey", "milestones");
    assert_eq!(milestones["field_type"], "reference_list", "{milestones}");
    assert_eq!(milestones["target_kind"], "milestone", "{milestones}");
    assert_eq!(milestones["inverse_of"], "journeys", "{milestones}");
    let persona = field_of("journey", "persona");
    assert_eq!(persona["field_type"], "reference", "{persona}");
    assert_eq!(persona["target_kind"], "persona", "{persona}");
    assert_eq!(persona["inverse_of"], "journeys", "{persona}");
    for (field, required) in [
        ("persona", true),
        ("steps", true),
        ("milestones", false),
        ("priority", false),
    ] {
        assert_eq!(
            field_of("journey", field)["required"] == true,
            required,
            "{field}"
        );
    }
    assert_eq!(
        field_of("journey", "priority")["enum_values"],
        serde_json::json!(["must", "should", "could"])
    );
}

#[test]
fn a_step_holds_an_action_and_an_optional_requirement() {
    let steps = field_of("journey", "steps");
    assert_eq!(steps["field_type"], "block_list", "{steps}");
    let action = member_of(&steps, "action");
    assert_eq!(action["field_type"], "string", "{action}");
    assert_eq!(action["required"], true, "{action}");
    let requirement = member_of(&steps, "requirement");
    assert_eq!(requirement["field_type"], "reference", "{requirement}");
    assert_eq!(requirement["target_kind"], "requirement", "{requirement}");
    assert_ne!(requirement["required"], true, "{requirement}");
}

#[test]
fn a_persona_has_its_context_its_five_step_scale_and_a_status() {
    assert!(kind_of("persona")["scoped_by"].is_null());
    assert_eq!(field_of("persona", "context")["required"], true);
    assert_eq!(
        field_of("persona", "technical_level")["enum_values"],
        serde_json::json!([
            "expert",
            "advanced",
            "intermediate",
            "beginner",
            "non_technical"
        ])
    );
    assert_eq!(
        field_of("persona", "status")["enum_values"],
        serde_json::json!(["active", "deprecated"])
    );
    for field in fields_of("persona") {
        assert_ne!(field_of("persona", &field)["writer"], "computed", "{field}");
        if field != "context" {
            assert_ne!(field_of("persona", &field)["required"], true, "{field}");
        }
    }
}

/// A pass sees no entity's title, so the word a term defines is its `name`, required with its definition.
#[test]
fn a_term_carries_its_word_because_a_pass_sees_no_title() {
    assert!(kind_of("term")["scoped_by"].is_null());
    for field in ["name", "definition"] {
        let declared = field_of("term", field);
        assert_eq!(declared["required"], true, "{declared}");
        assert_eq!(declared["field_type"], "string", "{declared}");
    }
    for field in ["avoid", "aliases", "related", "context", "tags"] {
        assert_ne!(field_of("term", field)["required"], true, "{field}");
    }
    let related = field_of("term", "related");
    assert_eq!(related["field_type"], "reference_list", "{related}");
    assert_eq!(related["target_kind"], "term", "{related}");
    assert!(related["inverse_of"].is_null(), "{related}");
    for field in ["avoid", "aliases"] {
        assert_eq!(
            field_of("term", field)["field_type"],
            "string_list",
            "{field}"
        );
    }
}
