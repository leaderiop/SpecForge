//! Integration tests for spec/behaviors/zero-entity-validation.spec
//!
//! Covers these behaviors:
//! - execute_validation_pattern (9)
//! - detect_unknown_entity_fields (6)
//! - parse_validation_rule_pattern (5)
//! - emit_diagnostic_from_pattern (5)
//! - register_custom_validation_patterns (2; the rest in specforge-project)
//! - registry_build_rules (1; the rest through the build, tests/build/rules.rs)

use specforge_common::{Severity, SourceSpan, Sym};
use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::{FieldConstraintDescriptor, ValidationRuleDescriptor};
use specforge_registry::RegistryBuild;
use specforge_registry::compilation::tests::support::{registries, software};
use specforge_registry::entity::{Direction, EntityRecord, RuleInput};
use specforge_registry::validation_engine::{
    CustomVerdict, ValidationPatternKind, ValidationRulePattern, WasmValidationRuntime,
    execute_pattern, interpolate_template, parse_all_rule_patterns, parse_rule_pattern,
    resolve_edge_rules,
};
use specforge_registry::{
    EdgeRegistry, EdgeRegistryEntry, FieldRegistryEntry, KindRegistry, KindRegistryEntry,
    ManifestFieldType,
};
use specforge_test_macros::test as specforge_test;
use std::path::Path;

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

/// A span that outlives the test's entity views.
fn pinned(span: SourceSpan) -> &'static SourceSpan {
    Box::leak(Box::new(span))
}

fn make_rule(code: &str, check: &str) -> ValidationRuleDescriptor {
    ValidationRuleDescriptor {
        code: code.to_string(),
        severity: ValidationSeverity::Warning,
        message_template: "orphan {kind} '{id}'".to_string(),
        check: check.to_string(),
        target_kind: Some("behavior".to_string()),
        edge_type: None,
        field: None,
        constraint: None,
        wasm_function: None,
    }
}

/// The rules' input over `entities`, with no edges and no spec root.
fn rules_over(entities: &[EntityRecord]) -> RuleInput<'_> {
    RuleInput {
        entities,
        edges: &[],
        spec_root: Path::new(""),
    }
}

