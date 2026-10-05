use specforge_common::{Diagnostic, Severity};
use specforge_protocol_types::ConstraintKind;
use specforge_protocol_types::{ValidationRuleDescriptor, ValidationSeverity};

/// Parsed and validated rule pattern, ready for execution.
#[derive(Debug, Clone)]
pub struct ValidationRulePattern {
    pub code: String,
    pub severity: Severity,
    pub message_template: String,
    pub check: ValidationPatternKind,
    pub target_kind: Option<String>,
    pub edge_type: Option<String>,
    /// For an edge-scoped `no_outgoing_edges` / `no_incoming_edges` rule:
    /// the kind at the far end of `edge_type`, so only edges to (or from)
    /// that kind count. Filled by [`resolve_edge_rules`].
    pub edge_peer_kind: Option<String>,
    pub field: Option<String>,
    pub constraint: Option<FieldConstraintPattern>,
    pub wasm_function: Option<String>,
}

/// What a rule checks — the extension vocabulary's [`CheckKind`], under
/// the name the validation engine has always used for it.
///
/// [`CheckKind`]: specforge_protocol_types::CheckKind
pub type ValidationPatternKind = specforge_protocol_types::CheckKind;

/// The statement that declares an entity's obligations.
const VERIFY_FIELD: &str = "verify";

#[derive(Debug, Clone)]
pub struct FieldConstraintPattern {
    /// `None` for a name this host does not read. Only
    /// `field_value_constraint` dispatches on the kind, and it rejects such
    /// a name at parse time; the other checks read `pattern` and `values`.
    pub kind: Option<ConstraintKind>,
    pub pattern: Option<String>,
    pub values: Vec<String>,
    /// Regex compiled once at parse time for `field_value_constraint` rules
    /// with a `matches` constraint, so execution never recompiles per entity.
    pub compiled_pattern: Option<regex::Regex>,
}

/// Verdict of a detailed custom validation call: pass, or fail with the
/// offending field/value so message templates can interpolate them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomVerdict {
    Pass,
    Fail {
        field: Option<String>,
        value: Option<String>,
    },
}

/// A stub trait for Wasm validation dispatch. Real implementation in specforge-wasm.
pub trait WasmValidationRuntime {
    fn call_custom_validator(
        &self,
        wasm_function: &str,
        entity_id: &str,
        entity_kind: &str,
    ) -> Result<bool, String>;

