//! What each check reads, resolved, and how it runs over the entity
//! snapshot's records: per entity for every check but cycles, which walk
//! the input's edges.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use specforge_common::Diagnostic;
use specforge_common::cycles::{CycleOptions, find_cycles};

use super::Rule;
use super::verdicts::{CustomCall, CustomVerdicts, Subject, Verdict, VerdictError};
use crate::entity::{Direction, EntityRecord, RuleInput};

/// One variant per check kind; each carries what it reads, resolved.
#[derive(Debug, Clone)]
pub(crate) enum Check {
    NoIncomingEdges(EdgeScope),
    NoOutgoingEdges(EdgeScope),
    NoEdges,
    /// `missing_field_when_flag_set` (`skip_owing_nothing` when the field is
    /// `verify`: an entity that owes no statements is not missing them) and
    /// `missing_required_field`.
    MissingField {
        field: String,
        skip_owing_nothing: bool,
    },
    FieldValue {
        field: String,
        constraint: ValueConstraint,
    },
    /// When `when_field` holds one of `equals`, `field` must be present and
    /// non-empty.
    ConditionalFieldRequired {
        field: String,
        when_field: String,
        equals: Vec<String>,
    },
    /// The entities on a cycle of the edges labelled one of `labels` (the
    /// fields that write the edge type). No labels: it reports nothing.
    Cycle {
        labels: Vec<String>,
    },
    FileExists {
        field: String,
    },
    VerifyKindAllowlist {
        allowed: Vec<String>,
    },
    NoVerifyStatements {
        obligations: Obligations,
    },
    Custom {
        extension: String,
        function: String,
    },
}

/// Which edges an edge rule counts.
#[derive(Debug, Clone)]
pub(crate) enum EdgeScope {
    /// Every edge.
    Any,
    /// Only edges to (outgoing) or from (incoming) an entity of this kind:
    /// the far end of the rule's edge type.
    Peer(String),
}

/// A `field_value_constraint`'s predicate on the field's text.
#[derive(Debug, Clone)]
pub(crate) enum ValueConstraint {
    NonEmpty,
    OneOf(Vec<String>),
    /// Compiled once when the rule is built.
    Matches(regex::Regex),
}

/// Where a `no_verify_statements` rule reads an entity's obligations.
#[derive(Debug, Clone)]
pub(crate) enum Obligations {
    /// Its `verify` statements.
    Statements,
    /// A field the declaring extension names instead.
    Field(String),
}

/// A violation's `{field}` and `{value}`, when the check names them.
#[derive(Default)]
struct Violation {
    field: Option<String>,
    value: Option<String>,
}

impl Violation {
    fn value(value: impl Into<String>) -> Self {
        Violation {
            field: None,
            value: Some(value.into()),
        }
    }
}

/// `rule` over `input`: its violations, entities by id.
pub(super) fn run(
    rule: &Rule,
    input: &RuleInput<'_>,
    verdicts: &dyn CustomVerdicts,
) -> Vec<Diagnostic> {
    if let Check::Cycle { labels } = &rule.check {
        return cycles(rule, labels, input);
    }
    let mut diagnostics = Vec::new();
    for record in input.entities.iter().filter(|e| rule.applies_to(&e.kind)) {
        let violation = match evaluate(rule, record, input, verdicts) {
            Ok(Some(violation)) => violation,
            Ok(None) => continue,
            // No verdict: the entity is not checked.
            Err(_) => continue,
        };
        diagnostics.push(report(rule, record, violation));
    }
    diagnostics
}

/// Whether `record` violates `rule`, and what the violation names. Err: a
/// custom rule got no verdict.
fn evaluate(
    rule: &Rule,
    record: &EntityRecord,
    input: &RuleInput<'_>,
    verdicts: &dyn CustomVerdicts,
) -> Result<Option<Violation>, VerdictError> {
    let violated = |violated: bool| Ok(violated.then(Violation::default));
    match &rule.check {
        Check::NoIncomingEdges(scope) => {
            violated(record.edges(Direction::Incoming, scope.peer()) == 0)
        }
        Check::NoOutgoingEdges(scope) => {
            violated(record.edges(Direction::Outgoing, scope.peer()) == 0)
        }
        Check::NoEdges => violated(
            record.edges(Direction::Incoming, None) == 0
                && record.edges(Direction::Outgoing, None) == 0,
        ),
        Check::MissingField {
            field,
            skip_owing_nothing,
        } => {
            // An entity that owes no obligations (a union, which has no
            // body to hold them, one an extension's flag exempts, or one
            // whose kind accepts no `verify`) is not missing `verify`.
            violated(!(*skip_owing_nothing && record.exempts_statements()) && !record.writes(field))
        }
        Check::FieldValue { field, constraint } => violated(
            // A field that is not written breaks no constraint.
            record.field(field).is_some_and(|value| match constraint {
                ValueConstraint::NonEmpty => value.is_empty(),
                ValueConstraint::OneOf(allowed) => !allowed.iter().any(|a| a == value),
                ValueConstraint::Matches(regex) => !regex.is_match(value),
            }),
        ),
        Check::ConditionalFieldRequired {
            field,
            when_field,
            equals,
        } => violated(
            record
                .field(when_field)
                .is_some_and(|condition| equals.iter().any(|v| v == condition))
                && record.field(field).is_none_or(str::is_empty),
        ),
        Check::Cycle { .. } => Ok(None),
        Check::FileExists { field } => violated(
            // A relative path is the spec root's; an absolute one is checked
            // as written (`join` keeps it).
            record
                .field(field)
                .is_some_and(|path| !input.spec_root.join(path).exists()),
        ),
        Check::VerifyKindAllowlist { allowed } => Ok(record
            .obligations
            .iter()
            .map(|o| &o.kind)
            // A bare `verify "..."` has no kind.
            .filter(|kind| !kind.is_empty())
            .find(|kind| !allowed.contains(kind))
            .map(|kind| Violation::value(kind.clone()))),
        Check::NoVerifyStatements { obligations } => violated(match obligations {
            // A struct member named `verify` is a field, not a statement,
            // so it never stands in for one; what exempts an entity is its
            // record's exemption, never a field's name.
            Obligations::Statements => {
                !record.exempts_statements() && record.obligations.is_empty()
            }
            Obligations::Field(field) => !record.exempts_fields() && !record.writes(field),
        }),
        Check::Custom {
            extension,
            function,
        } => match verdicts.verdict(CustomCall {
            extension,
            function,
            subject: Subject::Entity(record),
        })? {
            Verdict::Pass => Ok(None),
            Verdict::Fail { field, value } => Ok(Some(Violation { field, value })),
        },
    }
}

