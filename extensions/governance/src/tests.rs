use specforge_extension_sdk::FieldDescriptor;
use specforge_test::prelude::*;

/// The declared field `name` of `kind`.
fn declared(kind: &str, name: &str) -> FieldDescriptor {
    crate::specforge_extension_build()
        .declaration()
        .entities
        .into_iter()
        .find(|k| k.keyword.as_deref() == Some(kind))
        .unwrap_or_else(|| panic!("{kind} is not declared"))
        .fields
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{kind}.{name} is not declared"))
}

/// The fields that take a closed set of
/// values are enums of it, with no default, and a value outside it is the
/// core's E061, so the extension declares no rule of its own.
#[specforge_test(
    behavior = "ge_declare_vocabularies",
    verify = "decision status, constraint priority and every failure mode rating are declared enums of their vocabularies"
)]
fn governance_declares_its_rating_and_status_vocabularies() {
    const SEVERITY: &[&str] = &["critical", "high", "medium", "low"];
    const OCCURRENCE: &[&str] = &["certain", "likely", "occasional", "unlikely", "rare"];
    const DETECTION: &[&str] = &["certain", "likely", "moderate", "unlikely", "undetectable"];
    for (kind, field, values) in [
        (
            "decision",
            "status",
            &["proposed", "accepted", "deprecated", "superseded"][..],
        ),
        ("constraint", "priority", SEVERITY),
        ("failure_mode", "severity", SEVERITY),
        ("failure_mode", "post_severity", SEVERITY),
        ("failure_mode", "occurrence", OCCURRENCE),
        ("failure_mode", "post_occurrence", OCCURRENCE),
        ("failure_mode", "detection", DETECTION),
        ("failure_mode", "post_detection", DETECTION),
    ] {
        let declared = declared(kind, field);
        assert_eq!(declared.field_type, "enum", "{kind}.{field}");
        assert_eq!(declared.enum_values, values, "{kind}.{field}");
        assert_eq!(declared.default_value, None, "{kind}.{field}");
    }
    let rules = crate::specforge_extension_build()
        .declaration()
        .validation_rules;
    let codes: Vec<&str> = rules.iter().map(|r| r.code.as_str()).collect();
    assert_eq!(codes, ["W003"], "no value rule is declared");
}

/// A constraint's category is a tag: one word, written bare (ADR 0147).
#[test]
fn the_constraint_category_is_a_word() {
    assert_eq!(declared("constraint", "category").field_type, "identifier");
}

/// A decision's date is one calendar day (ADR 0234).
#[test]
fn a_decisions_date_is_a_date() {
    assert_eq!(declared("decision", "date").field_type, "date");
}

/// Governance declares the rules a project keeps (ADR 0280).
#[specforge_test(
    behavior = "ge_declare_manifest",
    verify = "the declaration declares invariant with its guarantee and its risk levels, and W003 over it"
)]
fn governance_declares_invariant_its_risk_and_w003() {
    let guarantee = declared("invariant", "guarantee");
    assert!(guarantee.required, "invariant.guarantee is required");
    let risk = declared("invariant", "risk");
    assert_eq!(risk.field_type, "enum");
    assert_eq!(risk.enum_values, ["critical", "high", "medium", "low"]);
    let rules = crate::specforge_extension_build()
        .declaration()
        .validation_rules;
    let w003 = rules
        .iter()
        .find(|r| r.code == "W003")
        .expect("W003 is declared");
    assert_eq!(w003.target_kind.as_deref(), Some("invariant"));
    assert_eq!(w003.check, "no_incoming_edges");
    assert_eq!(w003.edge_type, None, "any reference silences W003");
}