    /// Rich verdict variant: implementations that can localize the
    /// violation override this; the default delegates to the bool form.
    fn call_custom_validator_detailed(
        &self,
        wasm_function: &str,
        entity_id: &str,
        entity_kind: &str,
    ) -> Result<CustomVerdict, String> {
        Ok(
            match self.call_custom_validator(wasm_function, entity_id, entity_kind)? {
                true => CustomVerdict::Pass,
                false => CustomVerdict::Fail {
                    field: None,
                    value: None,
                },
            },
        )
    }
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

/// C6-12: diagnostic for a structurally impossible rule — one whose check
/// kind requires a field or constraint it does not carry, or whose
/// constraint can never match. Such a rule would execute as a silent no-op,
/// so it is rejected at parse time (W112) and never registered.
fn unexecutable_rule(extension_name: &str, rule_code: &str, why: &str) -> Diagnostic {
    Diagnostic {
        code: "W112".to_string(),
        severity: Severity::Warning,
        message: format!(
            "extension '{}': rule '{}': {} — the rule can never fire and was not registered",
            extension_name, rule_code, why
        ),
        span: None,
        suggestion: None,
        data: None,
    }
}

/// Parse a declared validation rule into a ValidationRulePattern.
/// Returns Ok(pattern) or Err(diagnostic) when the rule is unrecognized or
/// structurally cannot fire (missing field/constraint, empty values — W112).
#[allow(clippy::result_large_err)]
pub(crate) fn parse_rule_pattern(
    rule: &ValidationRuleDescriptor,
    extension_name: &str,
) -> Result<ValidationRulePattern, Diagnostic> {
    let Some(check) = ValidationPatternKind::parse(&rule.check) else {
        return Err(Diagnostic {
            code: "W112".to_string(),
            severity: Severity::Warning,
            message: format!(
                "extension '{}': unrecognized validation pattern kind '{}'",
                extension_name, rule.check
            ),
            span: None,
            suggestion: None,
            data: None,
        });
    };

    let severity = match rule.severity {
        ValidationSeverity::Error => Severity::Error,
        ValidationSeverity::Warning => Severity::Warning,
        ValidationSeverity::Info => Severity::Info,
    };

    // C6-12: structural validation. A rule missing the field or constraint
    // its check kind reads — or carrying a constraint shape that can never
    // match — would execute as a silent no-op. Reject it at parse time so
    // the misconfiguration is reported instead of shipping a dead rule.
    match check {
        ValidationPatternKind::FieldValueConstraint => match rule.constraint.as_ref() {
            None => {
                return Err(unexecutable_rule(
                    extension_name,
                    &rule.code,
                    "check 'field_value_constraint' requires a constraint but none is set",
                ));
            }
            Some(c) => match ConstraintKind::parse(&c.kind) {
                Some(ConstraintKind::NonEmpty) => {}
                Some(ConstraintKind::OneOf) if c.values.is_empty() => {
                    return Err(unexecutable_rule(
                        extension_name,
                        &rule.code,
                        "one_of constraint has an empty values list — every field value would be flagged as a violation",
                    ));
                }
                Some(ConstraintKind::Matches) if c.pattern.is_none() => {
                    return Err(unexecutable_rule(
                        extension_name,
                        &rule.code,
                        "matches constraint has no pattern — no value can ever be checked",
                    ));
                }
                Some(ConstraintKind::OneOf | ConstraintKind::Matches) => {}
                Some(ConstraintKind::WhenFieldEquals) | None => {
                    return Err(unexecutable_rule(
                        extension_name,
                        &rule.code,
                        &format!(
                            "unknown constraint kind '{}' for check 'field_value_constraint' (expected non_empty, one_of, or matches)",
                            c.kind
                        ),
                    ));
                }
            },
        },
        ValidationPatternKind::ConditionalFieldRequired => match rule.constraint.as_ref() {
            None => {
                return Err(unexecutable_rule(
                    extension_name,
                    &rule.code,
                    "check 'conditional_field_required' requires a constraint but none is set",
                ));
            }
            Some(c) => {
                if c.pattern.is_none() {
                    return Err(unexecutable_rule(
                        extension_name,
                        &rule.code,
                        "conditional_field_required requires constraint.pattern (the condition field) — without it the condition can never be met",
                    ));
                }
                if c.values.is_empty() {
                    return Err(unexecutable_rule(
                        extension_name,
                        &rule.code,
                        "conditional_field_required has an empty condition values list — the condition can never be met",
                    ));
                }
            }
        },
        ValidationPatternKind::MissingFieldWhenFlagSet
        | ValidationPatternKind::FileExists
        | ValidationPatternKind::MissingRequiredField
            if rule.field.is_none() =>
        {
            return Err(unexecutable_rule(
                extension_name,
                &rule.code,
                &format!("check '{check}' requires a field but none is set"),
            ));
        }
        ValidationPatternKind::Custom if rule.wasm_function.is_none() => {
            return Err(unexecutable_rule(
                extension_name,
                &rule.code,
                "check 'custom' requires a wasm_function but none is set",
            ));
        }
        _ => {}
    }

    let constraint = match rule.constraint.as_ref() {
        Some(c) => {
            let kind = ConstraintKind::parse(&c.kind);
            let compiled_pattern = if matches!(check, ValidationPatternKind::FieldValueConstraint)
                && kind == Some(ConstraintKind::Matches)
            {
                match c.pattern.as_deref().map(regex::Regex::new) {
                    Some(Ok(re)) => Some(re),
                    Some(Err(err)) => {
                        return Err(Diagnostic {
                            code: "W112".to_string(),
                            severity: Severity::Warning,
                            message: format!(
                                "extension '{}': rule '{}': invalid regex pattern '{}': {}",
                                extension_name,
                                rule.code,
                                c.pattern.as_deref().unwrap_or_default(),
                                err
                            ),
                            span: None,
                            suggestion: None,
                            data: None,
                        });
                    }
                    None => None,
                }
            } else {
                None
            };
            Some(FieldConstraintPattern {
                kind,
                pattern: c.pattern.clone(),
                values: c.values.clone(),
                compiled_pattern,
            })
        }
        None => None,
    };

    Ok(ValidationRulePattern {
        code: rule.code.clone(),
        severity,
        message_template: rule.message_template.clone(),
        check,
        target_kind: rule.target_kind.clone(),
        edge_type: rule.edge_type.clone(),
        edge_peer_kind: None,
        field: rule.field.clone(),
        constraint,
        wasm_function: rule.wasm_function.clone(),
    })
}

/// Scope edge rules to their declared `edge_type`.
///
/// A `no_outgoing_edges` rule on `BehaviorImplementsFeature` asks whether a
/// behavior implements a feature, not whether it references anything at
/// all, so it counts only edges to the edge type's target kind (its source
/// kind for `no_incoming_edges`). When no loaded extension declares that
/// kind, the edge can't exist in the project and the rule is dropped: a
/// project without `feature` can't be told to implement one.
pub(crate) fn resolve_edge_rules(
    patterns: &mut Vec<(ValidationRulePattern, String)>,
    edges: &crate::EdgeRegistry,
    kinds: &crate::KindRegistry,
) {
    patterns.retain_mut(|(pattern, _)| {
        let peer = match pattern.check {
            ValidationPatternKind::NoOutgoingEdges => pattern
                .edge_type
                .as_deref()
                .and_then(|label| edges.get(label))
                .and_then(|edge| edge.declared.target_kind.clone()),
            ValidationPatternKind::NoIncomingEdges => pattern
                .edge_type
                .as_deref()
                .and_then(|label| edges.get(label))
                .and_then(|edge| edge.declared.source_kind.clone()),
            _ => None,
        };
        let Some(peer) = peer else {
            return true;
        };
        if !kinds.contains(&peer) {
            return false;
        }
        pattern.edge_peer_kind = Some(peer);
        true
    });
}

/// Parse every declared rule into a validated pattern, paired with the
/// extension that declared it.
///
/// The origin is required to dispatch `check: "custom"` rules: the
/// `wasm_function` is an export of THAT extension's module, so the host must
/// know which runtime entry to call (WASM-only migration, Phase 5).
pub(crate) fn parse_all_rule_patterns(
    declared: &[(String, Vec<ValidationRuleDescriptor>)], // (ext_name, rules)
) -> (Vec<(ValidationRulePattern, String)>, Vec<Diagnostic>) {
    let mut patterns: Vec<(ValidationRulePattern, String)> = Vec::new();
    let mut diagnostics = Vec::new();

    for (ext_name, rules) in declared {
        for rule in rules {
            match parse_rule_pattern(rule, ext_name) {
                Ok(pattern) => patterns.push((pattern, ext_name.clone())),
                Err(diag) => diagnostics.push(diag),
            }
        }
    }

    // Sort by code for deterministic execution order
    patterns.sort_by(|a, b| a.0.code.cmp(&b.0.code));
    (patterns, diagnostics)
}

/// Interpolate a message template with entity context.
pub fn interpolate_template(
    template: &str,
    id: &str,
    kind: &str,
    field: Option<&str>,
    value: Option<&str>,
    allowed: Option<&str>,
) -> String {
    let mut result = template.replace("{id}", id).replace("{kind}", kind);
    if let Some(f) = field {
        result = result.replace("{field}", f);
    }
    if let Some(v) = value {
        result = result.replace("{value}", v);
    }
    if let Some(a) = allowed {
        result = result.replace("{allowed}", a);
    }
    result
}

/// A simple entity representation for validation.
///
/// Serialized as the input payload of extension-owned compiler passes
/// (`__pass_<name>` wasm exports).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationEntity {
    pub id: String,
    pub kind: String,
    pub fields: std::collections::HashMap<String, String>,
    pub incoming_edge_count: usize,
    pub outgoing_edge_count: usize,
    pub span: specforge_common::SourceSpan,
    /// Kinds of the entity's verify statements (unit/integration/property/...),
    /// used by [`ValidationPatternKind::VerifyKindAllowlist`].
    #[serde(default)]
    pub verify_kinds: Vec<String>,
    /// The verify statements' texts, parallel to `verify_kinds`.
    #[serde(default)]
    pub verify_texts: Vec<String>,
    /// Outgoing edges by the kind of the entity they reach, for edge-scoped
    /// rules ([`ValidationRulePattern::edge_peer_kind`]).
    #[serde(skip)]
    pub outgoing_kinds: std::collections::BTreeMap<String, usize>,
    /// Incoming edges by the kind of the entity they come from.
    #[serde(skip)]
    pub incoming_kinds: std::collections::BTreeMap<String, usize>,
    /// The entity owes no obligations of its own: a union type, which has
    /// no body to hold them, or an entity marked `abstract true` through a
    /// field its kind's registry entry declares. The host decides it from
    /// the entity's structure and the field registry, never from a field's
    /// name alone, so a struct member named `abstract` or `gherkin`
    /// exempts nothing.
    #[serde(default)]
    pub obligation_exempt: bool,
}