impl EdgeScope {
    fn peer(&self) -> Option<&str> {
        match self {
            EdgeScope::Any => None,
            EdgeScope::Peer(kind) => Some(kind),
        }
    }
}

/// The diagnostic of `rule` on `record`: `{field}` and `{value}` are the
/// violation's when it names them, else the rule's field and its text.
fn report(rule: &Rule, record: &EntityRecord, violation: Violation) -> Diagnostic {
    let default_field = rule.message_field.as_deref();
    let default_value = record.field(default_field.unwrap_or(""));
    let field = violation.field.as_deref().or(default_field);
    let value = violation.value.as_deref().or(default_value);
    let allowed = match &rule.check {
        Check::VerifyKindAllowlist { allowed } => Some(allowed.join(", ")),
        _ => None,
    };
    let message = interpolate(
        &rule.template,
        &record.id,
        &record.kind,
        field,
        value,
        allowed.as_deref(),
    );
    diagnostic(rule, message, record)
}

fn diagnostic(rule: &Rule, message: String, record: &EntityRecord) -> Diagnostic {
    Diagnostic {
        code: rule.code.clone(),
        severity: rule.severity,
        message,
        span: Some(record.span.clone()),
        suggestion: None,
        data: None,
    }
}

/// `template` with `{id}`, `{kind}`, and `{field}`, `{value}`, `{allowed}`
/// when given; a placeholder with nothing to put in stays as written.
fn interpolate(
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

/// A cycle rule over `input`: each entity it applies to (every entity when
/// it names no target kind) that sits on a cycle of the edges labelled one
/// of `labels` between such entities, by id. `{kind}` is the member's own
/// kind; `{field}` and `{value}` default to the rule's field, as for every
/// check.
fn cycles(rule: &Rule, labels: &[String], input: &RuleInput<'_>) -> Vec<Diagnostic> {
    if labels.is_empty() {
        return Vec::new();
    }
    let members: BTreeMap<&str, &EntityRecord> = input
        .entities
        .iter()
        .filter(|e| rule.applies_to(&e.kind))
        .map(|e| (e.id.as_str(), e))
        .collect();
    if members.is_empty() {
        return Vec::new();
    }
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for edge in input.edges {
        if labels.contains(&edge.label)
            && members.contains_key(edge.source.as_str())
            && members.contains_key(edge.target.as_str())
        {
            adjacency
                .entry(edge.source.clone())
                .or_default()
                .insert(edge.target.clone());
        }
    }
    // Seeds in id order (R-6: the walk must not depend on hash order).
    let seeds: Vec<String> = members.keys().map(|id| id.to_string()).collect();
    let (on_cycle, _) = find_cycles(&seeds, &adjacency, CycleOptions::default());
    on_cycle
        .iter()
        .filter_map(|id| members.get(id.as_str()))
        .map(|record| report(rule, record, Violation::default()))
        .collect()
}

/// The W112 of a custom rule whose function cannot answer the load-time
/// probe; `None` for any other rule, or when it answers (or no runtime can).
pub(super) fn probe(rule: &Rule, verdicts: &dyn CustomVerdicts) -> Option<Diagnostic> {
    let Check::Custom {
        extension,
        function,
    } = &rule.check
    else {
        return None;
    };
    let call = CustomCall {
        extension,
        function,
        subject: Subject::Probe {
            kind: rule.target.as_deref(),
        },
    };
    match verdicts.verdict(call) {
        Ok(_) | Err(VerdictError::Unavailable) => None,
        Err(VerdictError::Failed(error)) => Some(
            Diagnostic::warning(
                "W112",
                format!(
                    "extension '{extension}': rule '{}': wasm_function '{function}' could not be resolved ({error}) — the rule will not fire",
                    rule.code
                ),
            )
            .with_suggestion(format!(
                "export '{function}' from '{extension}', or fix the rule's wasm_function"
            )),
        ),
    }
}

/// The paths a `file_exists` rule reads on `input`, against the spec root.
pub(super) fn files(rule: &Rule, input: &RuleInput<'_>) -> Vec<PathBuf> {
    let Check::FileExists { field } = &rule.check else {
        return Vec::new();
    };
    input
        .entities
        .iter()
        .filter(|e| rule.applies_to(&e.kind))
        .filter_map(|e| e.field(field))
        .map(|path| input.spec_root.join(path))
        .collect()
}
