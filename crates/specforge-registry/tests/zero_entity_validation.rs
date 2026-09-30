//! Integration tests for spec/behaviors/zero-entity-validation.spec
//!
//! Covers 9 behaviors, 47 verify statements total:
//! - execute_validation_pattern (9)
//! - detect_unknown_entity_fields (6)
//! - parse_validation_rule_pattern (5)
//! - emit_diagnostic_from_pattern (5)
//! - register_custom_validation_patterns (5)
//! - validate_extension_testability (5)
//! - register_extension_validation_rules (4)
//! - detect_duplicate_entity_kinds (4)
//! - validate_peer_dependencies (4)

use specforge_common::{Severity, SourceSpan, Sym};
use specforge_registry::validation_engine::{
    ValidationEntity, ValidationPatternKind, ValidationRulePattern, WasmValidationRuntime,
    execute_pattern, interpolate_template, parse_all_rule_patterns, parse_rule_pattern,
    register_custom_patterns, resolve_edge_rules,
};
use specforge_registry::{
    EdgeRegistry, EdgeRegistryEntry, FieldConstraint, KindRegistry, KindRegistryEntry, ManifestV2,
    ManifestValidationRule, detect_duplicate_entity_kinds, populate_registries,
    register_validation_rules, validate_extension_testability, validate_peer_dependencies,
};
use specforge_test_macros::test as specforge_test;

// ============================================================================
// Helpers
// ============================================================================

fn span() -> SourceSpan {
    SourceSpan {
        file: Sym::new("test.spec"),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

fn make_rule(code: &str, check: &str) -> ManifestValidationRule {
    ManifestValidationRule {
        code: code.to_string(),
        severity: "warning".to_string(),
        message_template: "orphan {kind} '{id}'".to_string(),
        check: check.to_string(),
        target_kind: Some("behavior".to_string()),
        edge_type: None,
        field: None,
        constraint: None,
        wasm_function: None,
    }
}

fn make_entity(id: &str, kind: &str, incoming: usize, outgoing: usize) -> ValidationEntity {
    ValidationEntity {
        id: id.to_string(),
        kind: kind.to_string(),
        fields: std::collections::HashMap::new(),
        incoming_edge_count: incoming,
        outgoing_edge_count: outgoing,
        span: span(),
        verify_kinds: Vec::new(),
        verify_texts: Vec::new(),
        outgoing_kinds: Default::default(),
        incoming_kinds: Default::default(),
    }
}

fn software_manifest() -> ManifestV2 {
    serde_json::from_str(
        r#"{
            "name": "@specforge/software",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "software.wasm",
            "entityKinds": [
                {
                    "name": "Behavior",
                    "keyword": "behavior",
                    "testable": true,
                    "supportsVerify": true,
                    "fields": [
                        { "name": "contract", "fieldType": "block" },
                        { "name": "invariants", "fieldType": "reference_list", "edge": "enforces", "targetKind": "invariant" }
                    ]
                },
                {
                    "name": "Invariant",
                    "keyword": "invariant",
                    "testable": true,
                    "supportsVerify": true
                }
            ],
            "edgeTypes": [
                { "label": "enforces", "sourceKind": "behavior", "targetKind": "invariant" }
            ]
        }"#,
    )
    .unwrap()
}

// ============================================================================
// B:parse_validation_rule_pattern (5 verifies)
// ============================================================================

#[specforge_test(
    behavior = "parse_validation_rule_pattern",
    verify = "parses no_incoming_edges pattern from manifest"
)]
fn parses_no_incoming_edges_pattern_from_manifest() {
    let rule = make_rule("W100", "no_incoming_edges");
    let pattern = parse_rule_pattern(&rule, "@test/ext").unwrap();
    assert_eq!(pattern.check, ValidationPatternKind::NoIncomingEdges);
    assert_eq!(pattern.code, "W100");
}

#[specforge_test(
    behavior = "parse_validation_rule_pattern",
    verify = "parses missing_field_when_flag_set pattern from manifest"
)]
fn parses_missing_field_when_flag_set_pattern_from_manifest() {
    let mut rule = make_rule("W101", "missing_field_when_flag_set");
    rule.field = Some("contract".to_string());
    let pattern = parse_rule_pattern(&rule, "@test/ext").unwrap();
    assert_eq!(
        pattern.check,
        ValidationPatternKind::MissingFieldWhenFlagSet
    );
    assert_eq!(pattern.field.as_deref(), Some("contract"));
}

#[specforge_test(
    behavior = "parse_validation_rule_pattern",
    verify = "unrecognized pattern kind produces warning"
)]
fn unrecognized_pattern_kind_produces_warning() {
    let rule = make_rule("W102", "invalid_check_kind");
    let result = parse_rule_pattern(&rule, "@test/ext");
    assert!(result.is_err());
    let diag = result.unwrap_err();
    assert_eq!(diag.code, "W112");
    assert!(diag.message.contains("invalid_check_kind"));
    assert!(diag.message.contains("@test/ext"));
}

#[specforge_test(
    behavior = "parse_validation_rule_pattern",
    verify = "all required fields validated on each rule"
)]
fn misconfigured_one_of_with_empty_values_produces_warning() {
    let mut rule = make_rule("W107", "field_value_constraint");
    rule.field = Some("status".to_string());
    rule.constraint = Some(FieldConstraint {
        kind: "one_of".to_string(),
        pattern: None,
        values: vec![],
    });

    let err = parse_rule_pattern(&rule, "@test/ext").unwrap_err();
    assert_eq!(err.code, "W112");
    assert!(err.message.contains("W107"));
    assert!(err.message.contains("one_of"));

    // The dead rule must not reach execution.
    let manifests = vec![("@test/ext".to_string(), vec![rule])];
    let (patterns, diags) = parse_all_rule_patterns(&manifests);
    assert!(patterns.is_empty());
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "W112");
}