impl ValidationEntity {
    /// Edges out of (`outgoing`) or into this entity, only those to or from
    /// `peer_kind` when it is set.
    fn edge_count(&self, outgoing: bool, peer_kind: Option<&str>) -> usize {
        let (total, by_kind) = if outgoing {
            (self.outgoing_edge_count, &self.outgoing_kinds)
        } else {
            (self.incoming_edge_count, &self.incoming_kinds)
        };
        match peer_kind {
            Some(kind) => by_kind.get(kind).copied().unwrap_or(0),
            None => total,
        }
    }
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
        let mut violation_field: Option<String> = None;
        let mut violation_value: Option<String> = None;
        let violated = match pattern.check {
            ValidationPatternKind::NoIncomingEdges => {
                entity.edge_count(false, pattern.edge_peer_kind.as_deref()) == 0
            }
            ValidationPatternKind::NoOutgoingEdges => {
                entity.edge_count(true, pattern.edge_peer_kind.as_deref()) == 0
            }
            ValidationPatternKind::NoEdges => {
                entity.incoming_edge_count == 0 && entity.outgoing_edge_count == 0
            }
            ValidationPatternKind::MissingFieldWhenFlagSet => {
                if let Some(ref field_name) = pattern.field {
                    // An entity that owes no obligations (a union, which has
                    // no body to hold them, or one an extension's flag
                    // exempts: `obligation_exempt`) is not missing `verify`.
                    if field_name == VERIFY_FIELD && entity.obligation_exempt {
                        false
                    } else {
                        !entity.fields.contains_key(field_name)
                    }
                } else {
                    false
                }
            }
            ValidationPatternKind::FieldValueConstraint => {
                if let (Some(field_name), Some(constraint)) = (&pattern.field, &pattern.constraint)
                {
                    if let Some(value) = entity.fields.get(field_name) {
                        match constraint.kind {
                            Some(ConstraintKind::NonEmpty) => value.is_empty(),
                            Some(ConstraintKind::OneOf) => !constraint.values.contains(value),
                            Some(ConstraintKind::Matches) => {
                                // The regex was compiled once at parse time; a
                                // malformed pattern is rejected at load time with
                                // a W112 diagnostic, so `None` here only means the
                                // rule never carried a pattern (not a violation).
                                match &constraint.compiled_pattern {
                                    Some(re) => !re.is_match(value),
                                    None => false,
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
            ValidationPatternKind::ConditionalFieldRequired => {
                // When constraint.pattern (condition field) equals one of constraint.values,
                // then pattern.field (required field) must be present and non-empty.
                if let (Some(required_field), Some(constraint)) =
                    (&pattern.field, &pattern.constraint)
                {
                    if let Some(condition_field) = &constraint.pattern {
                        if let Some(condition_value) = entity.fields.get(condition_field) {
                            // Check if the condition value matches one of the trigger values
                            let condition_met = constraint.values.contains(condition_value);
                            if condition_met {
                                // Condition met — required field must be present and non-empty
                                match entity.fields.get(required_field) {
                                    None => true,            // field missing => violation
                                    Some(v) => v.is_empty(), // empty => violation
                                }
                            } else {
                                false // condition not met, no violation
                            }
                        } else {
                            false // condition field not present on entity
                        }
                    } else {
                        false // no condition field configured
                    }
                } else {
                    false // misconfigured pattern
                }
            }
            ValidationPatternKind::MissingRequiredField => {
                if let Some(ref field_name) = pattern.field {
                    !entity.fields.contains_key(field_name)
                } else {
                    false
                }
            }
            ValidationPatternKind::VerifyKindAllowlist => {
                let allowlist: Vec<String> = pattern
                    .constraint
                    .as_ref()
                    .map(|c| c.values.clone())
                    .unwrap_or_default();
                let offender = entity
                    .verify_kinds
                    .iter()
                    .filter(|k| !k.is_empty()) // bare `verify "..."` has no kind
                    .find(|k| !allowlist.contains(k))
                    .cloned();
                match offender {
                    Some(kind) => {
                        violation_value = Some(kind);
                        true
                    }
                    None => false,
                }
            }
            ValidationPatternKind::NoVerifyStatements => {
                // The obligations are the entity's `verify` statements (or
                // the field the declaring extension names instead). A
                // struct member named `verify` is a field, not a statement,
                // so it never stands in for one. Union types and abstract
                // entities owe none (`obligation_exempt`, which the host
                // sets from structure and the registry).
                let declared = match pattern.field.as_deref().unwrap_or(VERIFY_FIELD) {
                    VERIFY_FIELD => !entity.verify_texts.is_empty(),
                    field => entity.fields.contains_key(field),
                };
                !entity.obligation_exempt && !declared
            }
            ValidationPatternKind::Custom => {
                if let (Some(func), Some(rt)) = (&pattern.wasm_function, wasm) {
                    match rt.call_custom_validator_detailed(func, &entity.id, &entity.kind) {
                        Ok(CustomVerdict::Pass) => false,
                        Ok(CustomVerdict::Fail { field, value }) => {
                            violation_field = field;
                            violation_value = value;
                            true
                        }
                        // A runtime error means a broken or trapping export,
                        // already reported once (W112) by the probe that
                        // resolves the wasm_function when the extension's
                        // rules are loaded; repeating per entity would only
                        // spam.
                        Err(_) => false,
                    }
                } else {
                    false
                }
            }
        };

        if violated {
            let default_field = pattern.field.as_deref();
            let default_value = entity
                .fields
                .get(pattern.field.as_deref().unwrap_or(""))
                .map(|s| s.as_str());
            let (field, value) = match (&violation_field, &violation_value) {
                (Some(f), Some(v)) => (Some(f.as_str()), Some(v.as_str())),
                (Some(f), None) => (Some(f.as_str()), default_value),
                (None, Some(v)) => (default_field, Some(v.as_str())),
                _ => (default_field, default_value),
            };
            let allowed = if pattern.check == ValidationPatternKind::VerifyKindAllowlist {
                Some(
                    pattern
                        .constraint
                        .as_ref()
                        .map(|c| c.values.join(", "))
                        .unwrap_or_default(),
                )
            } else {
                None
            };
            let message = interpolate_template(
                &pattern.message_template,
                &entity.id,
                &entity.kind,
                field,
                value,
                allowed.as_deref(),
            );

            diagnostics.push(Diagnostic {
                code: pattern.code.clone(),
                severity: pattern.severity,
                message,
                span: Some(entity.span.clone()),
                suggestion: None,
                data: None,
            });
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::Sym;
    use specforge_protocol_types::FieldConstraintDescriptor;

    fn span() -> specforge_common::SourceSpan {
        specforge_common::SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 0,
        }
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
            obligation_exempt: false,
        }
    }

    fn allowlist_rule(code: &str, target: &str, allowed: &[&str]) -> ValidationRulePattern {
        ValidationRulePattern {
            code: code.to_string(),
            severity: Severity::Warning,
            message_template:
                "entity '{id}' has verify kind '{value}' not in allowed set {allowed}".to_string(),
            check: ValidationPatternKind::VerifyKindAllowlist,
            target_kind: Some(target.to_string()),
            edge_type: None,
            edge_peer_kind: None,
            field: None,
            constraint: Some(FieldConstraintPattern {
                kind: Some(ConstraintKind::OneOf),
                pattern: None,
                values: allowed.iter().map(|s| s.to_string()).collect(),
                compiled_pattern: None,
            }),
            wasm_function: None,
        }
    }

    fn entity_with_verify_kinds(id: &str, kind: &str, kinds: &[&str]) -> ValidationEntity {
        let mut e = make_entity(id, kind, 1, 1);
        e.verify_kinds = kinds.iter().map(|s| s.to_string()).collect();
        e
    }

    #[test]
    fn verify_kind_allowlist_flags_offending_kind() {
        let rule = allowlist_rule("W009", "invariant", &["property", "unit"]);
        let e = entity_with_verify_kinds("inv", "invariant", &["load"]);
        let diags = execute_pattern(&rule, &[e], None);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("load"), "{}", diags[0].message);
        assert!(
            diags[0].message.contains("property, unit"),
            "{}",
            diags[0].message
        );
    }

    #[test]
    fn verify_kind_allowlist_passes_allowed_and_exempt() {
        let rule = allowlist_rule("W009", "invariant", &["property", "unit", "mutation"]);
        // every kind allowed
        let ok = entity_with_verify_kinds("inv", "invariant", &["unit", "mutation"]);
        assert!(execute_pattern(&rule, &[ok], None).is_empty());
        // a bare `verify "..."` (empty kind) is exempt
        let bare = entity_with_verify_kinds("inv2", "invariant", &[""]);
        assert!(execute_pattern(&rule, &[bare], None).is_empty());
    }

    fn w004_rule() -> ValidationRulePattern {
        ValidationRulePattern {
            code: "W004".to_string(),
            severity: Severity::Warning,
            message_template: "{kind} '{id}' has no verify".to_string(),
            check: ValidationPatternKind::NoVerifyStatements,
            target_kind: Some("behavior".to_string()),
            edge_type: None,
            edge_peer_kind: None,
            field: Some("verify".to_string()),
            constraint: None,
            wasm_function: None,
        }
    }

    #[specforge_test_macros::test(
        behavior = "te_validate_unverified_testable",
        verify = "a field named like an obligation or an exemption exempts nothing from W004"
    )]
    fn w004_reads_statements_and_the_exemption_flag_not_field_names() {
        let rule = w004_rule();
        let unverified = make_entity("b1", "behavior", 1, 1);
        assert_eq!(execute_pattern(&rule, &[unverified], None).len(), 1);

        let mut verified = make_entity("b2", "behavior", 1, 1);
        verified.verify_kinds = vec!["unit".into()];
        verified.verify_texts = vec!["it works".into()];
        assert!(execute_pattern(&rule, &[verified], None).is_empty());

        // Members named like a statement or an exemption are fields: none
        // of them stands in for an obligation or exempts the entity.
        for (name, value) in [
            ("verify", "string"),
            ("gherkin", "string"),
            ("abstract", "true"),
            ("variants", "open | done"),
        ] {
            let mut named = make_entity("b3", "behavior", 1, 1);
            named.fields.insert(name.to_string(), value.to_string());
            assert_eq!(
                execute_pattern(&rule, &[named], None).len(),
                1,
                "a field named {name} exempts nothing"
            );
        }

        let mut exempt = make_entity("b4", "behavior", 1, 1);
        exempt.obligation_exempt = true;
        assert!(execute_pattern(&rule, &[exempt], None).is_empty());
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
        assert_eq!(
            pattern.check,
            ValidationPatternKind::MissingFieldWhenFlagSet
        );
        assert_eq!(pattern.field.as_deref(), Some("contract"));
    }

    // B:parse_validation_rule_pattern — verify unit "unrecognized pattern kind produces warning"
    #[test]
    fn test_unrecognized_pattern_kind_warning() {
        let rule = make_rule("W102", "invalid_check_kind");
        let result = parse_rule_pattern(&rule, "@test/ext");
        assert!(result.is_err());
        let diag = result.unwrap_err();
        assert_eq!(diag.code, "W112");
        assert!(diag.message.contains("invalid_check_kind"));
    }

    // B:parse_validation_rule_pattern — verify unit "all required fields validated on each rule"
    #[test]
    fn test_all_required_fields_validated() {
        // A valid rule must have code, severity, messageTemplate, check
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
    }

    // B:parse_validation_rule_pattern — verify contract "requires/ensures consistency for validation rule parsing"
    #[test]
    fn test_parse_validation_rule_pattern_contract() {
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

    // -- B:execute_validation_pattern --

    // B:execute_validation_pattern — verify unit "no_incoming_edges detects orphan entities"
    #[test]
    fn test_no_incoming_edges_detects_orphans() {
        let pattern = parse_rule_pattern(&make_rule("W100", "no_incoming_edges"), "@test").unwrap();
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

        e2.fields
            .insert("contract".to_string(), "some text".to_string());
        // b2 has "contract" → ok

        let diags = execute_pattern(&pattern, &[e1, e2], None);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("b1"));
    }

    // B:execute_validation_pattern — verify unit "field_value_constraint rejects invalid field value"
    #[test]
    fn test_field_value_constraint_rejects_invalid() {
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
        e1.fields
            .insert("status".to_string(), "invalid_status".to_string());
        let mut e2 = make_entity("b2", "behavior", 1, 0);
        e2.fields.insert("status".to_string(), "active".to_string());

        let diags = execute_pattern(&pattern, &[e1, e2], None);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("b1"));
    }

    // B:execute_validation_pattern — verify unit "matches constraint accepts a value satisfying the regex"
    #[test]
    fn test_matches_constraint_accepts_valid_semver() {
        let rule = ValidationRuleDescriptor {
            code: "W093".to_string(),
            severity: ValidationSeverity::Warning,
            message_template: "{kind} '{id}' has invalid {field}".to_string(),
            check: "field_value_constraint".to_string(),
            target_kind: Some("release".to_string()),
            edge_type: None,
            field: Some("version".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "matches".to_string(),
                pattern: Some(r"^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?(\+[a-zA-Z0-9.]+)?$".to_string()),
                values: vec![],
            }),
            wasm_function: None,
        };
        let pattern = parse_rule_pattern(&rule, "@test").unwrap();

        let mut e1 = make_entity("r1", "release", 1, 0);
        e1.fields.insert("version".to_string(), "1.0.0".to_string());

        let diags = execute_pattern(&pattern, &[e1], None);
        assert!(
            diags.is_empty(),
            "a valid semver value must satisfy the matches constraint, got: {:?}",
            diags
        );
    }