fn make_entity(id: &str, kind: &str, incoming: usize, outgoing: usize) -> EntityRecord {
    let mut entity = EntityRecord::new(kind, id, &span());
    entity.incoming.total = incoming;
    entity.outgoing.total = outgoing;
    entity
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
    rule.constraint = Some(FieldConstraintDescriptor {
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
    rule.constraint = Some(FieldConstraintDescriptor {
        kind: "one_of".to_string(),
        pattern: None,
        values: vec!["draft".to_string(), "active".to_string()],
    });
    let pattern = parse_rule_pattern(&rule, "@test/ext").unwrap();
    assert_eq!(pattern.check, ValidationPatternKind::FieldValueConstraint);
}

#[test]
fn all_required_fields_validated_on_each_rule() {
    let rule = ValidationRuleDescriptor {
        code: "W100".to_string(),
        severity: ValidationSeverity::Error,
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
    let diags = execute_pattern(&pattern, &rules_over(&entities), None);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("b1"));
}

fn kind(name: &str) -> KindRegistryEntry {
    KindRegistryEntry {
        kind_name: name.to_string(),
        source_extension: "@test".to_string(),
        testable: false,
        supports_verify: false,
        allowed_verify_kinds: Vec::new(),
        lifecycle_field: None,
        ..Default::default()
    }
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "an edge rule counts only edges of its edge type and is dropped when no extension declares the kind at its far end"
)]
fn an_edge_rule_counts_only_its_edge_type() {
    let mut edges = EdgeRegistry::new();
    edges.register(EdgeRegistryEntry {
        source_extension: "@test".to_string(),
        declared: specforge_protocol_types::EdgeTypeDescriptor {
            label: "BehaviorImplementsFeature".to_string(),
            source_kind: Some("behavior".to_string()),
            target_kind: Some("feature".to_string()),
            ..Default::default()
        },
    });
    let mut rule = make_rule("W001", "no_outgoing_edges");
    rule.edge_type = Some("BehaviorImplementsFeature".to_string());
    rule.message_template = "behavior '{id}' does not implement any feature".to_string();

    // b1 references an event but no feature; b2 implements a feature.
    let b1 = make_entity("b1", "behavior", 0, 0).with_edges(Direction::Outgoing, "event", 1);
    let b2 = make_entity("b2", "behavior", 0, 0).with_edges(Direction::Outgoing, "feature", 1);
    let entities = vec![b1, b2];

    let mut kinds = KindRegistry::new();
    kinds.register(kind("behavior"));
    kinds.register(kind("feature"));
    let (mut patterns, _) = parse_all_rule_patterns(&[("@test".to_string(), vec![rule.clone()])]);
    resolve_edge_rules(&mut patterns, &edges, &kinds);
    let diags = execute_pattern(&patterns[0].0, &rules_over(&entities), None);
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
    let diags = execute_pattern(&pattern, &rules_over(&entities), None);
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
    e2 = e2.with_field("contract", "some text");

    let diags = execute_pattern(&pattern, &rules_over(&[e1, e2]), None);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("b1"));
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "field_value_constraint rejects invalid field value"
)]
fn field_value_constraint_rejects_invalid_field_value() {
    let rule = ValidationRuleDescriptor {
        code: "W103".to_string(),
        severity: ValidationSeverity::Warning,
        message_template: "{kind} '{id}' has invalid {field}='{value}'".to_string(),
        check: "field_value_constraint".to_string(),
        target_kind: Some("behavior".to_string()),
        edge_type: None,
        field: Some("status".to_string()),
        constraint: Some(FieldConstraintDescriptor {
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
    e1 = e1.with_field("status", "invalid_status");
    let mut e2 = make_entity("b2", "behavior", 1, 0);
    e2 = e2.with_field("status", "active");

    let diags = execute_pattern(&pattern, &rules_over(&[e1, e2]), None);
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
    let diags = execute_pattern(
        &pattern,
        &rules_over(&[make_entity("b1", "behavior", 1, 1)]),
        None,
    );
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
    let rule = ValidationRuleDescriptor {
        code: "E101".to_string(),
        severity: ValidationSeverity::Error,
        message_template: "{kind} '{id}' references missing file".to_string(),
        check: "file_exists".to_string(),
        target_kind: Some("behavior".to_string()),
        edge_type: None,
        field: Some("gherkin".to_string()),
        constraint: None,
        wasm_function: None,
    };
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("features")).unwrap();
    std::fs::write(root.path().join("features/login.feature"), "").unwrap();
    let naming = |id: &str, path: &str| {
        let mut entity = make_entity(id, "behavior", 1, 0);
        entity = entity.with_field("gherkin", path);
        entity
    };
    let present = root.path().join("features/login.feature");
    let entities = [
        naming("b1", "features/login.feature"),
        naming("b2", "features/logout.feature"),
        naming("b3", "/nonexistent/file.feature"),
        naming("b4", present.to_str().unwrap()),
    ];

    // Relative paths are the spec root's; absolute ones are checked as
    // written.
    let diags = execute_pattern(
        &pattern,
        &RuleInput {
            entities: &entities,
            edges: &[],
            spec_root: root.path(),
        },
        None,
    );
    let reported: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(
        reported,
        [
            "behavior 'b2' references missing file",
            "behavior 'b3' references missing file",
        ]
    );
    assert!(diags.iter().all(|d| d.code == "E101"));
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "custom pattern dispatches to registered Wasm function"
)]
fn custom_pattern_dispatches_to_registered_wasm_function() {
    struct NamingValidator;
    impl WasmValidationRuntime for NamingValidator {
        fn custom_verdict(
            &self,
            func: &str,
            id: &str,
            _kind: &str,
        ) -> Result<CustomVerdict, String> {
            if func == "validate_naming" && id == "bad_name" {
                Ok(failed()) // fails
            } else {
                Ok(CustomVerdict::Pass) // passes
            }
        }
    }

    let rule = ValidationRuleDescriptor {
        code: "E200".to_string(),
        severity: ValidationSeverity::Error,
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
    let diags = execute_pattern(&pattern, &rules_over(&entities), Some(&NamingValidator));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("bad_name"));
}

#[specforge_test(
    behavior = "execute_validation_pattern",
    verify = "pattern violation produces diagnostic with configured code and severity"
)]
fn pattern_violation_produces_diagnostic_with_configured_code_and_severity() {
    let rule = ValidationRuleDescriptor {
        code: "E999".to_string(),
        severity: ValidationSeverity::Error,
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
    let diags = execute_pattern(&pattern, &rules_over(&entities), None);
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
    let diags = execute_pattern(&pattern, &rules_over(&entities), None);
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
    let rule = ValidationRuleDescriptor {
        code: "E999".to_string(),
        severity: ValidationSeverity::Error,
        message_template: "test".to_string(),
        check: "no_incoming_edges".to_string(),
        target_kind: Some("behavior".to_string()),
        edge_type: None,
        field: None,
        constraint: None,
        wasm_function: None,
    };
    let pattern = parse_rule_pattern(&rule, "@test").unwrap();
    let diags = execute_pattern(
        &pattern,
        &rules_over(&[make_entity("b1", "behavior", 0, 0)]),
        None,
    );
    assert_eq!(diags[0].code, "E999");
}

#[specforge_test(
    behavior = "emit_diagnostic_from_pattern",
    verify = "diagnostic severity matches pattern severity"
)]
fn diagnostic_severity_matches_pattern_severity() {
    for (sev, expected) in &[
        (ValidationSeverity::Error, Severity::Error),
        (ValidationSeverity::Warning, Severity::Warning),
        (ValidationSeverity::Info, Severity::Info),
    ] {
        let rule = ValidationRuleDescriptor {
            code: "X001".to_string(),
            severity: sev.clone(),
            message_template: "test".to_string(),
            check: "no_incoming_edges".to_string(),
            target_kind: Some("behavior".to_string()),
            edge_type: None,
            field: None,
            constraint: None,
            wasm_function: None,
        };
        let pattern = parse_rule_pattern(&rule, "@test").unwrap();
        let diags = execute_pattern(
            &pattern,
            &rules_over(&[make_entity("b1", "behavior", 0, 0)]),
            None,
        );
        assert_eq!(
            diags[0].severity, *expected,
            "severity mismatch for {:?}",
            sev
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
    let diags = execute_pattern(
        &pattern,
        &rules_over(&[make_entity("b1", "behavior", 0, 0)]),
        None,
    );
    assert_eq!(diags[0].code, "W100");
    assert_eq!(diags[0].severity, Severity::Warning);
}

// ============================================================================
// B:register_custom_validation_patterns (2 of 5 verifies; the load-time
// registration and wasm_function resolution go through Environment::load in
// crates/specforge-project/tests/custom_rules.rs)
// ============================================================================

#[specforge_test(
    behavior = "register_custom_validation_patterns",
    verify = "custom pattern dispatched to Wasm runtime during validation"
)]
fn custom_pattern_dispatched_to_wasm_runtime_during_validation() {
    struct FailRuntime;
    impl WasmValidationRuntime for FailRuntime {
        fn custom_verdict(
            &self,
            _func: &str,
            id: &str,
            _kind: &str,
        ) -> Result<CustomVerdict, String> {
            Ok(if id == "bad" {
                failed()
            } else {
                CustomVerdict::Pass
            }) // "bad" fails
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
    let diags = execute_pattern(&pattern, &rules_over(&entities), Some(&FailRuntime));
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
        fn custom_verdict(
            &self,
            _func: &str,
            _id: &str,
            _kind: &str,
        ) -> Result<CustomVerdict, String> {
            Ok(failed())
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
        &rules_over(&[make_entity("b1", "behavior", 1, 0)]),
        Some(&AlwaysFail),
    );
    assert_eq!(diags[0].code, "E201");
    assert_eq!(diags[0].severity, Severity::Error);
}

// ============================================================================
// B:detect_unknown_entity_fields (6 verifies)
// ============================================================================

#[specforge_test(
    behavior = "detect_unknown_entity_fields",
    verify = "unregistered field name produces W020"
)]
fn unregistered_field_name_produces_w020() {
    let RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = registries(&[software()]);
    let entities =
        vec![EntityRecord::new("behavior", "b1", pinned(span())).with_fields(&["unknown_field"])];
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
    let RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = registries(&[software()]);
    let s = SourceSpan {
        file: Sym::new("my.spec"),
        start_line: 5,
        start_col: 3,
        end_line: 5,
        end_col: 20,
    };
    let entities = vec![EntityRecord::new("behavior", "b1", &s).with_fields(&["bogus_field"])];
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
    verify = "expression is checked like any other field (W020 where undeclared)"
)]
fn expression_is_checked_like_any_other_field() {
    let RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = registries(&[software()]);
    // software's invariant and behavior declare no `expression`; without an
    // extension that declares it (formal enhances invariant), it is W020.
    let entities = vec![
        EntityRecord::new("invariant", "i1", pinned(span())).with_fields(&["expression"]),
        EntityRecord::new("behavior", "b1", pinned(span())).with_fields(&["expression"]),
    ];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    let flagged: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(diags.len(), 2, "{flagged:?}");
    assert!(
        diags
            .iter()
            .all(|d| d.code == "W020" && d.message.contains("'expression'")),
        "{flagged:?}"
    );

    // Declared on a kind, it is a field of that kind like any other.
    let mut field_reg = field_reg;
    field_reg.register(FieldRegistryEntry {
        kind_name: "invariant".to_string(),
        field_type: ManifestFieldType::String,
        source_extension: "@specforge/formal".to_string(),
        proof_role: Some(specforge_registry::ProofRole::Claim),
        declared: specforge_protocol_types::FieldDescriptor {
            name: "expression".to_string(),
            normative: true,
            ..Default::default()
        },
    });
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert!(diags[0].message.contains("behavior"), "{diags:?}");
}