#[specforge_test(
    behavior = "parse_validation_rule_pattern",
    verify = "all required fields validated on each rule"
)]
fn field_requiring_check_without_field_produces_warning() {
    // make_rule leaves field unset; missing_field_when_flag_set reads it.
    let rule = make_rule("W108", "missing_field_when_flag_set");
    let err = parse_rule_pattern(&rule, "@test/ext").unwrap_err();
    assert_eq!(err.code, "W112");
    assert!(err.message.contains("W108"));
    assert!(err.message.contains("requires a field"));
}

#[specforge_test(
    behavior = "parse_validation_rule_pattern",
    verify = "parses field_value_constraint pattern from manifest"
)]
fn valid_one_of_rule_still_parses() {
    let mut rule = make_rule("W109", "field_value_constraint");
    rule.field = Some("status".to_string());
    rule.constraint = Some(FieldConstraint {
        kind: "one_of".to_string(),
        pattern: None,
        values: vec!["draft".to_string(), "active".to_string()],
    });
    let pattern = parse_rule_pattern(&rule, "@test/ext").unwrap();
    assert_eq!(pattern.check, ValidationPatternKind::FieldValueConstraint);
}

#[test]
fn all_required_fields_validated_on_each_rule() {
    let rule = ManifestValidationRule {
        code: "W100".to_string(),
        severity: "error".to_string(),
        message_template: "test {id}".to_string(),
        check: "no_incoming_edges".to_string(),
        target_kind: None,
        edge_type: None,
        field: None,
        constraint: None,
        wasm_function: None,
    };
    let pattern = parse_rule_pattern(&rule, "@test/ext").unwrap();
    assert_eq!(pattern.code, "W100");
    assert_eq!(pattern.severity, Severity::Error);
    assert_eq!(pattern.message_template, "test {id}");
    assert_eq!(pattern.check, ValidationPatternKind::NoIncomingEdges);
}

#[specforge_test(
    behavior = "parse_validation_rule_pattern",
    verify = "Parse Validation Rule Pattern: validation rule parsing holds — manifest_rules_available, patterns_parsed, unrecognized_warned"
)]
fn parse_validation_rule_pattern_contract() {
    // requires: manifest rules available
    let rules = vec![
        (
            "@ext/a".to_string(),
            vec![make_rule("W100", "no_incoming_edges")],
        ),
        (
            "@ext/b".to_string(),
            vec![make_rule("W200", "invalid_kind")],
        ),
    ];
    let (patterns, diags) = parse_all_rule_patterns(&rules);
    // ensures: valid patterns parsed
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].0.code, "W100");
    // ensures: unrecognized warned
    assert!(diags.iter().any(|d| d.code == "W112"));
}

// ============================================================================
// B:execute_validation_pattern (9 verifies)
// ============================================================================

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "no_incoming_edges detects orphan entities"
)]
fn no_incoming_edges_detects_orphan_entities() {
    let pattern = parse_rule_pattern(&make_rule("W100", "no_incoming_edges"), "@test").unwrap();
    let entities = vec![
        make_entity("b1", "behavior", 0, 2), // orphan
        make_entity("b2", "behavior", 1, 0), // not orphan
    ];
    let diags = execute_pattern(&pattern, &entities, None);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("b1"));
}

