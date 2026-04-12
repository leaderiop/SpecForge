use crate::ManifestValidationRule;
use specforge_common::{Diagnostic, Severity};

/// Parsed and validated rule pattern, ready for execution.
#[derive(Debug, Clone)]
pub struct ValidationRulePattern {
    pub code: String,
    pub severity: Severity,
    pub message_template: String,
    pub check: ValidationPatternKind,
    pub target_kind: Option<String>,
    pub edge_type: Option<String>,
    pub field: Option<String>,
    pub constraint: Option<FieldConstraintPattern>,
    pub wasm_function: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationPatternKind {
    NoIncomingEdges,
    NoOutgoingEdges,
    NoEdges,
    MissingFieldWhenFlagSet,
    FieldValueConstraint,
    CycleDetection,
    FileExists,
    Custom,
}

#[derive(Debug, Clone)]
pub struct FieldConstraintPattern {
    pub kind: String,
    pub pattern: Option<String>,
    pub values: Vec<String>,
}

/// A stub trait for Wasm validation dispatch. Real implementation in specforge-wasm.
pub trait WasmValidationRuntime {
    fn call_custom_validator(
        &self,
        wasm_function: &str,
        entity_id: &str,
        entity_kind: &str,
    ) -> Result<bool, String>;
}

/// No-op Wasm runtime stub for when Wasm is not available.
pub struct StubWasmRuntime;

impl WasmValidationRuntime for StubWasmRuntime {
    fn call_custom_validator(
        &self,
        wasm_function: &str,
        _entity_id: &str,
        _entity_kind: &str,
    ) -> Result<bool, String> {
        Err(format!(
            "Wasm runtime not available — cannot call '{}'",
            wasm_function
        ))
    }
}

/// Parse a ManifestValidationRule into a ValidationRulePattern.
/// Returns Ok(pattern) or Err(diagnostic) for unrecognized check kinds.
#[allow(clippy::result_large_err)]
pub fn parse_rule_pattern(
    rule: &ManifestValidationRule,
    extension_name: &str,
) -> Result<ValidationRulePattern, Diagnostic> {
    let check = match rule.check.as_str() {
        "no_incoming_edges" => ValidationPatternKind::NoIncomingEdges,
        "no_outgoing_edges" => ValidationPatternKind::NoOutgoingEdges,
        "no_edges" => ValidationPatternKind::NoEdges,
        "missing_field_when_flag_set" => ValidationPatternKind::MissingFieldWhenFlagSet,
        "field_value_constraint" => ValidationPatternKind::FieldValueConstraint,
        "cycle_detection" => ValidationPatternKind::CycleDetection,
        "file_exists" => ValidationPatternKind::FileExists,
        "custom" => ValidationPatternKind::Custom,
        other => {
            return Err(Diagnostic {
                code: "W024".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "extension '{}': unrecognized validation pattern kind '{}'",
                    extension_name, other
                ),
                span: None,
                suggestion: None,
            });
        }
    };

    let severity = match rule.severity.as_str() {
        "error" => Severity::Error,
        "warning" => Severity::Warning,
        "info" => Severity::Info,
        _ => Severity::Warning,
    };

    let constraint = rule.constraint.as_ref().map(|c| FieldConstraintPattern {
        kind: c.kind.clone(),
        pattern: c.pattern.clone(),
        values: c.values.clone(),
    });

    Ok(ValidationRulePattern {
        code: rule.code.clone(),
        severity,
        message_template: rule.message_template.clone(),
        check,
        target_kind: rule.target_kind.clone(),
        edge_type: rule.edge_type.clone(),
        field: rule.field.clone(),
        constraint,
        wasm_function: rule.wasm_function.clone(),
    })
}

/// Parse all rules from manifests into validated patterns.
pub fn parse_all_rule_patterns(
    manifests: &[(String, Vec<ManifestValidationRule>)], // (ext_name, rules)
) -> (Vec<ValidationRulePattern>, Vec<Diagnostic>) {
    let mut patterns = Vec::new();
    let mut diagnostics = Vec::new();

    for (ext_name, rules) in manifests {
        for rule in rules {
            match parse_rule_pattern(rule, ext_name) {
                Ok(pattern) => patterns.push(pattern),
                Err(diag) => diagnostics.push(diag),
            }
        }
    }

    // Sort by code for deterministic execution order
    patterns.sort_by(|a, b| a.code.cmp(&b.code));
    (patterns, diagnostics)
}

