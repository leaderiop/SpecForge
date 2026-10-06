//! `parse_validation_rule_pattern`: a declared rule becomes a typed rule,
//! or W112 when it cannot work as declared.

use specforge_common::Severity;
use specforge_protocol_types::{CheckKind, ValidationRuleDescriptor, ValidationSeverity};
use specforge_test_macros::test as spec;

use super::{constraint, entity, one, rule, rules};

#[spec(
    behavior = "parse_validation_rule_pattern",
    verify = "parses no_incoming_edges pattern from manifest"
)]
fn parses_no_incoming_edges_pattern_from_manifest() {
    let built = one(rule("W100", "no_incoming_edges"));
    assert!(built.diagnostics.is_empty(), "{:?}", built.diagnostics);
    let rule = built.rules.iter().next().unwrap();
    assert_eq!(rule.check_kind(), CheckKind::NoIncomingEdges);
    assert_eq!(rule.code(), "W100");
}

#[spec(
    behavior = "parse_validation_rule_pattern",
    verify = "parses missing_field_when_flag_set pattern from manifest"
)]
fn parses_missing_field_when_flag_set_pattern_from_manifest() {
    let mut declared = rule("W101", "missing_field_when_flag_set");
    declared.field = Some("contract".to_string());
    let built = one(declared);
    let rule = built.rules.iter().next().unwrap();
    assert_eq!(rule.check_kind(), CheckKind::MissingFieldWhenFlagSet);
    assert_eq!(rule.describe()["field"], "contract");
}

#[spec(
    behavior = "parse_validation_rule_pattern",
    verify = "unrecognized pattern kind produces warning"
)]
fn unrecognized_pattern_kind_produces_warning() {
    let built = one(rule("W102", "invalid_check_kind"));
    assert!(built.rules.is_empty());
    let w112 = built.coded("W112");
    assert_eq!(w112.len(), 1, "{:?}", built.diagnostics);
    assert_eq!(w112[0].severity, Severity::Warning);
    assert_eq!(
        w112[0].message,
        "extension '@test': unrecognized validation pattern kind 'invalid_check_kind'"
    );
}

#[spec(
    behavior = "parse_validation_rule_pattern",
    verify = "all required fields validated on each rule"
)]
fn misconfigured_one_of_with_empty_values_produces_warning() {
    let mut declared = rule("W107", "field_value_constraint");
    declared.field = Some("status".to_string());
    declared.constraint = Some(constraint("one_of", None, &[]));

    let built = one(declared);

    // The dead rule must not reach execution.
    assert!(built.rules.is_empty());
    assert_eq!(built.diagnostics.len(), 1);
    assert_eq!(
        built.diagnostics[0].message,
        "extension '@test': rule 'W107': one_of constraint has an empty values list — every field value would be flagged as a violation — the rule can never fire and was not registered"
    );
}

#[spec(
    behavior = "parse_validation_rule_pattern",
    verify = "all required fields validated on each rule"
)]
fn field_requiring_check_without_field_produces_warning() {
    for check in [
        "missing_field_when_flag_set",
        "missing_required_field",
        "file_exists",
    ] {
        let built = one(rule("W108", check));
        assert!(built.rules.is_empty(), "{check}");
        assert_eq!(
            built.coded("W112")[0].message,
            format!(
                "extension '@test': rule 'W108': check '{check}' requires a field but none is set — the rule can never fire and was not registered"
            )
        );
    }
}

#[spec(
    behavior = "parse_validation_rule_pattern",
    verify = "parses field_value_constraint pattern from manifest"
)]
fn valid_one_of_rule_still_parses() {
    let mut declared = rule("W109", "field_value_constraint");
    declared.field = Some("status".to_string());
    declared.constraint = Some(constraint("one_of", None, &["draft", "active"]));
    let built = one(declared);
    assert!(built.diagnostics.is_empty(), "{:?}", built.diagnostics);
    let rule = built.rules.iter().next().unwrap();
    assert_eq!(rule.check_kind(), CheckKind::FieldValueConstraint);
    assert_eq!(
        rule.describe()["constraint"],
        serde_json::json!({ "kind": "one_of", "pattern": null, "values": ["draft", "active"] })
    );
}

#[test]
fn a_rule_keeps_its_declared_head() {
    let built = one(ValidationRuleDescriptor {
        code: "W100".to_string(),
        severity: ValidationSeverity::Error,
        message_template: "test {id}".to_string(),
        check: "no_incoming_edges".to_string(),
        ..Default::default()
    });
    let rule = built.rules.iter().next().unwrap();
    assert_eq!(rule.code(), "W100");
    assert_eq!(rule.severity(), Severity::Error);
    assert_eq!(rule.describe()["message_template"], "test {id}");
    assert_eq!(rule.check_kind(), CheckKind::NoIncomingEdges);
    assert_eq!(rule.target_kind(), None);
}

#[spec(
    behavior = "parse_validation_rule_pattern",
    verify = "Parse Validation Rule Pattern: validation rule parsing holds — manifest_rules_available, patterns_parsed, unrecognized_warned"
)]
fn parse_validation_rule_pattern_contract() {
    // requires: manifest rules available
    let built = rules(vec![
        rule("W100", "no_incoming_edges"),
        rule("W200", "invalid_kind"),
    ]);
    // ensures: valid patterns parsed
    assert_eq!(built.codes(), ["W100"]);
    // ensures: unrecognized warned, naming the extension
    let w112 = built.coded("W112");
    assert_eq!(w112.len(), 1);
    assert!(w112[0].message.contains("'@test'"), "{}", w112[0].message);
}