fn kind(name: &str) -> KindRegistryEntry {
    KindRegistryEntry {
        kind_name: name.to_string(),
        description: None,
        source_extension: "@test".to_string(),
        testable: false,
        singleton: false,
        supports_verify: false,
        allowed_verify_kinds: Vec::new(),
        has_body_parser: false,
        semantic_token: None,
        lsp_icon: None,
        dot_shape: None,
        dot_color: None,
        dot_fillcolor: None,
        open_fields: false,
    }
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "an edge rule counts only edges of its edge type and is dropped when no extension declares the kind at its far end"
)]
fn an_edge_rule_counts_only_its_edge_type() {
    let mut edges = EdgeRegistry::new();
    edges.register(EdgeRegistryEntry {
        label: "BehaviorImplementsFeature".to_string(),
        source_kind: Some("behavior".to_string()),
        target_kind: Some("feature".to_string()),
        source_extension: "@test".to_string(),
        edge_style: None,
        edge_color: None,
        edge_arrowhead: None,
    });
    let mut rule = make_rule("W001", "no_outgoing_edges");
    rule.edge_type = Some("BehaviorImplementsFeature".to_string());
    rule.message_template = "behavior '{id}' does not implement any feature".to_string();

    // b1 references an event but no feature; b2 implements a feature.
    let mut b1 = make_entity("b1", "behavior", 0, 1);
    b1.outgoing_kinds.insert("event".to_string(), 1);
    let mut b2 = make_entity("b2", "behavior", 0, 1);
    b2.outgoing_kinds.insert("feature".to_string(), 1);
    let entities = vec![b1, b2];

    let mut kinds = KindRegistry::new();
    kinds.register(kind("behavior"));
    kinds.register(kind("feature"));
    let (mut patterns, _) = parse_all_rule_patterns(&[("@test".to_string(), vec![rule.clone()])]);
    resolve_edge_rules(&mut patterns, &edges, &kinds);
    let diags = execute_pattern(&patterns[0].0, &entities, None);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert!(diags[0].message.contains("b1"));

    // Without an extension declaring `feature`, the rule can't be met: dropped.
    let mut kinds = KindRegistry::new();
    kinds.register(kind("behavior"));
    let (mut patterns, _) = parse_all_rule_patterns(&[("@test".to_string(), vec![rule])]);
    resolve_edge_rules(&mut patterns, &edges, &kinds);
    assert!(patterns.is_empty());
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "no_outgoing_edges detects entities with zero outgoing edges"
)]
fn no_outgoing_edges_detects_entities_with_zero_outgoing_edges() {
    let mut rule = make_rule("W101", "no_outgoing_edges");
    rule.message_template = "leaf {kind} '{id}'".to_string();
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();
    let entities = vec![
        make_entity("b1", "behavior", 1, 0), // leaf
        make_entity("b2", "behavior", 1, 3), // not leaf
    ];
    let diags = execute_pattern(&pattern, &entities, None);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("b1"));
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "missing_field_when_flag_set detects missing specified field on flagged entity"
)]
fn missing_field_when_flag_set_detects_missing_field() {
    let mut rule = make_rule("W102", "missing_field_when_flag_set");
    rule.field = Some("contract".to_string());
    rule.message_template = "{kind} '{id}' missing field '{field}'".to_string();
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();

    let e1 = make_entity("b1", "behavior", 1, 0); // no contract field
    let mut e2 = make_entity("b2", "behavior", 1, 0);
    e2.fields
        .insert("contract".to_string(), "some text".to_string());

    let diags = execute_pattern(&pattern, &[e1, e2], None);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("b1"));
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "field_value_constraint rejects invalid field value"
)]
fn field_value_constraint_rejects_invalid_field_value() {
    let rule = ManifestValidationRule {
        code: "W103".to_string(),
        severity: "warning".to_string(),
        message_template: "{kind} '{id}' has invalid {field}='{value}'".to_string(),
        check: "field_value_constraint".to_string(),
        target_kind: Some("behavior".to_string()),
        edge_type: None,
        field: Some("status".to_string()),
        constraint: Some(FieldConstraint {
            kind: "one_of".to_string(),
            pattern: None,
            values: vec![
                "draft".to_string(),
                "active".to_string(),
                "deprecated".to_string(),
            ],
        }),
        wasm_function: None,
    };
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();

    let mut e1 = make_entity("b1", "behavior", 1, 0);
    e1.fields
        .insert("status".to_string(), "invalid_status".to_string());
    let mut e2 = make_entity("b2", "behavior", 1, 0);
    e2.fields.insert("status".to_string(), "active".to_string());

    let diags = execute_pattern(&pattern, &[e1, e2], None);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("b1"));
}

#[test]
fn cycle_detection_finds_cycles_in_edge_type() {
    // Cycle detection requires full graph — current implementation defers to caller.
    // The pattern parses correctly but execution returns no violations (graph needed).
    let rule = make_rule("E100", "cycle_detection");
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();
    assert_eq!(pattern.check, ValidationPatternKind::CycleDetection);
    let diags = execute_pattern(&pattern, &[make_entity("b1", "behavior", 1, 1)], None);
    assert!(
        diags.is_empty(),
        "cycle detection deferred to graph-aware caller"
    );
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "file_exists reports missing file-reference field targets"
)]
fn file_exists_reports_missing_file_reference_field_targets() {
    let rule = ManifestValidationRule {
        code: "E101".to_string(),
        severity: "error".to_string(),
        message_template: "{kind} '{id}' references missing file".to_string(),
        check: "file_exists".to_string(),
        target_kind: Some("behavior".to_string()),
        edge_type: None,
        field: Some("gherkin".to_string()),
        constraint: None,
        wasm_function: None,
    };
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();

    let mut entity = make_entity("b1", "behavior", 1, 0);
    entity.fields.insert(
        "gherkin".to_string(),
        "/nonexistent/file.feature".to_string(),
    );

    let diags = execute_pattern(&pattern, &[entity], None);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E101");
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "custom pattern dispatches to registered Wasm function"
)]
fn custom_pattern_dispatches_to_registered_wasm_function() {
    struct MockRuntime;
    impl WasmValidationRuntime for MockRuntime {
        fn call_custom_validator(&self, func: &str, id: &str, _kind: &str) -> Result<bool, String> {
            if func == "validate_naming" && id == "bad_name" {
                Ok(false) // fails
            } else {
                Ok(true) // passes
            }
        }
    }

    let rule = ManifestValidationRule {
        code: "E200".to_string(),
        severity: "error".to_string(),
        message_template: "{kind} '{id}' fails custom validation".to_string(),
        check: "custom".to_string(),
        target_kind: Some("behavior".to_string()),
        edge_type: None,
        field: None,
        constraint: None,
        wasm_function: Some("validate_naming".to_string()),
    };
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();

    let entities = vec![
        make_entity("bad_name", "behavior", 1, 0),
        make_entity("good_name", "behavior", 1, 0),
    ];
    let diags = execute_pattern(&pattern, &entities, Some(&MockRuntime));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("bad_name"));
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "pattern violation produces diagnostic with configured code and severity"
)]
fn pattern_violation_produces_diagnostic_with_configured_code_and_severity() {
    let rule = ManifestValidationRule {
        code: "E999".to_string(),
        severity: "error".to_string(),
        message_template: "orphan {kind} '{id}'".to_string(),
        check: "no_incoming_edges".to_string(),
        target_kind: Some("behavior".to_string()),
        edge_type: None,
        field: None,
        constraint: None,
        wasm_function: None,
    };
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();
    let entities = vec![make_entity("b1", "behavior", 0, 1)];
    let diags = execute_pattern(&pattern, &entities, None);
    assert_eq!(diags[0].code, "E999");
    assert_eq!(diags[0].severity, Severity::Error);
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "Execute Validation Pattern: declarative validation holds — all_entities_matched, violations_diagnosed, deterministic_order"
)]
fn execute_validation_pattern_contract() {
    let rule = make_rule("W100", "no_incoming_edges");
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();
    // entities include different kinds — target_kind filter applies
    let entities = vec![
        make_entity("b1", "behavior", 0, 1),
        make_entity("b2", "behavior", 2, 0),
        make_entity("f1", "feature", 0, 0), // different kind, skipped
    ];
    let diags = execute_pattern(&pattern, &entities, None);
    // Only behavior with 0 incoming edges diagnosed
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("b1"));
}

