//! Declared descriptor → [`Rule`]: its shape checked (W112 for a rule that
//! cannot work as declared), its references resolved against the
//! registries, then the host's E006 rules and W023 for a code two
//! extensions declare.

use std::collections::{BTreeSet, HashMap};

use specforge_common::{Diagnostic, Severity};
use specforge_protocol_types::{
    CheckKind, ConstraintKind, ExtensionDeclaration, ValidationRuleDescriptor, ValidationSeverity,
};

use super::check::{Check, EdgeScope, Obligations, ValueConstraint};
use super::{Declared, DeclaredConstraint, Origin, Registries, Rule, Rules};

/// The statement that declares an entity's obligations.
const VERIFY_FIELD: &str = "verify";

pub(super) fn build(
    declarations: &[ExtensionDeclaration],
    registries: Registries<'_>,
) -> (Rules, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut rules = Vec::new();
    for declaration in declarations {
        let extension = declaration.name();
        for descriptor in &declaration.validation_rules {
            // Plan 11-T7: the check that an extension may report this code
            // (`check_extension_code`, W150) runs here, per declared rule.
            match shape(descriptor, extension) {
                Ok(rule) => rules.extend(resolve(rule, registries)),
                Err(w112) => diagnostics.push(w112),
            }
        }
    }
    // Code order (stable: a code declared for several kinds keeps its
    // declaration order).
    rules.sort_by(|a: &Rule, b: &Rule| a.code.cmp(&b.code));
    diagnostics.extend(duplicate_codes(declarations));
    rules.extend(required_field_rules(registries));
    (Rules { rules }, diagnostics)
}

/// W112 for `extension`'s rule `code`: it cannot work as declared (`why`),
/// so it is not registered.
fn cannot_fire(extension: &str, code: &str, why: &str) -> Diagnostic {
    Diagnostic::warning(
        "W112",
        format!(
            "extension '{extension}': rule '{code}': {why} — the rule can never fire and was not registered"
        ),
    )
}

/// The rule `descriptor` declares, with the check its kind names and the
/// properties that check reads; W112 when it cannot work as declared.
/// Edge rules are resolved afterwards ([`resolve`]).
#[allow(clippy::result_large_err)]
fn shape(descriptor: &ValidationRuleDescriptor, extension: &str) -> Result<Rule, Diagnostic> {
    let r = descriptor;
    let Some(check_kind) = CheckKind::parse(&r.check) else {
        return Err(Diagnostic::warning(
            "W112",
            format!(
                "extension '{extension}': unrecognized validation pattern kind '{}'",
                r.check
            ),
        ));
    };
    let fail = |why: &str| cannot_fire(extension, &r.code, why);
    let requires_field = || {
        r.field.clone().ok_or_else(|| {
            fail(&format!(
                "check '{check_kind}' requires a field but none is set"
            ))
        })
    };

    let check = match check_kind {
        // Resolved against the edge registry by `resolve`.
        CheckKind::NoIncomingEdges => Check::NoIncomingEdges(EdgeScope::Any),
        CheckKind::NoOutgoingEdges => Check::NoOutgoingEdges(EdgeScope::Any),
        CheckKind::NoEdges => Check::NoEdges,
        CheckKind::MissingFieldWhenFlagSet => {
            let field = requires_field()?;
            Check::MissingField {
                skip_owing_nothing: field == VERIFY_FIELD,
                field,
            }
        }
        CheckKind::MissingRequiredField => Check::MissingField {
            field: requires_field()?,
            skip_owing_nothing: false,
        },
        CheckKind::FileExists => Check::FileExists {
            field: requires_field()?,
        },
        CheckKind::FieldValueConstraint => {
            let Some(c) = r.constraint.as_ref() else {
                return Err(fail(
                    "check 'field_value_constraint' requires a constraint but none is set",
                ));
            };
            let constraint = match ConstraintKind::parse(&c.kind) {
                Some(ConstraintKind::NonEmpty) => ValueConstraint::NonEmpty,
                Some(ConstraintKind::OneOf) if c.values.is_empty() => {
                    return Err(fail(
                        "one_of constraint has an empty values list — every field value would be flagged as a violation",
                    ));
                }
                Some(ConstraintKind::OneOf) => ValueConstraint::OneOf(c.values.clone()),
                Some(ConstraintKind::Matches) => {
                    let Some(pattern) = c.pattern.as_deref() else {
                        return Err(fail(
                            "matches constraint has no pattern — no value can ever be checked",
                        ));
                    };
                    match regex::Regex::new(pattern) {
                        Ok(regex) => ValueConstraint::Matches(regex),
                        Err(err) => {
                            return Err(Diagnostic::warning(
                                "W112",
                                format!(
                                    "extension '{extension}': rule '{}': invalid regex pattern '{pattern}': {err}",
                                    r.code
                                ),
                            ));
                        }
                    }
                }
                Some(ConstraintKind::WhenFieldEquals) | None => {
                    return Err(fail(&format!(
                        "unknown constraint kind '{}' for check 'field_value_constraint' (expected non_empty, one_of, or matches)",
                        c.kind
                    )));
                }
            };
            Check::FieldValue {
                // PIN (plan 02 T4): without a field it reads no text and never fires.
                field: r.field.clone().unwrap_or_default(),
                constraint,
            }
        }
        CheckKind::ConditionalFieldRequired => {
            let Some(c) = r.constraint.as_ref() else {
                return Err(fail(
                    "check 'conditional_field_required' requires a constraint but none is set",
                ));
            };
            let Some(when_field) = c.pattern.clone() else {
                return Err(fail(
                    "conditional_field_required requires constraint.pattern (the condition field) — without it the condition can never be met",
                ));
            };
            if c.values.is_empty() {
                return Err(fail(
                    "conditional_field_required has an empty condition values list — the condition can never be met",
                ));
            }
            Check::ConditionalFieldRequired {
                // PIN (plan 02 T4): without a field it reads no text and never fires.
                field: r.field.clone().unwrap_or_default(),
                when_field,
                equals: c.values.clone(),
            }
        }
        // Resolved against the field registry by `resolve`.
        CheckKind::CycleDetection => Check::Cycle { labels: Vec::new() },
        CheckKind::VerifyKindAllowlist => Check::VerifyKindAllowlist {
            allowed: r
                .constraint
                .as_ref()
                .map(|c| c.values.clone())
                .unwrap_or_default(),
        },
        CheckKind::NoVerifyStatements => Check::NoVerifyStatements {
            obligations: match r.field.as_deref().unwrap_or(VERIFY_FIELD) {
                VERIFY_FIELD => Obligations::Statements,
                field => Obligations::Field(field.to_string()),
            },
        },
        CheckKind::Custom => Check::Custom {
            extension: extension.to_string(),
            function: r
                .wasm_function
                .clone()
                .ok_or_else(|| fail("check 'custom' requires a wasm_function but none is set"))?,
        },
    };

    Ok(Rule {
        code: r.code.clone(),
        severity: match r.severity {
            ValidationSeverity::Error => Severity::Error,
            ValidationSeverity::Warning => Severity::Warning,
            ValidationSeverity::Info => Severity::Info,
        },
        template: r.message_template.clone(),
        target: r.target_kind.clone(),
        message_field: r.field.clone(),
        origin: Origin::Extension(extension.to_string()),
        check_kind,
        check,
        declared: Declared {
            edge_type: r.edge_type.clone(),
            constraint: r.constraint.as_ref().map(|c| DeclaredConstraint {
                kind: ConstraintKind::parse(&c.kind),
                pattern: c.pattern.clone(),
                values: c.values.clone(),
            }),
            wasm_function: r.wasm_function.clone(),
        },
    })
}