    // B:execute_validation_pattern — verify unit "matches constraint flags a value violating the regex"
    #[test]
    fn test_matches_constraint_flags_invalid_semver() {
        let rule = ValidationRuleDescriptor {
            code: "W093".to_string(),
            severity: ValidationSeverity::Warning,
            message_template: "{kind} '{id}' has invalid {field}".to_string(),
            check: "field_value_constraint".to_string(),
            target_kind: Some("release".to_string()),
            edge_type: None,
            field: Some("version".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "matches".to_string(),
                pattern: Some(r"^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?(\+[a-zA-Z0-9.]+)?$".to_string()),
                values: vec![],
            }),
            wasm_function: None,
        };
        let pattern = parse_rule_pattern(&rule, "@test").unwrap();

        let mut e1 = make_entity("r1", "release", 1, 0);
        e1.fields.insert("version".to_string(), "v1.2".to_string());

        let diags = execute_pattern(&pattern, &[e1], None);
        assert_eq!(diags.len(), 1, "a non-semver value must be flagged");
        assert!(diags[0].message.contains("r1"));
    }

    // C14: the matches regex compiles once at parse time and applies to every entity.
    #[test]
    fn test_matches_constraint_compiles_once_and_checks_all_entities() {
        let rule = ValidationRuleDescriptor {
            code: "W094".to_string(),
            severity: ValidationSeverity::Warning,
            message_template: "{kind} '{id}' has invalid {field}='{value}'".to_string(),
            check: "field_value_constraint".to_string(),
            target_kind: Some("release".to_string()),
            edge_type: None,
            field: Some("version".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "matches".to_string(),
                pattern: Some(r"^v\d+$".to_string()),
                values: vec![],
            }),
            wasm_function: None,
        };
        let pattern = parse_rule_pattern(&rule, "@test").unwrap();
        // The regex is compiled at parse time, not per entity at execution.
        let constraint = pattern.constraint.as_ref().unwrap();
        assert!(
            constraint.compiled_pattern.is_some(),
            "a matches constraint must carry a compiled regex after parsing"
        );

        let entities: Vec<ValidationEntity> = (0..50)
            .map(|i| {
                let mut e = make_entity(&format!("r{i}"), "release", 1, 0);
                let version = if i % 2 == 0 { "v1" } else { "bad" };
                e.fields.insert("version".to_string(), version.to_string());
                e
            })
            .collect();

        let diags = execute_pattern(&pattern, &entities, None);
        assert_eq!(
            diags.len(),
            25,
            "exactly the non-matching values are flagged"
        );
        assert!(diags.iter().all(|d| d.message.contains("version='bad'")));
    }

    // C14: a malformed regex is rejected at load time with a diagnostic and never executes.
    #[test]
    fn test_invalid_regex_pattern_fails_at_load_and_matches_nothing() {
        let rule = ValidationRuleDescriptor {
            code: "W095".to_string(),
            severity: ValidationSeverity::Warning,
            message_template: "{kind} '{id}' has invalid {field}".to_string(),
            check: "field_value_constraint".to_string(),
            target_kind: Some("release".to_string()),
            edge_type: None,
            field: Some("version".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "matches".to_string(),
                pattern: Some(r"(unclosed".to_string()),
                values: vec![],
            }),
            wasm_function: None,
        };

        let err = parse_rule_pattern(&rule, "@test").unwrap_err();
        assert_eq!(err.code, "W112");
        assert!(
            err.message.contains("W095"),
            "diagnostic names the rule: {}",
            err.message
        );
        assert!(
            err.message.contains("(unclosed"),
            "diagnostic names the bad pattern: {}",
            err.message
        );

        let declared = vec![("@test".to_string(), vec![rule.clone()])];
        let (patterns, diags) = parse_all_rule_patterns(&declared);
        assert!(
            patterns.is_empty(),
            "the invalid rule must not reach execution"
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W112");
    }

    // C6-12: a one_of constraint with no values misconfigures the allowlist.
    #[test]
    fn test_empty_one_of_values_rejected_at_parse() {
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
                values: vec![],
            }),
            wasm_function: None,
        };

        let err = parse_rule_pattern(&rule, "@test").unwrap_err();
        assert_eq!(err.code, "W112");
        assert!(err.message.contains("W103"), "{}", err.message);
        assert!(err.message.contains("one_of"), "{}", err.message);

        let declared = vec![("@test".to_string(), vec![rule])];
        let (patterns, diags) = parse_all_rule_patterns(&declared);
        assert!(
            patterns.is_empty(),
            "the misconfigured rule must not reach execution"
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W112");
    }

    // C6-12: a check kind that reads a field cannot run without one.
    #[test]
    fn test_missing_field_for_field_check_rejected_at_parse() {
        let rule = make_rule("W104", "missing_field_when_flag_set"); // field: None
        let err = parse_rule_pattern(&rule, "@test").unwrap_err();
        assert_eq!(err.code, "W112");
        assert!(err.message.contains("W104"), "{}", err.message);
        assert!(err.message.contains("requires a field"), "{}", err.message);
    }

    // C6-12: a matches constraint without a pattern can never check anything.
    #[test]
    fn test_matches_without_pattern_rejected_at_parse() {
        let rule = ValidationRuleDescriptor {
            code: "W105".to_string(),
            severity: ValidationSeverity::Warning,
            message_template: "{kind} '{id}' has invalid {field}".to_string(),
            check: "field_value_constraint".to_string(),
            target_kind: Some("release".to_string()),
            edge_type: None,
            field: Some("version".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "matches".to_string(),
                pattern: None,
                values: vec![],
            }),
            wasm_function: None,
        };
        let err = parse_rule_pattern(&rule, "@test").unwrap_err();
        assert_eq!(err.code, "W112");
        assert!(
            err.message.contains("matches constraint has no pattern"),
            "{}",
            err.message
        );
    }

    // C6-12: an unrecognized constraint kind never matches — reject loudly.
    #[test]
    fn test_unknown_constraint_kind_rejected_at_parse() {
        let rule = ValidationRuleDescriptor {
            code: "W106".to_string(),
            severity: ValidationSeverity::Warning,
            message_template: "{kind} '{id}' has invalid {field}".to_string(),
            check: "field_value_constraint".to_string(),
            target_kind: Some("behavior".to_string()),
            edge_type: None,
            field: Some("status".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "equals".to_string(),
                pattern: None,
                values: vec!["active".to_string()],
            }),
            wasm_function: None,
        };
        let err = parse_rule_pattern(&rule, "@test").unwrap_err();
        assert_eq!(err.code, "W112");
        assert!(
            err.message.contains("unknown constraint kind 'equals'"),
            "{}",
            err.message
        );
    }

    // C6-12: a conditional rule with no condition values can never trigger.
    #[test]
    fn test_conditional_field_required_empty_condition_values_rejected() {
        let rule = ValidationRuleDescriptor {
            code: "I059".to_string(),
            severity: ValidationSeverity::Info,
            message_template: "feature '{id}' has status 'deferred' but no reason".to_string(),
            check: "conditional_field_required".to_string(),
            target_kind: Some("feature".to_string()),
            edge_type: None,
            field: Some("reason".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "when_field_equals".to_string(),
                pattern: Some("status".to_string()),
                values: vec![],
            }),
            wasm_function: None,
        };
        let err = parse_rule_pattern(&rule, "@test").unwrap_err();
        assert_eq!(err.code, "W112");
        assert!(
            err.message.contains("empty condition values"),
            "{}",
            err.message
        );
    }

    // B:execute_validation_pattern — verify unit "matches constraint anchors the full value (not a substring)"
    #[test]
    fn test_matches_constraint_anchors_full_value() {
        let rule = ValidationRuleDescriptor {
            code: "W093".to_string(),
            severity: ValidationSeverity::Warning,
            message_template: "{kind} '{id}' has invalid {field}".to_string(),
            check: "field_value_constraint".to_string(),
            target_kind: Some("release".to_string()),
            edge_type: None,
            field: Some("version".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "matches".to_string(),
                pattern: Some(r"^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?(\+[a-zA-Z0-9.]+)?$".to_string()),
                values: vec![],
            }),
            wasm_function: None,
        };
        let pattern = parse_rule_pattern(&rule, "@test").unwrap();

        // Contains a valid semver as a substring but has trailing junk — the old
        // substring check would have wrongly accepted this.
        let mut e1 = make_entity("r1", "release", 1, 0);
        e1.fields
            .insert("version".to_string(), "1.0.0-not valid".to_string());

        let diags = execute_pattern(&pattern, &[e1], None);
        assert_eq!(diags.len(), 1, "anchored regex must reject trailing junk");
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
        assert!(
            diags.is_empty(),
            "cycle detection deferred to graph-aware caller"
        );
    }

    // B:execute_validation_pattern — verify unit "file_exists reports missing file-reference field targets"
    #[test]
    fn test_file_exists_reports_missing() {
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

        let mut entity = make_entity("b1", "behavior", 1, 0);
        entity.fields.insert(
            "gherkin".to_string(),
            "/nonexistent/file.feature".to_string(),
        );

        let diags = execute_pattern(&pattern, &[entity], None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E101");
    }

    // B:execute_validation_pattern — verify unit "custom pattern dispatches to registered Wasm function"
    #[test]
    fn test_custom_pattern_dispatches_to_wasm() {
        struct MockRuntime;
        impl WasmValidationRuntime for MockRuntime {
            fn call_custom_validator(
                &self,
                func: &str,
                id: &str,
                _kind: &str,
            ) -> Result<bool, String> {
                if func == "validate_naming" && id == "bad_name" {
                    Ok(false) // fails
                } else {
                    Ok(true) // passes
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
        let diags = execute_pattern(&pattern, &entities, Some(&MockRuntime));
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("bad_name"));
    }

    // B:execute_validation_pattern — verify unit "pattern violation produces diagnostic with configured code and severity"
    #[test]
    fn test_violation_produces_configured_diagnostic() {
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

    // B:emit_diagnostic_from_pattern — verify unit "message template interpolates {field} and {value}"
    #[test]
    fn test_template_interpolates_field_and_value() {
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

    // B:emit_diagnostic_from_pattern — verify unit "diagnostic code matches pattern code"
    #[test]
    fn test_diagnostic_code_matches_pattern() {
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
        let diags = execute_pattern(&pattern, &[make_entity("b1", "behavior", 0, 0)], None);
        assert_eq!(diags[0].code, "E999");
    }

    // B:emit_diagnostic_from_pattern — verify unit "diagnostic severity matches pattern severity"
    #[test]
    fn test_diagnostic_severity_matches_pattern() {
        for (sev_str, severity, expected) in [
            ("error", ValidationSeverity::Error, Severity::Error),
            ("warning", ValidationSeverity::Warning, Severity::Warning),
            ("info", ValidationSeverity::Info, Severity::Info),
        ] {
            let rule = ValidationRuleDescriptor {
                code: "X001".to_string(),
                severity,
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
                diags[0].severity, expected,
                "severity mismatch for {}",
                sev_str
            );
        }
    }

    // B:emit_diagnostic_from_pattern — verify contract "requires/ensures consistency for pattern diagnostic emission"
    #[test]
    fn test_emit_diagnostic_from_pattern_contract() {
        // requires: violation detected, pattern configured
        let result =
            interpolate_template("{kind} '{id}' orphan", "b1", "behavior", None, None, None);
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
            (
                "@ext/a".to_string(),
                vec![make_rule("W100", "no_incoming_edges")],
            ),
            (
                "@ext/b".to_string(),
                vec![make_rule("W200", "no_outgoing_edges")],
            ),
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
            (
                "@ext/a".to_string(),
                vec![make_rule("W100", "no_incoming_edges")],
            ),
            (
                "@ext/b".to_string(),
                vec![make_rule("W100", "no_outgoing_edges")],
            ),
        ];
        let (patterns, _) = parse_all_rule_patterns(&rules);
        // Both are parsed — duplicate detection is in validate.rs
        assert_eq!(patterns.len(), 2);
    }

    // verify unit "rules sorted by code for deterministic order"
    #[test]
    fn test_rules_sorted_by_code() {
        let rules = vec![
            (
                "@ext/a".to_string(),
                vec![
                    make_rule("W300", "no_incoming_edges"),
                    make_rule("W100", "no_incoming_edges"),
                ],
            ),
            (
                "@ext/b".to_string(),
                vec![make_rule("W200", "no_outgoing_edges")],
            ),
        ];
        let (patterns, _) = parse_all_rule_patterns(&rules);
        let codes: Vec<&str> = patterns.iter().map(|p| p.0.code.as_str()).collect();
        assert_eq!(codes, vec!["W100", "W200", "W300"]);
    }

    // verify contract "requires/ensures consistency for cross-extension rule aggregation"
    #[test]
    fn test_register_extension_validation_rules_contract() {
        let rules = vec![
            (
                "@ext/a".to_string(),
                vec![make_rule("W100", "no_incoming_edges")],
            ),
            (
                "@ext/b".to_string(),
                vec![make_rule("W200", "no_outgoing_edges")],
            ),
        ];
        let (patterns, diags) = parse_all_rule_patterns(&rules);
        // ensures: unified set
        assert_eq!(patterns.len(), 2);
        // ensures: deterministic order
        assert_eq!(patterns[0].0.code, "W100");
        assert_eq!(patterns[1].0.code, "W200");
        // ensures: no warnings for valid rules
        assert!(diags.is_empty());
    }

    // -- B:register_custom_validation_patterns --

    // B:register_custom_validation_patterns — verify unit "custom pattern registered with wasm_function reference"
    #[test]
    fn test_custom_pattern_registered_with_wasm_function() {
        let rule = ValidationRuleDescriptor {
            code: "E200".to_string(),
            severity: ValidationSeverity::Error,
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

    // A custom rule naming no wasm_function has nothing to dispatch to: it
    // is rejected at parse time (W112) instead of never firing.
    #[test]
    fn test_custom_rule_without_wasm_function_is_w112() {
        let rule = make_rule("E200", "custom");
        let err = parse_rule_pattern(&rule, "@test").unwrap_err();
        assert_eq!(err.code, "W112");
        assert!(err.message.contains("wasm_function"), "{}", err.message);
    }

    // B:register_custom_validation_patterns — verify unit "custom pattern dispatched to Wasm runtime during validation"
    #[test]
    fn test_custom_pattern_dispatched_during_validation() {
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

    // B:register_custom_validation_patterns — verify unit "custom pattern failure emits configured diagnostic"
    #[test]
    fn test_custom_pattern_failure_emits_diagnostic() {
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

    // -- B:conditional_field_required -- M1 fix: remove hardcoded CONDITIONAL_RULES

    // RED: parses "conditional_field_required" check kind
    #[test]
    fn test_parses_conditional_field_required() {
        let rule = ValidationRuleDescriptor {
            code: "I059".to_string(),
            severity: ValidationSeverity::Info,
            message_template: "feature '{id}' has status 'deferred' but no reason".to_string(),
            check: "conditional_field_required".to_string(),
            target_kind: Some("feature".to_string()),
            field: Some("reason".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "when_field_equals".to_string(),
                pattern: Some("status".to_string()),
                values: vec!["deferred".to_string()],
            }),
            edge_type: None,
            wasm_function: None,
        };
        let pattern = parse_rule_pattern(&rule, "@specforge/product").unwrap();
        assert_eq!(
            pattern.check,
            ValidationPatternKind::ConditionalFieldRequired
        );
        assert_eq!(pattern.field.as_deref(), Some("reason"));
        assert_eq!(
            pattern.constraint.as_ref().unwrap().kind,
            Some(ConstraintKind::WhenFieldEquals)
        );
        assert_eq!(
            pattern.constraint.as_ref().unwrap().pattern.as_deref(),
            Some("status")
        );
        assert_eq!(
            pattern.constraint.as_ref().unwrap().values,
            vec!["deferred"]
        );
    }

    // RED: conditional_field_required fires when condition met and field missing
    #[test]
    fn test_conditional_field_required_fires_when_condition_met_field_missing() {
        let rule = ValidationRuleDescriptor {
            code: "I059".to_string(),
            severity: ValidationSeverity::Info,
            message_template: "feature '{id}' has status 'deferred' but no reason".to_string(),
            check: "conditional_field_required".to_string(),
            target_kind: Some("feature".to_string()),
            field: Some("reason".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "when_field_equals".to_string(),
                pattern: Some("status".to_string()),
                values: vec!["deferred".to_string()],
            }),
            edge_type: None,
            wasm_function: None,
        };
        let pattern = parse_rule_pattern(&rule, "@specforge/product").unwrap();

        // Entity has status=deferred but no reason field
        let mut entity = make_entity("my_feature", "feature", 1, 0);
        entity
            .fields
            .insert("status".to_string(), "deferred".to_string());

        let diags = execute_pattern(&pattern, &[entity], None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "I059");
        assert!(diags[0].message.contains("my_feature"));
    }

    // RED: conditional_field_required does NOT fire when condition not met
    #[test]
    fn test_conditional_field_required_silent_when_condition_not_met() {
        let rule = ValidationRuleDescriptor {
            code: "I059".to_string(),
            severity: ValidationSeverity::Info,
            message_template: "feature '{id}' has status 'deferred' but no reason".to_string(),
            check: "conditional_field_required".to_string(),
            target_kind: Some("feature".to_string()),
            field: Some("reason".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "when_field_equals".to_string(),
                pattern: Some("status".to_string()),
                values: vec!["deferred".to_string()],
            }),
            edge_type: None,
            wasm_function: None,
        };
        let pattern = parse_rule_pattern(&rule, "@specforge/product").unwrap();

        // Entity has status=active (not deferred), no reason field
        let mut entity = make_entity("my_feature", "feature", 1, 0);
        entity
            .fields
            .insert("status".to_string(), "active".to_string());

        let diags = execute_pattern(&pattern, &[entity], None);
        assert!(
            diags.is_empty(),
            "should not fire when condition value doesn't match"
        );
    }

    // RED: conditional_field_required does NOT fire when required field present
    #[test]
    fn test_conditional_field_required_silent_when_field_present() {
        let rule = ValidationRuleDescriptor {
            code: "I059".to_string(),
            severity: ValidationSeverity::Info,
            message_template: "feature '{id}' has status 'deferred' but no reason".to_string(),
            check: "conditional_field_required".to_string(),
            target_kind: Some("feature".to_string()),
            field: Some("reason".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "when_field_equals".to_string(),
                pattern: Some("status".to_string()),
                values: vec!["deferred".to_string()],
            }),
            edge_type: None,
            wasm_function: None,
        };
        let pattern = parse_rule_pattern(&rule, "@specforge/product").unwrap();

        // Entity has status=deferred AND reason field
        let mut entity = make_entity("my_feature", "feature", 1, 0);
        entity
            .fields
            .insert("status".to_string(), "deferred".to_string());
        entity
            .fields
            .insert("reason".to_string(), "Waiting for upstream".to_string());

        let diags = execute_pattern(&pattern, &[entity], None);
        assert!(
            diags.is_empty(),
            "should not fire when required field is present"
        );
    }

    // RED: conditional_field_required does NOT fire when condition field absent
    #[test]
    fn test_conditional_field_required_silent_when_condition_field_absent() {
        let rule = ValidationRuleDescriptor {
            code: "I059".to_string(),
            severity: ValidationSeverity::Info,
            message_template: "feature '{id}' has status 'deferred' but no reason".to_string(),
            check: "conditional_field_required".to_string(),
            target_kind: Some("feature".to_string()),
            field: Some("reason".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "when_field_equals".to_string(),
                pattern: Some("status".to_string()),
                values: vec!["deferred".to_string()],
            }),
            edge_type: None,
            wasm_function: None,
        };
        let pattern = parse_rule_pattern(&rule, "@specforge/product").unwrap();

        // Entity has no status field at all
        let entity = make_entity("my_feature", "feature", 1, 0);

        let diags = execute_pattern(&pattern, &[entity], None);
        assert!(
            diags.is_empty(),
            "should not fire when condition field is absent"
        );
    }

    // -- B:missing_required_field --

    #[test]
    fn test_parses_missing_required_field() {
        let mut rule = make_rule("E006", "missing_required_field");
        rule.severity = ValidationSeverity::Error;
        rule.field = Some("contract".to_string());
        rule.message_template = "behavior '{id}' is missing required field 'contract'".to_string();
        let pattern = parse_rule_pattern(&rule, "@specforge/software").unwrap();
        assert_eq!(pattern.check, ValidationPatternKind::MissingRequiredField);
        assert_eq!(pattern.field.as_deref(), Some("contract"));
        assert_eq!(pattern.severity, Severity::Error);
    }

    #[test]
    fn test_missing_required_field_fires_when_absent() {
        let mut rule = make_rule("E006", "missing_required_field");
        rule.severity = ValidationSeverity::Error;
        rule.field = Some("contract".to_string());
        rule.message_template = "behavior '{id}' is missing required field 'contract'".to_string();
        let pattern = parse_rule_pattern(&rule, "@specforge/software").unwrap();

        let entity = make_entity("my_beh", "behavior", 1, 0);
        let diags = execute_pattern(&pattern, &[entity], None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E006");
        assert_eq!(diags[0].severity, Severity::Error);
        assert!(diags[0].message.contains("my_beh"));
        assert!(diags[0].message.contains("contract"));
    }

    #[test]
    fn test_missing_required_field_silent_when_present() {
        let mut rule = make_rule("E006", "missing_required_field");
        rule.severity = ValidationSeverity::Error;
        rule.field = Some("contract".to_string());
        rule.message_template = "behavior '{id}' is missing required field 'contract'".to_string();
        let pattern = parse_rule_pattern(&rule, "@specforge/software").unwrap();

        let mut entity = make_entity("my_beh", "behavior", 1, 0);
        entity
            .fields
            .insert("contract".to_string(), "Handles user login".to_string());
        let diags = execute_pattern(&pattern, &[entity], None);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_missing_required_field_only_targets_matching_kind() {
        let mut rule = make_rule("E006", "missing_required_field");
        rule.severity = ValidationSeverity::Error;
        rule.field = Some("contract".to_string());
        rule.message_template = "behavior '{id}' is missing required field 'contract'".to_string();
        let pattern = parse_rule_pattern(&rule, "@specforge/software").unwrap();

        // behavior without contract → fire
        let beh = make_entity("my_beh", "behavior", 1, 0);
        // event without contract → skip (different kind)
        let evt = make_entity("my_evt", "event", 1, 0);
        let diags = execute_pattern(&pattern, &[beh, evt], None);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("my_beh"));
    }
}