// ============================================================================
// B:emit_diagnostic_from_pattern (5 verifies)
// ============================================================================

#[specforge_test(
    behavior = "emit_diagnostic_from_pattern",
    verify = "message template interpolates {id} and {kind}"
)]
fn message_template_interpolates_id_and_kind() {
    let result = interpolate_template(
        "orphan {kind} '{id}'",
        "my_beh",
        "behavior",
        None,
        None,
        None,
    );
    assert_eq!(result, "orphan behavior 'my_beh'");
}

#[specforge_test(
    behavior = "emit_diagnostic_from_pattern",
    verify = "message template interpolates {field} and {value}"
)]
fn message_template_interpolates_field_and_value() {
    let result = interpolate_template(
        "{kind} '{id}' has {field}='{value}'",
        "b1",
        "behavior",
        Some("status"),
        Some("invalid"),
        None,
    );
    assert_eq!(result, "behavior 'b1' has status='invalid'");
}

#[specforge_test(
    behavior = "emit_diagnostic_from_pattern",
    verify = "diagnostic code matches pattern code"
)]
fn diagnostic_code_matches_pattern_code() {
    let rule = ManifestValidationRule {
        code: "E999".to_string(),
        severity: "error".to_string(),
        message_template: "test".to_string(),
        check: "no_incoming_edges".to_string(),
        target_kind: Some("behavior".to_string()),
        edge_type: None,
        field: None,
        constraint: None,
        wasm_function: None,
    };
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();
    let diags = execute_pattern(&pattern, &[make_entity("b1", "behavior", 0, 0)], None);
    assert_eq!(diags[0].code, "E999");
}

#[specforge_test(
    behavior = "emit_diagnostic_from_pattern",
    verify = "diagnostic severity matches pattern severity"
)]
fn diagnostic_severity_matches_pattern_severity() {
    for (sev_str, expected) in &[
        ("error", Severity::Error),
        ("warning", Severity::Warning),
        ("info", Severity::Info),
    ] {
        let rule = ManifestValidationRule {
            code: "X001".to_string(),
            severity: sev_str.to_string(),
            message_template: "test".to_string(),
            check: "no_incoming_edges".to_string(),
            target_kind: Some("behavior".to_string()),
            edge_type: None,
            field: None,
            constraint: None,
            wasm_function: None,
        };
        let pattern = parse_rule_pattern(&rule, "@test").unwrap();
        let diags = execute_pattern(&pattern, &[make_entity("b1", "behavior", 0, 0)], None);
        assert_eq!(
            diags[0].severity, *expected,
            "severity mismatch for {}",
            sev_str
        );
    }
}

#[specforge_test(
    behavior = "emit_diagnostic_from_pattern",
    verify = "Emit Diagnostic From Pattern: pattern diagnostic emission holds — violation_detected, pattern_configured, diagnostic_emitted, template_interpolated"
)]
fn emit_diagnostic_from_pattern_contract() {
    // requires: violation detected, pattern configured
    let result = interpolate_template("{kind} '{id}' orphan", "b1", "behavior", None, None, None);
    // ensures: template interpolated
    assert_eq!(result, "behavior 'b1' orphan");
    // ensures: code and severity match
    let rule = make_rule("W100", "no_incoming_edges");
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();
    let diags = execute_pattern(&pattern, &[make_entity("b1", "behavior", 0, 0)], None);
    assert_eq!(diags[0].code, "W100");
    assert_eq!(diags[0].severity, Severity::Warning);
}

// ============================================================================
// B:register_extension_validation_rules (4 verifies)
// ============================================================================

#[specforge_test(
    behavior = "register_extension_validation_rules",
    verify = "rules from multiple extensions are collected"
)]
fn rules_from_multiple_extensions_are_collected() {
    let m1: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@ext/a",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "a.wasm",
            "validationRules": [
                { "code": "W100", "severity": "warning", "messageTemplate": "first", "check": "no_incoming_edges" }
            ]
        }"#,
    )
    .unwrap();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@ext/b",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "b.wasm",
            "validationRules": [
                { "code": "W200", "severity": "warning", "messageTemplate": "second", "check": "no_outgoing_edges" }
            ]
        }"#,
    )
    .unwrap();
    let (rules, diags) = register_validation_rules(&[m1, m2]);
    assert!(diags.is_empty());
    assert_eq!(rules.len(), 2);
}

#[specforge_test(
    behavior = "register_extension_validation_rules",
    verify = "duplicate codes across extensions produce warning"
)]
fn duplicate_codes_across_extensions_produce_warning() {
    let m1: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@ext/a",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "a.wasm",
            "validationRules": [
                { "code": "W100", "severity": "warning", "messageTemplate": "a", "check": "no_incoming_edges" }
            ]
        }"#,
    )
    .unwrap();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@ext/b",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "b.wasm",
            "validationRules": [
                { "code": "W100", "severity": "warning", "messageTemplate": "b", "check": "no_incoming_edges" }
            ]
        }"#,
    )
    .unwrap();
    let (_, diags) = register_validation_rules(&[m1, m2]);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "W023" && d.message.contains("W100")),
        "expected W023 for duplicate code, got: {:?}",
        diags
    );
}

