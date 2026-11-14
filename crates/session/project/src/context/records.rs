//! The records a context route keeps of its pack (ADR 0356 D12): on each check, every record of a route with a stale
//! code, when its field's owner gate admits it, is evaluated again at its recorded budget. A pack whose digest moved,
//! a record with no subject or several, or a core above its budget is one finding of the route's stale code at the
//! field, with the record's suggestion. A project that holds no record evaluates no pack.

use specforge_common::{Diagnostic, DiagnosticData, Severity};
use specforge_diagnostics::{CodeRef, Level, letter_level};
use specforge_emitter::{EmitFormat, EmitterError, generate_schema};
use specforge_protocol_types::OwnerState;
use specforge_registry::{ContextRecord, ContextRoute, EntityRecord, FieldRecord};

use super::{Over, pack_over};
use crate::select::Selection;

/// A held record: the field written, the route it names, the digest and the budget it recorded.
struct Held<'a> {
    field: &'a FieldRecord,
    route: &'a str,
    digest: &'a str,
    budget: usize,
}

/// The record `field` of `entity` holds, when it holds the three members.
fn held<'a>(entity: &'a EntityRecord, field: &str) -> Option<Held<'a>> {
    let field = entity.fields.iter().find(|f| f.key == field)?;
    let value = field.written.as_ref()?;
    Some(Held {
        field,
        route: value.get("route")?.as_str()?,
        digest: value.get("digest")?.as_str()?,
        budget: usize::try_from(value.get("max_tokens")?.as_u64()?).ok()?,
    })
}

/// Why a record is stale.
enum Stale {
    Moved { subject: String, now: String },
    NoSubject,
    Several(Vec<String>),
    OverBudget { subject: String, why: String },
}

/// The findings of every stale record `over` holds.
pub(crate) fn stale(over: &Over<'_>) -> Vec<Diagnostic> {
    let registries = over.env.registries();
    let routes: Vec<&ContextRoute> = registries
        .contexts()
        .iter()
        .filter(|route| route.stale.is_some())
        .collect();
    if routes.is_empty() {
        return Vec::new();
    }
    let checks = registries.checks();
    let selection = Selection::of(over.graph, registries);
    let mut schema = None;
    let mut findings = Vec::new();
    for (entity, _) in over.entities.iter() {
        // Read once per entity, and only when it holds a gated record.
        let mut pending = None;
        for route in &routes {
            let Some(record) = route.records.iter().find(|r| r.kind == entity.kind) else {
                continue;
            };
            // The value names the route that recorded it: another route's record is that route's to check.
            let Some(held) = held(entity, &record.field).filter(|held| held.route == route.name)
            else {
                continue;
            };
            let admitted = match checks.gate(&entity.kind, &record.field) {
                None => true,
                Some(gate) => {
                    let pending = *pending.get_or_insert_with(|| over.entities.is_pending(entity));
                    pending == (gate == OwnerState::Pending)
                }
            };
            if !admitted {
                continue;
            }
            let subjects = selection.entities(&entity.id, &record.subject);
            let mut subjects = subjects.into_iter();
            let stale = match (subjects.next(), subjects.next()) {
                (None, _) => Some(Stale::NoSubject),
                (Some(first), Some(second)) => Some(Stale::Several(
                    [first, second].into_iter().chain(subjects).collect(),
                )),
                (Some(subject), None) => {
                    let schema = schema.get_or_insert_with(|| generate_schema(registries));
                    match pack_over(
                        over,
                        route,
                        &subject,
                        held.budget,
                        EmitFormat::Context,
                        schema,
                    ) {
                        Ok(packed) if packed.digest == held.digest => None,
                        Ok(packed) => Some(Stale::Moved {
                            subject,
                            now: packed.digest,
                        }),
                        Err(error @ EmitterError::BudgetTooSmall { .. }) => {
                            Some(Stale::OverBudget {
                                subject,
                                why: error.to_string(),
                            })
                        }
                        // A pack that cannot be rendered is a host failure, never the record's.
                        Err(_) => None,
                    }
                }
            };
            if let Some(stale) = stale {
                findings.push(finding(route, record, entity, &held, stale));
            }
        }
    }
    findings
}

/// The finding of a stale record: the route's stale code, its severity by its letter, at the field.
fn finding(
    route: &ContextRoute,
    record: &ContextRecord,
    entity: &EntityRecord,
    held: &Held<'_>,
    stale: Stale,
) -> Diagnostic {
    let code = route.stale.clone().unwrap_or_default();
    let severity = match letter_level(CodeRef::parse(&code).local) {
        Some(Level::Error) => Severity::Error,
        Some(Level::Info) => Severity::Info,
        _ => Severity::Warning,
    };
    let at = format!(
        "{} '{}': its {} records the pack of {}",
        entity.kind, entity.id, record.field, route.name
    );
    let (message, subject) = match stale {
        Stale::Moved { subject, now } => (
            format!(
                "{at} for '{subject}', which has moved since (recorded {}, now {now})",
                held.digest
            ),
            subject,
        ),
        Stale::NoSubject => (
            format!(
                "{at}, but it reaches no '{}' by '{}' now",
                route.subject, record.subject
            ),
            route.subject.clone(),
        ),
        Stale::Several(subjects) => (
            format!(
                "{at}, but it reaches several '{}' by '{}' now: {}",
                route.subject,
                record.subject,
                subjects.join(", ")
            ),
            route.subject.clone(),
        ),
        Stale::OverBudget { subject, why } => (
            format!(
                "{at} for '{subject}' at {} tokens, which no longer holds its core: {why}",
                held.budget
            ),
            subject,
        ),
    };
    let span = held
        .field
        .value_span
        .clone()
        .unwrap_or_else(|| entity.span.clone());
    let mut diagnostic = Diagnostic::from_extension(&route.extension, code, severity, message)
        .with_span(span)
        .with_data(DiagnosticData::Subject {
            entity: entity.id.clone(),
        });
    if let Some(suggestion) = &record.suggestion {
        diagnostic = diagnostic.with_suggestion(
            suggestion
                .replace("{entity}", &entity.id)
                .replace("{subject}", &subject)
                .replace("{route}", &route.name),
        );
    }
    diagnostic
}