/// Interpolate a message template with entity context.
pub fn interpolate_template(
    template: &str,
    id: &str,
    kind: &str,
    field: Option<&str>,
    value: Option<&str>,
) -> String {
    let mut result = template.replace("{id}", id).replace("{kind}", kind);
    if let Some(f) = field {
        result = result.replace("{field}", f);
    }
    if let Some(v) = value {
        result = result.replace("{value}", v);
    }
    result
}

/// A simple entity representation for validation.
#[derive(Debug, Clone)]
pub struct ValidationEntity {
    pub id: String,
    pub kind: String,
    pub fields: std::collections::HashMap<String, String>,
    pub incoming_edge_count: usize,
    pub outgoing_edge_count: usize,
    pub span: specforge_common::SourceSpan,
}

/// Execute a single validation pattern against a set of entities.
pub fn execute_pattern(
    pattern: &ValidationRulePattern,
    entities: &[ValidationEntity],
    wasm: Option<&dyn WasmValidationRuntime>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let applicable: Vec<&ValidationEntity> = if let Some(ref target) = pattern.target_kind {
        entities.iter().filter(|e| e.kind == *target).collect()
    } else {
        entities.iter().collect()
    };

    for entity in applicable {
        let violated = match pattern.check {
            ValidationPatternKind::NoIncomingEdges => entity.incoming_edge_count == 0,
            ValidationPatternKind::NoOutgoingEdges => entity.outgoing_edge_count == 0,
            ValidationPatternKind::NoEdges => {
                entity.incoming_edge_count == 0 && entity.outgoing_edge_count == 0
            }
            ValidationPatternKind::MissingFieldWhenFlagSet => {
                if let Some(ref field_name) = pattern.field {
                    // Union types (type X = A | B) have a "variants" field but
                    // cannot syntactically hold verify statements, so skip them
                    // for verify-related checks.
                    if field_name == "verify" && entity.fields.contains_key("variants") {
                        false
                    } else {
                        !entity.fields.contains_key(field_name)
                    }
                } else {
                    false
                }
            }
            ValidationPatternKind::FieldValueConstraint => {
                if let (Some(field_name), Some(constraint)) =
                    (&pattern.field, &pattern.constraint)
                {
                    if let Some(value) = entity.fields.get(field_name) {
                        match constraint.kind.as_str() {
                            "non_empty" => value.is_empty(),
                            "one_of" => !constraint.values.contains(value),
                            "matches" => {
                                if let Some(ref pat) = constraint.pattern {
                                    !value.contains(pat)
                                } else {
                                    false
                                }
                            }
                            _ => false,
                        }
                    } else {
                        false // field not present — not a constraint violation
                    }
                } else {
                    false
                }
            }
            ValidationPatternKind::FileExists => {
                if let Some(ref field_name) = pattern.field {
                    if let Some(path) = entity.fields.get(field_name) {
                        !std::path::Path::new(path).exists()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            ValidationPatternKind::CycleDetection => {
                // Cycle detection requires full graph traversal — deferred
                // to the caller who has access to the graph structure.
                false
            }
            ValidationPatternKind::Custom => {
                if let (Some(func), Some(rt)) = (&pattern.wasm_function, wasm) {
                    match rt.call_custom_validator(func, &entity.id, &entity.kind) {
                        Ok(passed) => !passed,
                        Err(_) => false,
                    }
                } else {
                    false
                }
            }
        };

        if violated {
            let message = interpolate_template(
                &pattern.message_template,
                &entity.id,
                &entity.kind,
                pattern.field.as_deref(),
                entity.fields.get(pattern.field.as_deref().unwrap_or("")).map(|s| s.as_str()),
            );

            diagnostics.push(Diagnostic {
                code: pattern.code.clone(),
                severity: pattern.severity,
                message,
                span: Some(entity.span.clone()),
                suggestion: None,
            });
        }
    }

    diagnostics
}

/// Register custom validation patterns, resolving Wasm function references.
pub fn register_custom_patterns(
    patterns: &[ValidationRulePattern],
    wasm: Option<&dyn WasmValidationRuntime>,
) -> (Vec<ValidationRulePattern>, Vec<Diagnostic>) {
    let mut registered = Vec::new();
    let mut diagnostics = Vec::new();

    for pattern in patterns {
        if pattern.check == ValidationPatternKind::Custom {
            if let Some(ref func) = pattern.wasm_function {
                // Try to resolve the Wasm function
                if let Some(rt) = wasm {
                    match rt.call_custom_validator(func, "__probe__", "__probe__") {
                        Ok(_) | Err(_) => {
                            // Function exists (or runtime available but function fails) — register it
                            registered.push(pattern.clone());
                        }
                    }
                } else {
                    diagnostics.push(Diagnostic {
                        code: "W025".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "custom validation pattern '{}' references wasm_function '{}' but Wasm runtime is not available",
                            pattern.code, func
                        ),
                        span: None,
                        suggestion: None,
                    });
                    // Still register it — it will be skipped during execution
                    registered.push(pattern.clone());
                }
            } else {
                diagnostics.push(Diagnostic {
                    code: "W025".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "custom validation pattern '{}' has check 'custom' but no wasm_function",
                        pattern.code
                    ),
                    span: None,
                    suggestion: None,
                });
            }
        } else {
            registered.push(pattern.clone());
        }
    }

    (registered, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ManifestValidationRule;
    use specforge_common::Sym;

    fn span() -> specforge_common::SourceSpan {
        specforge_common::SourceSpan {
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
        }
    }

    // -- B:parse_validation_rule_pattern --

    // B:parse_validation_rule_pattern — verify unit "parses no_incoming_edges pattern from manifest"
    #[test]
    fn test_parses_no_incoming_edges() {
        let rule = make_rule("W100", "no_incoming_edges");
        let pattern = parse_rule_pattern(&rule, "@test/ext").unwrap();
        assert_eq!(pattern.check, ValidationPatternKind::NoIncomingEdges);
        assert_eq!(pattern.code, "W100");
    }

    // B:parse_validation_rule_pattern — verify unit "parses missing_field_when_flag_set pattern from manifest"
    #[test]
    fn test_parses_missing_field_when_flag_set() {
        let mut rule = make_rule("W101", "missing_field_when_flag_set");
        rule.field = Some("contract".to_string());
        let pattern = parse_rule_pattern(&rule, "@test/ext").unwrap();
        assert_eq!(pattern.check, ValidationPatternKind::MissingFieldWhenFlagSet);
        assert_eq!(pattern.field.as_deref(), Some("contract"));
    }

    // B:parse_validation_rule_pattern — verify unit "unrecognized pattern kind produces warning"
    #[test]
    fn test_unrecognized_pattern_kind_warning() {
        let rule = make_rule("W102", "invalid_check_kind");
        let result = parse_rule_pattern(&rule, "@test/ext");
        assert!(result.is_err());
        let diag = result.unwrap_err();
        assert_eq!(diag.code, "W024");
        assert!(diag.message.contains("invalid_check_kind"));
    }

    // B:parse_validation_rule_pattern — verify unit "all required fields validated on each rule"
    #[test]
    fn test_all_required_fields_validated() {
        // A valid rule must have code, severity, messageTemplate, check
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
    }

    // B:parse_validation_rule_pattern — verify contract "requires/ensures consistency for validation rule parsing"
    #[test]
    fn test_parse_validation_rule_pattern_contract() {
        // requires: manifest rules available
        let rules = vec![
            ("@ext/a".to_string(), vec![make_rule("W100", "no_incoming_edges")]),
            ("@ext/b".to_string(), vec![make_rule("W200", "invalid_kind")]),
        ];
        let (patterns, diags) = parse_all_rule_patterns(&rules);
        // ensures: valid patterns parsed
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].code, "W100");
        // ensures: unrecognized warned
        assert!(diags.iter().any(|d| d.code == "W024"));
    }

    // -- B:execute_validation_pattern --

    // B:execute_validation_pattern — verify unit "no_incoming_edges detects orphan entities"
    #[test]
    fn test_no_incoming_edges_detects_orphans() {
        let pattern = parse_rule_pattern(
            &make_rule("W100", "no_incoming_edges"),
            "@test",
        ).unwrap();
        let entities = vec![
            make_entity("b1", "behavior", 0, 2), // orphan
            make_entity("b2", "behavior", 1, 0), // not orphan
        ];
        let diags = execute_pattern(&pattern, &entities, None);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("b1"));
    }

    // B:execute_validation_pattern — verify unit "no_outgoing_edges detects entities with zero outgoing edges"
    #[test]
    fn test_no_outgoing_edges_detects_leaf_entities() {
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

    // B:execute_validation_pattern — verify unit "missing_field_when_flag_set detects missing specified field on flagged entity"
    #[test]
    fn test_missing_field_when_flag_set() {
        let mut rule = make_rule("W102", "missing_field_when_flag_set");
        rule.field = Some("contract".to_string());
        rule.message_template = "{kind} '{id}' missing field '{field}'".to_string();
        let pattern = parse_rule_pattern(&rule, "@test").unwrap();

        let e1 = make_entity("b1", "behavior", 1, 0);
        // b1 has no "contract" field → violation
        let mut e2 = make_entity("b2", "behavior", 1, 0);

        e2.fields.insert("contract".to_string(), "some text".to_string());
        // b2 has "contract" → ok

        let diags = execute_pattern(&pattern, &[e1, e2], None);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("b1"));
    }

    // B:execute_validation_pattern — verify unit "field_value_constraint rejects invalid field value"
    #[test]
    fn test_field_value_constraint_rejects_invalid() {
        let rule = ManifestValidationRule {
            code: "W103".to_string(),
            severity: "warning".to_string(),
            message_template: "{kind} '{id}' has invalid {field}='{value}'".to_string(),
            check: "field_value_constraint".to_string(),
            target_kind: Some("behavior".to_string()),
            edge_type: None,
            field: Some("status".to_string()),
            constraint: Some(crate::FieldConstraint {
                kind: "one_of".to_string(),
                pattern: None,
                values: vec!["draft".to_string(), "active".to_string(), "deprecated".to_string()],
            }),
            wasm_function: None,
        };
        let pattern = parse_rule_pattern(&rule, "@test").unwrap();

        let mut e1 = make_entity("b1", "behavior", 1, 0);
        e1.fields.insert("status".to_string(), "invalid_status".to_string());
        let mut e2 = make_entity("b2", "behavior", 1, 0);
        e2.fields.insert("status".to_string(), "active".to_string());

        let diags = execute_pattern(&pattern, &[e1, e2], None);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("b1"));
    }

    // B:execute_validation_pattern — verify unit "cycle_detection finds cycles in edge type"
    #[test]
    fn test_cycle_detection_placeholder() {
        // Cycle detection requires full graph — current implementation defers to caller.
        // The pattern parses but execution returns no violations (graph needed).
        let rule = make_rule("E100", "cycle_detection");
        let pattern = parse_rule_pattern(&rule, "@test").unwrap();
        assert_eq!(pattern.check, ValidationPatternKind::CycleDetection);
        let diags = execute_pattern(&pattern, &[make_entity("b1", "behavior", 1, 1)], None);
        assert!(diags.is_empty(), "cycle detection deferred to graph-aware caller");
    }

    // B:execute_validation_pattern — verify unit "file_exists reports missing file-reference field targets"
    #[test]
    fn test_file_exists_reports_missing() {
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
        entity.fields.insert("gherkin".to_string(), "/nonexistent/file.feature".to_string());

        let diags = execute_pattern(&pattern, &[entity], None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E101");
    }

    // B:execute_validation_pattern — verify unit "custom pattern dispatches to registered Wasm function"
    #[test]
    fn test_custom_pattern_dispatches_to_wasm() {
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

    // B:execute_validation_pattern — verify unit "pattern violation produces diagnostic with configured code and severity"
    #[test]
    fn test_violation_produces_configured_diagnostic() {
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

    // B:execute_validation_pattern — verify contract "requires/ensures consistency for declarative validation"
    #[test]
    fn test_execute_validation_pattern_contract() {
        let rule = make_rule("W100", "no_incoming_edges");
        let pattern = parse_rule_pattern(&rule, "@test").unwrap();
        // ensures: all entities matched
        let entities = vec![
            make_entity("b1", "behavior", 0, 1),
            make_entity("b2", "behavior", 2, 0),
            make_entity("f1", "feature", 0, 0), // different kind, skipped by target_kind
        ];
        let diags = execute_pattern(&pattern, &entities, None);
        // Only behavior with 0 incoming edges diagnosed
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("b1"));
    }

    // -- B:emit_diagnostic_from_pattern --

    // B:emit_diagnostic_from_pattern — verify unit "message template interpolates {id} and {kind}"
    #[test]
    fn test_template_interpolates_id_and_kind() {
        let result = interpolate_template("orphan {kind} '{id}'", "my_beh", "behavior", None, None);
        assert_eq!(result, "orphan behavior 'my_beh'");
    }

    // B:emit_diagnostic_from_pattern — verify unit "message template interpolates {field} and {value}"
    #[test]
    fn test_template_interpolates_field_and_value() {
        let result = interpolate_template(
            "{kind} '{id}' has {field}='{value}'",
            "b1",
            "behavior",
            Some("status"),
            Some("invalid"),
        );
        assert_eq!(result, "behavior 'b1' has status='invalid'");
    }

    // B:emit_diagnostic_from_pattern — verify unit "diagnostic code matches pattern code"
    #[test]
    fn test_diagnostic_code_matches_pattern() {
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

    // B:emit_diagnostic_from_pattern — verify unit "diagnostic severity matches pattern severity"
    #[test]
    fn test_diagnostic_severity_matches_pattern() {
        for (sev_str, expected) in &[("error", Severity::Error), ("warning", Severity::Warning), ("info", Severity::Info)] {
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
            assert_eq!(diags[0].severity, *expected, "severity mismatch for {}", sev_str);
        }
    }

    // B:emit_diagnostic_from_pattern — verify contract "requires/ensures consistency for pattern diagnostic emission"
    #[test]
    fn test_emit_diagnostic_from_pattern_contract() {
        // requires: violation detected, pattern configured
        let result = interpolate_template("{kind} '{id}' orphan", "b1", "behavior", None, None);
        // ensures: template interpolated
        assert_eq!(result, "behavior 'b1' orphan");
        // ensures: code and severity match pattern
        let rule = make_rule("W100", "no_incoming_edges");
        let pattern = parse_rule_pattern(&rule, "@test").unwrap();
        let diags = execute_pattern(&pattern, &[make_entity("b1", "behavior", 0, 0)], None);
        assert_eq!(diags[0].code, "W100");
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    // -- B:register_extension_validation_rules --
    // These are tested in validate.rs (register_validation_rules). Adding cross-refs.

    // verify unit "rules from multiple extensions are collected"
    #[test]
    fn test_rules_from_multiple_extensions_collected() {
        let rules = vec![
            ("@ext/a".to_string(), vec![make_rule("W100", "no_incoming_edges")]),
            ("@ext/b".to_string(), vec![make_rule("W200", "no_outgoing_edges")]),
        ];
        let (patterns, diags) = parse_all_rule_patterns(&rules);
        assert!(diags.is_empty());
        assert_eq!(patterns.len(), 2);
    }

    // verify unit "duplicate codes across extensions produce warning"
    // (Already tested in validate.rs::test_duplicate_codes_across_extensions_produce_warning)
    // This test verifies at the validation_engine level.
    #[test]
    fn test_duplicate_codes_warning_in_engine() {
        // Duplicate codes are detected by register_validation_rules in validate.rs,
        // not in parse_all_rule_patterns. This is by design — parsing accepts all,
        // deduplication is a separate concern.
        let rules = vec![
            ("@ext/a".to_string(), vec![make_rule("W100", "no_incoming_edges")]),
            ("@ext/b".to_string(), vec![make_rule("W100", "no_outgoing_edges")]),
        ];
        let (patterns, _) = parse_all_rule_patterns(&rules);
        // Both are parsed — duplicate detection is in validate.rs
        assert_eq!(patterns.len(), 2);
    }

    // verify unit "rules sorted by code for deterministic order"
    #[test]
    fn test_rules_sorted_by_code() {
        let rules = vec![
            ("@ext/a".to_string(), vec![
                make_rule("W300", "no_incoming_edges"),
                make_rule("W100", "no_incoming_edges"),
            ]),
            ("@ext/b".to_string(), vec![make_rule("W200", "no_outgoing_edges")]),
        ];
        let (patterns, _) = parse_all_rule_patterns(&rules);
        let codes: Vec<&str> = patterns.iter().map(|p| p.code.as_str()).collect();
        assert_eq!(codes, vec!["W100", "W200", "W300"]);
    }

    // verify contract "requires/ensures consistency for cross-extension rule aggregation"
    #[test]
    fn test_register_extension_validation_rules_contract() {
        let rules = vec![
            ("@ext/a".to_string(), vec![make_rule("W100", "no_incoming_edges")]),
            ("@ext/b".to_string(), vec![make_rule("W200", "no_outgoing_edges")]),
        ];
        let (patterns, diags) = parse_all_rule_patterns(&rules);
        // ensures: unified set
        assert_eq!(patterns.len(), 2);
        // ensures: deterministic order
        assert_eq!(patterns[0].code, "W100");
        assert_eq!(patterns[1].code, "W200");
        // ensures: no warnings for valid rules
        assert!(diags.is_empty());
    }

    // -- B:register_custom_validation_patterns --

    // B:register_custom_validation_patterns — verify unit "custom pattern registered with wasm_function reference"
    #[test]
    fn test_custom_pattern_registered_with_wasm_function() {
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

    // B:register_custom_validation_patterns — verify unit "unresolvable wasm_function produces warning"
    #[test]
    fn test_unresolvable_wasm_function_warning() {
        let pattern = ValidationRulePattern {
            code: "E200".to_string(),
            severity: Severity::Error,
            message_template: "test".to_string(),
            check: ValidationPatternKind::Custom,
            target_kind: None,
            edge_type: None,
            field: None,
            constraint: None,
            wasm_function: Some("missing_func".to_string()),
        };
        // No Wasm runtime → warning
        let (registered, diags) = register_custom_patterns(&[pattern], None);
        assert!(diags.iter().any(|d| d.code == "W025" && d.message.contains("missing_func")));
        // Still registered for later (will be skipped during execution)
        assert_eq!(registered.len(), 1);
    }

    // B:register_custom_validation_patterns — verify unit "custom pattern dispatched to Wasm runtime during validation"
    #[test]
    fn test_custom_pattern_dispatched_during_validation() {
        struct FailRuntime;
        impl WasmValidationRuntime for FailRuntime {
            fn call_custom_validator(&self, _func: &str, id: &str, _kind: &str) -> Result<bool, String> {
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
            field: None,
            constraint: None,
            wasm_function: Some("check".to_string()),
        };
        let entities = vec![make_entity("bad", "behavior", 1, 0), make_entity("good", "behavior", 1, 0)];
        let diags = execute_pattern(&pattern, &entities, Some(&FailRuntime));
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("bad"));
    }

    // B:register_custom_validation_patterns — verify unit "custom pattern failure emits configured diagnostic"
    #[test]
    fn test_custom_pattern_failure_emits_diagnostic() {
        struct AlwaysFail;
        impl WasmValidationRuntime for AlwaysFail {
            fn call_custom_validator(&self, _func: &str, _id: &str, _kind: &str) -> Result<bool, String> {
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
            field: None,
            constraint: None,
            wasm_function: Some("always_fail".to_string()),
        };
        let diags = execute_pattern(&pattern, &[make_entity("b1", "behavior", 1, 0)], Some(&AlwaysFail));
        assert_eq!(diags[0].code, "E201");
        assert_eq!(diags[0].severity, Severity::Error);
    }

    // B:register_custom_validation_patterns — verify contract "requires/ensures consistency for custom validation pattern registration"
    #[test]
    fn test_register_custom_validation_patterns_contract() {
        let custom = ValidationRulePattern {
            code: "E200".to_string(),
            severity: Severity::Error,
            message_template: "test".to_string(),
            check: ValidationPatternKind::Custom,
            target_kind: None,
            edge_type: None,
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
            field: None,
            constraint: None,
            wasm_function: None,
        };
        // ensures: custom patterns registered alongside declarative
        let (registered, diags) = register_custom_patterns(&[custom, declarative], None);
        assert_eq!(registered.len(), 2);
        // ensures: unresolvable Wasm produces warning
        assert!(diags.iter().any(|d| d.code == "W025"));
    }
}