// C14: a malformed regex is rejected when the rule is built and never runs.
#[test]
fn an_invalid_regex_is_w112_and_not_registered() {
    let mut declared = rule("W095", "field_value_constraint");
    declared.field = Some("version".to_string());
    declared.constraint = Some(constraint("matches", Some("(unclosed"), &[]));
    let built = one(declared);
    assert!(built.rules.is_empty());
    let w112 = built.coded("W112");
    assert_eq!(w112.len(), 1);
    assert!(
        w112[0]
            .message
            .starts_with("extension '@test': rule 'W095': invalid regex pattern '(unclosed': "),
        "{}",
        w112[0].message
    );
}

// C6-12: a matches constraint without a pattern can never check anything;
// an unrecognized constraint kind never matches.
#[test]
fn a_constraint_that_can_never_match_is_w112() {
    for (kind, pattern, why) in [
        (
            "matches",
            None,
            "matches constraint has no pattern — no value can ever be checked",
        ),
        (
            "equals",
            None,
            "unknown constraint kind 'equals' for check 'field_value_constraint' (expected non_empty, one_of, or matches)",
        ),
        (
            "when_field_equals",
            Some("status"),
            "unknown constraint kind 'when_field_equals' for check 'field_value_constraint' (expected non_empty, one_of, or matches)",
        ),
    ] {
        let mut declared = rule("W105", "field_value_constraint");
        declared.field = Some("version".to_string());
        declared.constraint = Some(constraint(kind, pattern, &["active"]));
        let built = one(declared);
        assert!(built.rules.is_empty(), "{kind}");
        assert_eq!(
            built.coded("W112")[0].message,
            format!(
                "extension '@test': rule 'W105': {why} — the rule can never fire and was not registered"
            )
        );
    }
    let mut declared = rule("W106", "field_value_constraint");
    declared.field = Some("status".to_string());
    let built = one(declared);
    assert!(
        built.coded("W112")[0]
            .message
            .contains("check 'field_value_constraint' requires a constraint but none is set")
    );
}

// C6-12: a conditional rule needs a condition field and values.
#[test]
fn a_conditional_rule_that_can_never_trigger_is_w112() {
    for (declared_constraint, why) in [
        (
            None,
            "check 'conditional_field_required' requires a constraint but none is set",
        ),
        (
            Some(constraint("when_field_equals", None, &["deferred"])),
            "conditional_field_required requires constraint.pattern (the condition field) — without it the condition can never be met",
        ),
        (
            Some(constraint("when_field_equals", Some("status"), &[])),
            "conditional_field_required has an empty condition values list — the condition can never be met",
        ),
    ] {
        let mut declared = rule("I059", "conditional_field_required");
        declared.field = Some("reason".to_string());
        declared.constraint = declared_constraint;
        let built = one(declared);
        assert!(built.rules.is_empty());
        assert_eq!(
            built.coded("W112")[0].message,
            format!(
                "extension '@test': rule 'I059': {why} — the rule can never fire and was not registered"
            )
        );
    }
}

#[test]
fn a_conditional_rule_reads_the_condition_field_and_values() {
    let mut declared = rule("I059", "conditional_field_required");
    declared.target_kind = Some("feature".to_string());
    declared.field = Some("reason".to_string());
    declared.constraint = Some(constraint(
        "when_field_equals",
        Some("status"),
        &["deferred"],
    ));
    let built = one(declared);
    let rule = built.rules.iter().next().unwrap();
    assert_eq!(rule.check_kind(), CheckKind::ConditionalFieldRequired);
    assert_eq!(
        rule.describe()["constraint"],
        serde_json::json!({ "kind": "when_field_equals", "pattern": "status", "values": ["deferred"] })
    );
    // It runs: a deferred feature without a reason.
    let deferred = entity("f", "feature", 0, 0).with_field("status", "deferred");
    assert_eq!(super::check(&built, &[deferred]).len(), 1);
}

#[spec(
    behavior = "register_custom_validation_patterns",
    verify = "custom rule without a wasm_function produces warning and is not registered"
)]
fn a_custom_rule_without_a_wasm_function_is_w112() {
    let built = one(rule("E200", "custom"));
    assert!(built.rules.is_empty());
    assert_eq!(
        built.coded("W112")[0].message,
        "extension '@test': rule 'E200': check 'custom' requires a wasm_function but none is set — the rule can never fire and was not registered"
    );
}

#[spec(
    behavior = "register_custom_validation_patterns",
    verify = "custom pattern registered with wasm_function reference"
)]
fn a_custom_rule_is_registered_with_its_wasm_function() {
    let mut declared = rule("E200", "custom");
    declared.wasm_function = Some("validate_custom".to_string());
    let built = one(declared);
    let rule = built.rules.iter().next().unwrap();
    assert_eq!(rule.check_kind(), CheckKind::Custom);
    assert_eq!(rule.describe()["wasm_function"], "validate_custom");
    assert_eq!(rule.origin().name(), "@test");
}