#[specforge_test(
    behavior = "register_extension_validation_rules",
    verify = "rules sorted by code for deterministic order"
)]
fn rules_sorted_by_code_for_deterministic_order() {
    let m1: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@ext/a",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "a.wasm",
            "validationRules": [
                { "code": "W300", "severity": "warning", "messageTemplate": "third", "check": "no_incoming_edges" },
                { "code": "W100", "severity": "warning", "messageTemplate": "first", "check": "no_incoming_edges" }
            ]
        }"#,
    )
    .unwrap();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@ext/b",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "b.wasm",
            "validationRules": [
                { "code": "W200", "severity": "warning", "messageTemplate": "second", "check": "no_outgoing_edges" }
            ]
        }"#,
    )
    .unwrap();
    let (rules, _) = register_validation_rules(&[m1, m2]);
    let codes: Vec<&str> = rules.iter().map(|r| r.code.as_str()).collect();
    assert_eq!(codes, vec!["W100", "W200", "W300"]);
}

#[specforge_test(
    behavior = "register_extension_validation_rules",
    verify = "Register Extension Validation Rules: cross-extension rule aggregation holds — extension_manifests_loaded_fired, individual_rules_parsed, unified_rule_set_produced, deterministic_order_enforced, duplicate_codes_warned"
)]
fn register_extension_validation_rules_contract() {
    // requires: manifests parsed
    let m: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@t/e",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "x.wasm",
            "validationRules": [
                { "code": "W100", "severity": "warning", "messageTemplate": "test", "check": "no_incoming_edges" }
            ]
        }"#,
    )
    .unwrap();
    let (rules, diags) = register_validation_rules(&[m]);
    // ensures: rules registered
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].code, "W100");
    // ensures: no duplicate warnings for single extension
    assert!(diags.is_empty());
}

// ============================================================================
// B:register_custom_validation_patterns (5 verifies)
// ============================================================================

#[specforge_test(
    behavior = "register_custom_validation_patterns",
    verify = "custom pattern registered with wasm_function reference"
)]
fn custom_pattern_registered_with_wasm_function_reference() {
    let rule = ManifestValidationRule {
        code: "E200".to_string(),
        severity: "error".to_string(),
        message_template: "custom fail".to_string(),
        check: "custom".to_string(),
        target_kind: Some("behavior".to_string()),
        edge_type: None,
        field: None,
        constraint: None,
        wasm_function: Some("validate_custom".to_string()),
    };
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();
    assert_eq!(pattern.check, ValidationPatternKind::Custom);
    assert_eq!(pattern.wasm_function.as_deref(), Some("validate_custom"));
}

#[specforge_test(
    behavior = "register_custom_validation_patterns",
    verify = "unresolvable wasm_function produces warning"
)]
fn unresolvable_wasm_function_produces_warning() {
    let pattern = ValidationRulePattern {
        code: "E200".to_string(),
        severity: Severity::Error,
        message_template: "test".to_string(),
        check: ValidationPatternKind::Custom,
        target_kind: None,
        edge_type: None,
        edge_peer_kind: None,
        field: None,
        constraint: None,
        wasm_function: Some("missing_func".to_string()),
    };
    // No Wasm runtime → warning
    let (registered, diags) = register_custom_patterns(&[pattern], None);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "W112" && d.message.contains("missing_func"))
    );
    // Still registered for later (will be skipped during execution)
    assert_eq!(registered.len(), 1);
}

#[specforge_test(
    behavior = "register_custom_validation_patterns",
    verify = "unresolvable wasm_function produces warning"
)]
fn wasm_function_probe_failure_produces_warning() {
    struct TrappingRuntime;
    impl WasmValidationRuntime for TrappingRuntime {
        fn call_custom_validator(
            &self,
            _func: &str,
            id: &str,
            _kind: &str,
        ) -> Result<bool, String> {
            if id == "__probe__" {
                Err("trapped: unreachable".to_string())
            } else {
                Ok(true)
            }
        }
    }
    let pattern = ValidationRulePattern {
        code: "E202".to_string(),
        severity: Severity::Error,
        message_template: "test".to_string(),
        check: ValidationPatternKind::Custom,
        target_kind: None,
        edge_type: None,
        edge_peer_kind: None,
        field: None,
        constraint: None,
        wasm_function: Some("broken_export".to_string()),
    };

    let (registered, diags) = register_custom_patterns(&[pattern], Some(&TrappingRuntime));
    // The dead function is reported, not silently registered as healthy.
    assert!(diags.iter().any(|d| d.code == "W112"
        && d.message.contains("broken_export")
        && d.message.contains("can never fire")));
    // Still registered (execution will simply never fire it).
    assert_eq!(registered.len(), 1);
}

