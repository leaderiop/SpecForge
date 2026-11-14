//! What `@gsd/core` declares for the product kinds (`spec/product.md`), pinned through the SDK's runtime-free
//! `MockHost`. The model doc's tables are read here, each row's field, type and flags, so the doc cannot drift from the
//! declaration.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{describe, field_of, fields_of};

const PRODUCT_MD: &str = include_str!("../spec/product.md");

/// The rows of the field table under `## <kind>` in `spec/product.md`: field, type, flags, meaning.
fn documented(kind: &str) -> Vec<Vec<String>> {
    let table = gsd_kit::markdown::tables(PRODUCT_MD)
        .into_iter()
        .find(|t| t.heading == kind)
        .unwrap_or_else(|| panic!("no table under `## {kind}`"));
    assert_eq!(
        table.header,
        ["Field", "Type", "Flags", "Meaning"],
        "{kind}"
    );
    table.rows
}

/// A declared field (or block member) as the Type column of `spec/product.md` writes it: `str`, `strs`, `→ kind`,
/// `→* kind`, `enum{a, b}`, and a block list `{member type [R], …}`.
fn written_type(field: &serde_json::Value) -> String {
    let target = || field["target_kind"].as_str().expect("a target kind");
    match field["field_type"].as_str().expect("a field type") {
        "string" => "str".to_string(),
        "string_list" => "strs".to_string(),
        "reference" => format!("→ {}", target()),
        "reference_list" => format!("→* {}", target()),
        "enum" => {
            let words: Vec<&str> = field["enum_values"]
                .as_array()
                .expect("enum values")
                .iter()
                .map(|w| w.as_str().expect("a word"))
                .collect();
            format!("enum{{{}}}", words.join(", "))
        }
        "block_list" => {
            let members: Vec<String> = field["members"]
                .as_array()
                .expect("members")
                .iter()
                .map(|m| {
                    let flags = written_flags(m);
                    let name = m["name"].as_str().expect("a name");
                    let written = format!("{name} {}", written_type(m));
                    if flags.is_empty() {
                        written
                    } else {
                        format!("{written} {flags}")
                    }
                })
                .collect();
            format!("{{{}}}", members.join(", "))
        }
        other => panic!("spec/product.md writes no `{other}` field yet: give it its notation here"),
    }
}

/// The Flags column: `R` for a required field, `C` for a computed one, empty for neither.
fn written_flags(field: &serde_json::Value) -> String {
    match (field["required"] == true, field["writer"] == "computed") {
        (true, true) => panic!("a field is required or computed, never both: {field}"),
        (true, false) => "R".to_string(),
        (false, true) => "C".to_string(),
        (false, false) => String::new(),
    }
}

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
    for kind in ["persona", "journey", "term"] {
        let rows = documented(kind);
        let names: Vec<String> = rows.iter().map(|row| row[0].clone()).collect();
        assert_eq!(fields_of(kind), names, "{kind}");
        for row in &rows {
            let field = field_of(kind, &row[0]);
            assert_eq!(written_type(&field), row[1], "{kind}.{}: Type", row[0]);
            assert_eq!(written_flags(&field), row[2], "{kind}.{}: Flags", row[0]);
        }
    }
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