#[specforge_test(
    behavior = "detect_unknown_entity_fields",
    verify = "registered field name does not produce W020"
)]
fn registered_field_name_does_not_produce_w020() {
    let RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = registries(&[software()]);
    let entities =
        vec![EntityRecord::new("behavior", "b1", pinned(span())).with_fields(&["contract"])];
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
    let RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = registries(&[software()]);
    let entities =
        vec![EntityRecord::new("behavior", "b1", pinned(span())).with_fields(&["title", "verify"])];
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
    let RegistryBuild {
        kinds: mut kind_reg,
        fields: field_reg,
        ..
    } = registries(&[software()]);
    kind_reg.get_mut("behavior").unwrap().supports_verify = false;
    let entities =
        vec![EntityRecord::new("behavior", "b1", pinned(span())).with_fields(&["title", "verify"])];
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
    let RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = registries(&[software()]);
    let entities = vec![
        EntityRecord::new("nonexistent_kind", "x1", pinned(span())).with_fields(&["some_field"]),
    ];
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
    verify = "Detect Unknown Entity Fields: unknown field detection holds — registries_populated_fired, unknown_fields_diagnosed, cascading_avoided"
)]
fn detect_unknown_entity_fields_contract() {
    let RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = registries(&[software()]);
    // ensures: unknown field → W020
    let e1 =
        vec![EntityRecord::new("behavior", "b1", pinned(span())).with_fields(&["unknown_field"])];
    assert!(
        specforge_registry::compilation::detect_unknown_entity_fields(&e1, &kind_reg, &field_reg)
            .iter()
            .any(|d| d.code == "W020")
    );
    // ensures: registered field → no W020
    let e2 = vec![EntityRecord::new("behavior", "b2", pinned(span())).with_fields(&["contract"])];
    assert!(
        specforge_registry::compilation::detect_unknown_entity_fields(&e2, &kind_reg, &field_reg)
            .is_empty()
    );
    // ensures: unregistered kind → skipped
    let e3 = vec![EntityRecord::new("unknown_kind", "x", pinned(span())).with_fields(&["field"])];
    assert!(
        specforge_registry::compilation::detect_unknown_entity_fields(&e3, &kind_reg, &field_reg)
            .is_empty()
    );
}