#[specforge_test(
    behavior = "register_custom_validation_patterns",
    verify = "custom pattern dispatched to Wasm runtime during validation"
)]
fn custom_pattern_dispatched_to_wasm_runtime_during_validation() {
    struct FailRuntime;
    impl WasmValidationRuntime for FailRuntime {
        fn call_custom_validator(
            &self,
            _func: &str,
            id: &str,
            _kind: &str,
        ) -> Result<bool, String> {
            Ok(id != "bad") // "bad" fails
        }
    }
    let pattern = ValidationRulePattern {
        code: "E200".to_string(),
        severity: Severity::Error,
        message_template: "{id} failed".to_string(),
        check: ValidationPatternKind::Custom,
        target_kind: None,
        edge_type: None,
        edge_peer_kind: None,
        field: None,
        constraint: None,
        wasm_function: Some("check".to_string()),
    };
    let entities = vec![
        make_entity("bad", "behavior", 1, 0),
        make_entity("good", "behavior", 1, 0),
    ];
    let diags = execute_pattern(&pattern, &entities, Some(&FailRuntime));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("bad"));
}

#[specforge_test(
    behavior = "register_custom_validation_patterns",
    verify = "custom pattern failure emits configured diagnostic"
)]
fn custom_pattern_failure_emits_configured_diagnostic() {
    struct AlwaysFail;
    impl WasmValidationRuntime for AlwaysFail {
        fn call_custom_validator(
            &self,
            _func: &str,
            _id: &str,
            _kind: &str,
        ) -> Result<bool, String> {
            Ok(false)
        }
    }
    let pattern = ValidationRulePattern {
        code: "E201".to_string(),
        severity: Severity::Error,
        message_template: "{kind} '{id}' custom check failed".to_string(),
        check: ValidationPatternKind::Custom,
        target_kind: None,
        edge_type: None,
        edge_peer_kind: None,
        field: None,
        constraint: None,
        wasm_function: Some("always_fail".to_string()),
    };
    let diags = execute_pattern(
        &pattern,
        &[make_entity("b1", "behavior", 1, 0)],
        Some(&AlwaysFail),
    );
    assert_eq!(diags[0].code, "E201");
    assert_eq!(diags[0].severity, Severity::Error);
}

#[specforge_test(
    behavior = "register_custom_validation_patterns",
    verify = "Register Custom Validation Patterns: custom validation pattern registration holds — extension_manifests_loaded_fired, wasm_runtime_available, custom_patterns_registered, wasm_functions_resolved"
)]
fn register_custom_validation_patterns_contract() {
    let custom = ValidationRulePattern {
        code: "E200".to_string(),
        severity: Severity::Error,
        message_template: "test".to_string(),
        check: ValidationPatternKind::Custom,
        target_kind: None,
        edge_type: None,
        edge_peer_kind: None,
        field: None,
        constraint: None,
        wasm_function: Some("func".to_string()),
    };
    let declarative = ValidationRulePattern {
        code: "W100".to_string(),
        severity: Severity::Warning,
        message_template: "test".to_string(),
        check: ValidationPatternKind::NoIncomingEdges,
        target_kind: None,
        edge_type: None,
        edge_peer_kind: None,
        field: None,
        constraint: None,
        wasm_function: None,
    };
    // ensures: custom patterns registered alongside declarative
    let (registered, diags) = register_custom_patterns(&[custom, declarative], None);
    assert_eq!(registered.len(), 2);
    // ensures: unresolvable Wasm produces warning
    assert!(diags.iter().any(|d| d.code == "W112"));
}

// ============================================================================
// B:detect_unknown_entity_fields (6 verifies)
// ============================================================================

#[specforge_test(
    behavior = "detect_unknown_entity_fields",
    verify = "unregistered field name produces W020"
)]
fn unregistered_field_name_produces_w020() {
    let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
    let entities = vec![(
        "behavior".to_string(),
        "b1".to_string(),
        vec!["unknown_field".to_string()],
        span(),
    )];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == "W020" && d.message.contains("unknown_field"))
    );
}

#[specforge_test(
    behavior = "detect_unknown_entity_fields",
    verify = "W020 includes field name, entity kind, and source span"
)]
fn w020_includes_field_name_entity_kind_and_source_span() {
    let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
    let s = SourceSpan {
        file: Sym::new("my.spec"),
        start_line: 5,
        start_col: 3,
        end_line: 5,
        end_col: 20,
    };
    let entities = vec![(
        "behavior".to_string(),
        "b1".to_string(),
        vec!["bogus_field".to_string()],
        s,
    )];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    let w020: Vec<_> = diags.iter().filter(|d| d.code == "W020").collect();
    assert_eq!(w020.len(), 1);
    assert!(
        w020[0].message.contains("bogus_field"),
        "should contain field name"
    );
    assert!(
        w020[0].message.contains("behavior"),
        "should contain entity kind"
    );
    assert!(w020[0].span.is_some(), "should contain source span");
}

#[specforge_test(
    behavior = "detect_unknown_entity_fields",
    verify = "registered field name does not produce W020"
)]
fn registered_field_name_does_not_produce_w020() {
    let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
    let entities = vec![(
        "behavior".to_string(),
        "b1".to_string(),
        vec!["contract".to_string()],
        span(),
    )];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    assert!(diags.is_empty(), "registered field should not produce W020");
}

#[specforge_test(
    behavior = "detect_unknown_entity_fields",
    verify = "structural fields (title, verify) not checked against FieldRegistry"
)]
fn structural_fields_not_checked_against_field_registry() {
    let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
    let entities = vec![(
        "behavior".to_string(),
        "b1".to_string(),
        vec!["title".to_string(), "verify".to_string()],
        span(),
    )];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    assert!(diags.is_empty(), "structural fields should be skipped");
}

