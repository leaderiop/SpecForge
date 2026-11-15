//! What `@gsd/core` declares for the requirement's fields beyond GSD (`spec/project.md`), pinned through the SDK's
//! runtime-free `MockHost`.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{field_of, fields_of};

fn member_of(field: &serde_json::Value, member: &str) -> serde_json::Value {
    field["members"]
        .as_array()
        .and_then(|m| m.iter().find(|m| m["name"] == member))
        .cloned()
        .unwrap_or_else(|| panic!("no member {member} in {field}"))
}

/// `metric` is one optional record: `name`, a `threshold` number with no bounds (a latency is positive, a drift may
/// be negative: `direction` carries the sense), its `unit` when it has one (a count has none), and a `direction`.
#[test]
fn the_requirement_metric_is_a_record_with_a_number_threshold() {
    let metric = field_of("requirement", "metric");
    assert_eq!(metric["field_type"], "block", "{metric}");
    assert_ne!(metric["required"], true, "{metric}");
    let members: Vec<&str> = metric["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|m| m["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(members, ["name", "threshold", "unit", "direction"]);
    for (member, field_type, required) in [
        ("name", "string", true),
        ("threshold", "number", true),
        ("unit", "string", false),
        ("direction", "enum", true),
    ] {
        let declared = member_of(&metric, member);
        assert_eq!(declared["field_type"], field_type, "{declared}");
        assert_eq!(declared["required"] == true, required, "{declared}");
    }
    let threshold = member_of(&metric, "threshold");
    assert!(
        threshold["minimum"].is_null() && threshold["maximum"].is_null(),
        "{threshold}"
    );
    assert_eq!(
        member_of(&metric, "direction")["enum_values"],
        serde_json::json!(["at_most", "at_least", "exactly"])
    );
}

/// The quality fields are optional, typed and authored: none required, write-once or computed.
#[test]
fn the_requirement_quality_fields_are_optional() {
    for (field, field_type) in [
        ("category", "enum"),
        ("priority", "enum"),
        ("metric", "block"),
        ("source", "string_list"),
        ("tags", "string_list"),
    ] {
        let declared = field_of("requirement", field);
        assert_eq!(declared["field_type"], field_type, "{declared}");
        assert_ne!(declared["required"], true, "{declared}");
        assert_ne!(declared["immutable"], true, "{declared}");
        assert!(declared["writer"].is_null(), "{declared}");
    }
    assert_eq!(
        field_of("requirement", "category")["enum_values"],
        serde_json::json!([
            "functional",
            "performance",
            "compatibility",
            "usability",
            "reliability",
            "security",
            "maintainability",
            "portability",
            "safety",
            "regulatory"
        ])
    );
    // One MoSCoW list with the journey's.
    assert_eq!(
        field_of("requirement", "priority")["enum_values"],
        field_of("journey", "priority")["enum_values"]
    );
    assert_eq!(
        field_of("requirement", "priority")["enum_values"],
        serde_json::json!(["must", "should", "could"])
    );
}

/// Each relation names its reverse (ADR 0354), so the other side reads it back by that name and W162 stays silent; no
/// field or reverse here takes `risks`, the reverse of `risk.affects` on a requirement.
#[test]
fn the_requirement_relations_name_their_reverse() {
    for (field, target, reverse) in [
        ("personas", "persona", "requirements"),
        ("constrains", "requirement", "constrained_by"),
        ("protects", "invariant", "protected_by"),
    ] {
        let declared = field_of("requirement", field);
        assert_eq!(declared["field_type"], "reference_list", "{declared}");
        assert_eq!(declared["target_kind"], target, "{declared}");
        assert_eq!(declared["inverse_of"], reverse, "{declared}");
        assert_ne!(declared["required"], true, "{declared}");
        // The target declares no field of the reverse's name: the reverse is read back, never written.
        assert!(
            !fields_of(target).contains(&reverse.to_string()),
            "{target}: {reverse}"
        );
    }
    assert!(!fields_of("requirement").contains(&"risks".to_string()));
    let reverses: Vec<serde_json::Value> = fields_of("requirement")
        .iter()
        .map(|f| field_of("requirement", f)["inverse_of"].clone())
        .collect();
    assert!(
        !reverses.contains(&serde_json::json!("risks")),
        "{reverses:?}"
    );
}