/// `rule` with its edge type resolved against the registries; `None` when
/// it cannot apply to this project.
///
/// An edge rule (`no_*_edges` with an edge type) counts only edges to the
/// edge type's target kind (from its source kind for `no_incoming_edges`):
/// a `no_outgoing_edges` rule on `BehaviorImplementsFeature` asks whether a
/// behavior implements a feature, not whether it references anything. When
/// no loaded extension declares that kind, the edge can't exist in the
/// project and the rule is dropped. A cycle rule follows the fields that
/// write its edge type.
fn resolve(mut rule: Rule, registries: Registries<'_>) -> Option<Rule> {
    let edge_type = rule.declared.edge_type.clone();
    match &mut rule.check {
        Check::NoIncomingEdges(scope) | Check::NoOutgoingEdges(scope) => {
            let incoming = matches!(rule.check_kind, CheckKind::NoIncomingEdges);
            let peer = edge_type
                .as_deref()
                .and_then(|label| registries.edges.get(label))
                .and_then(|edge| {
                    if incoming {
                        edge.declared.source_kind.clone()
                    } else {
                        edge.declared.target_kind.clone()
                    }
                });
            if let Some(peer) = peer {
                if !registries.kinds.contains(&peer) {
                    return None;
                }
                *scope = EdgeScope::Peer(peer);
            }
        }
        Check::Cycle { labels } => {
            if let Some(edge_type) = edge_type {
                let writing: BTreeSet<String> = registries
                    .fields
                    .iter()
                    .filter(|(_, _, entry)| entry.declared.edge.as_deref() == Some(&edge_type))
                    .map(|(_, field, _)| field.to_string())
                    .collect();
                *labels = if writing.is_empty() {
                    // The raw label, as if a field wrote it.
                    vec![edge_type]
                } else {
                    writing.into_iter().collect()
                };
            }
        }
        _ => {}
    }
    Some(rule)
}

/// One E006 rule per `required` field, owned by the host, by (kind, field).
fn required_field_rules(registries: Registries<'_>) -> Vec<Rule> {
    let mut required: Vec<(String, String)> = registries
        .fields
        .iter()
        .filter(|(_, _, entry)| entry.declared.required)
        .map(|(kind, field, _)| (kind.to_string(), field.to_string()))
        .collect();
    required.sort();
    required
        .into_iter()
        .map(|(kind, field)| Rule {
            code: "E006".to_string(),
            severity: Severity::Error,
            template: format!("{kind} '{{id}}' is missing required field '{field}'"),
            target: Some(kind),
            message_field: Some(field.clone()),
            origin: Origin::Host,
            check_kind: CheckKind::MissingRequiredField,
            check: Check::MissingField {
                field,
                skip_owing_nothing: false,
            },
            declared: Declared::default(),
        })
        .collect()
}

/// W023: a validation rule code a later extension declares again, once
/// per repeat, naming the code, that extension and the first one. A code
/// one extension repeats is not reported. Every declared rule counts,
/// including one W112 rejects.
fn duplicate_codes(declarations: &[ExtensionDeclaration]) -> Vec<Diagnostic> {
    let mut first: HashMap<&str, &str> = HashMap::new();
    let mut diagnostics = Vec::new();
    for declaration in declarations {
        for rule in &declaration.validation_rules {
            match first.get(rule.code.as_str()) {
                None => {
                    first.insert(&rule.code, declaration.name());
                }
                Some(owner) if *owner != declaration.name() => {
                    diagnostics.push(Diagnostic::warning(
                        "W023",
                        format!(
                            "validation rule code '{}' from '{}' duplicates code from '{}'",
                            rule.code,
                            declaration.name(),
                            owner
                        ),
                    ));
                }
                Some(_) => {}
            }
        }
    }
    diagnostics
}