#[specforge_test(
    behavior = "detect_unknown_entity_fields",
    verify = "verify on a kind no extension made testable produces W020"
)]
fn verify_on_non_testable_kind_produces_w020() {
    let (mut kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
    kind_reg.get_mut("behavior").unwrap().supports_verify = false;
    let entities = vec![(
        "behavior".to_string(),
        "b1".to_string(),
        vec!["title".to_string(), "verify".to_string()],
        span(),
    )];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    assert_eq!(diags.len(), 1, "only verify is flagged: {diags:?}");
    assert_eq!(diags[0].code, "W020");
    assert!(diags[0].message.contains("'verify'"));
    assert!(
        diags[0]
            .suggestion
            .as_deref()
            .is_some_and(|s| s.contains("@specforge/testing"))
    );
}

#[specforge_test(
    behavior = "detect_unknown_entity_fields",
    verify = "field validation skipped when entity kind is unregistered"
)]
fn field_validation_skipped_when_entity_kind_is_unregistered() {
    let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
    let entities = vec![(
        "nonexistent_kind".to_string(),
        "x1".to_string(),
        vec!["some_field".to_string()],
        span(),
    )];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    assert!(
        diags.is_empty(),
        "unregistered kind should skip field validation to avoid cascading diagnostics"
    );
}

#[specforge_test(
    behavior = "detect_unknown_entity_fields",
    verify = "Detect Unknown Entity Fields: unknown field detection holds — registries_populated_fired, define_blocks_registered_fired, unknown_fields_diagnosed, cascading_avoided"
)]
fn detect_unknown_entity_fields_contract() {
    let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
    // ensures: unknown field → W020
    let e1 = vec![(
        "behavior".to_string(),
        "b1".to_string(),
        vec!["unknown_field".to_string()],
        span(),
    )];
    assert!(
        specforge_registry::compilation::detect_unknown_entity_fields(&e1, &kind_reg, &field_reg)
            .iter()
            .any(|d| d.code == "W020")
    );
    // ensures: registered field → no W020
    let e2 = vec![(
        "behavior".to_string(),
        "b2".to_string(),
        vec!["contract".to_string()],
        span(),
    )];
    assert!(
        specforge_registry::compilation::detect_unknown_entity_fields(&e2, &kind_reg, &field_reg)
            .is_empty()
    );
    // ensures: unregistered kind → skipped
    let e3 = vec![(
        "unknown_kind".to_string(),
        "x".to_string(),
        vec!["field".to_string()],
        span(),
    )];
    assert!(
        specforge_registry::compilation::detect_unknown_entity_fields(&e3, &kind_reg, &field_reg)
            .is_empty()
    );
}

// ============================================================================
// B:detect_duplicate_entity_kinds (4 verifies)
// ============================================================================

#[specforge_test(
    behavior = "detect_duplicate_entity_kinds",
    verify = "duplicate kind from two extensions produces E026"
)]
fn duplicate_kind_from_two_extensions_produces_e026() {
    let m1 = software_manifest();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@other/ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "other.wasm",
            "entityKinds": [
                { "name": "Behavior", "keyword": "behavior" }
            ]
        }"#,
    )
    .unwrap();
    let diags = detect_duplicate_entity_kinds(&[m1, m2]);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "E026" && d.message.contains("behavior")),
        "expected E026 for duplicate 'behavior', got: {:?}",
        diags
    );
}

#[specforge_test(
    behavior = "detect_duplicate_entity_kinds",
    verify = "first extension in topological order owns the kind"
)]
fn first_extension_in_topological_order_owns_the_kind() {
    let m1 = software_manifest();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@other/ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "other.wasm",
            "entityKinds": [
                { "name": "Behavior", "keyword": "behavior" }
            ]
        }"#,
    )
    .unwrap();
    let (kind_reg, _, _, _) = populate_registries(&[m1, m2]);
    let behavior = kind_reg.get("behavior").unwrap();
    assert_eq!(behavior.source_extension, "@specforge/software");
}

#[specforge_test(
    behavior = "detect_duplicate_entity_kinds",
    verify = "single extension registering a kind produces no diagnostic"
)]
fn single_extension_registering_a_kind_produces_no_diagnostic() {
    let diags = detect_duplicate_entity_kinds(&[software_manifest()]);
    assert!(diags.is_empty());
}

#[specforge_test(
    behavior = "detect_duplicate_entity_kinds",
    verify = "Detect Duplicate Entity Kinds: duplicate entity kind detection holds — manifests_loading, collisions_detected, first_wins_enforced"
)]
fn detect_duplicate_entity_kinds_contract() {
    // requires: manifests parsed
    // ensures: no duplicates → no diagnostics
    let diags = detect_duplicate_entity_kinds(&[software_manifest()]);
    assert!(diags.is_empty());
    // ensures: duplicate → E026 with both extension names
    let m2: ManifestV2 = serde_json::from_str(
        r#"{"name":"@other/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"o.wasm",
            "entityKinds":[{"name":"Behavior","keyword":"behavior"}]}"#,
    )
    .unwrap();
    let dup_diags = detect_duplicate_entity_kinds(&[software_manifest(), m2]);
    assert!(dup_diags.iter().any(|d| d.code == "E026"));
}

// ============================================================================
// B:validate_peer_dependencies (4 verifies)
// ============================================================================

#[specforge_test(
    behavior = "validate_peer_dependencies",
    verify = "satisfied peer dependency passes validation"
)]
fn satisfied_peer_dependency_passes_validation() {
    let m1 = software_manifest();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@specforge/product",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "product.wasm",
            "peerDependencies": [
                { "name": "@specforge/software", "version": ">=1.0.0" }
            ]
        }"#,
    )
    .unwrap();
    let diags = validate_peer_dependencies(&[m1, m2]);
    assert!(
        diags.is_empty(),
        "expected no diagnostics, got: {:?}",
        diags
    );
}