// ============================================================================
// B:registry_build_rules (the rule the build keeps, run)
// ============================================================================

// A rule for a `ghost` kind no loaded extension declares: it runs over the
// project's behaviors and reports nothing.
#[specforge_test(
    behavior = "registry_build_rules",
    verify = "a rule targeting a kind no loaded extension declares reports nothing"
)]
fn a_rule_for_an_unloaded_kind_reports_nothing() {
    let mut rule = make_rule("W100", "no_incoming_edges");
    rule.target_kind = Some("ghost".to_string());
    let (patterns, diags) = parse_all_rule_patterns(&[("@test".to_string(), vec![rule])]);
    assert!(diags.is_empty(), "{diags:?}");
    let orphans = vec![
        make_entity("b1", "behavior", 0, 0),
        make_entity("b2", "behavior", 0, 0),
    ];
    assert!(execute_pattern(&patterns[0].0, &rules_over(&orphans), None).is_empty());

    // The same rule on a loaded kind does fire: it is inert, not broken.
    let mut rule = make_rule("W100", "no_incoming_edges");
    rule.target_kind = Some("behavior".to_string());
    let (patterns, _) = parse_all_rule_patterns(&[("@test".to_string(), vec![rule])]);
    assert_eq!(
        execute_pattern(&patterns[0].0, &rules_over(&orphans), None).len(),
        2
    );
}

/// A custom rule's failing verdict, with no field or value to name.
fn failed() -> CustomVerdict {
    CustomVerdict::Fail {
        field: None,
        value: None,
    }
}