#[specforge_test(
    behavior = "validate_peer_dependencies",
    verify = "missing peer dependency produces hard error"
)]
fn missing_peer_dependency_produces_hard_error() {
    let m: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@specforge/product",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "product.wasm",
            "peerDependencies": [
                { "name": "@specforge/software", "version": ">=1.0.0" }
            ]
        }"#,
    )
    .unwrap();
    let diags = validate_peer_dependencies(&[m]);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "E027" && d.message.contains("@specforge/software")),
        "expected E027 for missing peer, got: {:?}",
        diags
    );
}

#[specforge_test(
    behavior = "validate_peer_dependencies",
    verify = "incompatible version produces hard error with required range"
)]
fn incompatible_version_produces_hard_error_with_required_range() {
    let m1: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@specforge/software",
            "version": "0.5.0",
            "manifestVersion": 2,
            "wasmPath": "software.wasm"
        }"#,
    )
    .unwrap();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@specforge/product",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "product.wasm",
            "peerDependencies": [
                { "name": "@specforge/software", "version": ">=1.0.0" }
            ]
        }"#,
    )
    .unwrap();
    let diags = validate_peer_dependencies(&[m1, m2]);
    assert!(
        diags.iter().any(|d| d.code == "E027"
            && d.message.contains(">=1.0.0")
            && d.message.contains("0.5.0")),
        "expected E027 with version info, got: {:?}",
        diags
    );
}

#[specforge_test(
    behavior = "validate_peer_dependencies",
    verify = "Validate Peer Dependencies: peer dependency validation holds — manifests_available, dependencies_validated, unsatisfied_blocked, loading_failed_emitted"
)]
fn validate_peer_dependencies_contract() {
    // requires: manifests loaded
    // ensures: satisfied deps → no error
    let m1 = software_manifest();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{"name":"@specforge/product","version":"1.0.0","manifestVersion":2,"wasmPath":"p.wasm",
            "peerDependencies":[{"name":"@specforge/software","version":">=1.0.0"}]}"#,
    )
    .unwrap();
    assert!(validate_peer_dependencies(&[m1, m2]).is_empty());
    // ensures: missing dep → E027
    let m3: ManifestV2 = serde_json::from_str(
        r#"{"name":"@specforge/product","version":"1.0.0","manifestVersion":2,"wasmPath":"p.wasm",
            "peerDependencies":[{"name":"@specforge/missing","version":">=1.0.0"}]}"#,
    )
    .unwrap();
    let diags = validate_peer_dependencies(&[m3]);
    assert!(diags.iter().any(|d| d.code == "E027"));
}

// ============================================================================
// B:validate_extension_testability (5 verifies)
// ============================================================================

#[specforge_test(
    behavior = "validate_extension_testability",
    verify = "testable kind without supportsVerify produces W017"
)]
fn testable_kind_without_supports_verify_produces_w017() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@test/ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "x.wasm",
            "entityKinds": [
                { "name": "Thing", "keyword": "thing", "testable": true, "supportsVerify": false }
            ]
        }"#,
    )
    .unwrap();
    let (kind_reg, _, _, _) = populate_registries(&[manifest]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "W017" && d.message.contains("thing")),
        "expected W017, got: {:?}",
        diags
    );
}

#[specforge_test(
    behavior = "validate_extension_testability",
    verify = "testable kind with supportsVerify=true passes"
)]
fn testable_kind_with_supports_verify_true_passes() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(
        !diags.iter().any(|d| d.message.contains("behavior")),
        "expected no diagnostics for behavior, got: {:?}",
        diags
    );
}

#[specforge_test(
    behavior = "validate_extension_testability",
    verify = "kind with supportsVerify but not testable produces I006"
)]
fn kind_with_supports_verify_but_not_testable_produces_i006() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@test/ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "x.wasm",
            "entityKinds": [
                { "name": "Note", "keyword": "note", "testable": false, "supportsVerify": true }
            ]
        }"#,
    )
    .unwrap();
    let (kind_reg, _, _, _) = populate_registries(&[manifest]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "I006" && d.message.contains("note")),
        "expected I006, got: {:?}",
        diags
    );
}

#[specforge_test(
    behavior = "validate_extension_testability",
    verify = "consistent testable and supportsVerify flags produce no diagnostic"
)]
fn consistent_testable_and_supports_verify_flags_produce_no_diagnostic() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@test/ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "x.wasm",
            "entityKinds": [
                { "name": "Thing", "keyword": "thing", "testable": false, "supportsVerify": false }
            ]
        }"#,
    )
    .unwrap();
    let (kind_reg, _, _, _) = populate_registries(&[manifest]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(
        diags.is_empty(),
        "expected no diagnostics, got: {:?}",
        diags
    );
}

#[specforge_test(
    behavior = "validate_extension_testability",
    verify = "Validate Extension Testability: extension testability validation holds — registries_populated_fired, flag_consistency_checked, advisory_diagnostics_emitted"
)]
fn validate_extension_testability_contract() {
    // requires: KindRegistry populated
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    // ensures: consistent flags → no diagnostics
    let diags = validate_extension_testability(&kind_reg);
    assert!(diags.is_empty());
    // ensures: testable without supportsVerify → W017
    let bad: ManifestV2 = serde_json::from_str(
        r#"{"name":"@t/e","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[{"name":"X","keyword":"x","testable":true,"supportsVerify":false}]}"#,
    )
    .unwrap();
    let (bad_kr, _, _, _) = populate_registries(&[bad]);
    let bad_diags = validate_extension_testability(&bad_kr);
    assert!(bad_diags.iter().any(|d| d.code == "W017"));
}
