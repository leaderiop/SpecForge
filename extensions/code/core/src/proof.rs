//! The check pass `proof` (ADR 0329 D6, ADR 0347 D11): a linked test that fails, and a test or item naming no entity.

use std::collections::{BTreeSet, HashMap};

use specforge_extension_sdk::prelude::*;

use crate::refs::at;
use crate::{reference, ITEM_KIND, KINDS, NAMES, RESULT, TEST_KIND};

/// W401 on each entity outside `@code/core`'s kinds that is not pending and links a `test` whose result ([`RESULT`])
/// is `failed`, once per test it links; I402 on each name a `test` or an `item` holds in `names` that is dangling: no
/// entity of the project, or the short name of several; E101 on each reference a fact of `@code/core`'s kinds holds
/// that answers nothing ([`disagreeing`]).
pub(crate) fn proof(input: &PassInput) -> Vec<PassDiagnostic> {
    let by_id: HashMap<&str, &PassEntity> =
        input.entities.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut found: Vec<PassDiagnostic> = failing_proofs(input, &by_id);
    found.extend(dangling_names(input));
    for fact in input
        .entities
        .iter()
        .filter(|e| !e.pending && KINDS.contains(&e.kind.as_str()))
    {
        found.extend(disagreeing(fact));
    }
    found
}

/// Each (citing entity, failed test) pair, in edge order.
fn failing_proofs(input: &PassInput, by_id: &HashMap<&str, &PassEntity>) -> Vec<PassDiagnostic> {
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    for edge in &input.edges {
        let (Some(source), Some(test)) = (
            by_id.get(edge.source.as_str()),
            by_id.get(edge.target.as_str()),
        ) else {
            continue;
        };
        let cites = !KINDS.contains(&source.kind.as_str()) && !source.pending;
        let failed = test.kind == TEST_KIND
            && test.fields.get(RESULT).map(String::as_str) == Some(TestStatus::Failed.as_str());
        if cites && failed && seen.insert((&source.id, &test.id)) {
            found.push(
                PassDiagnostic::warning(
                    "W401",
                    format!(
                        "`{}` links the test `{}`, whose last collected run failed",
                        source.id, test.id
                    ),
                )
                .with_entity(&source.id),
            );
        }
    }
    found
}

/// Each dangling `names` item of a `test` or an `item`, at the place its collector read it.
fn dangling_names(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = Vec::new();
    for holder in input
        .entities
        .iter()
        .filter(|e| [TEST_KIND, ITEM_KIND].contains(&e.kind.as_str()))
    {
        for name in holder.dangling.iter().filter(|d| d.field == NAMES) {
            let why = match name.candidates.is_empty() || name.reason != DanglingReason::Ambiguous {
                true => "which is no entity of the project".to_string(),
                false => format!(
                    "the short name of several entities ({})",
                    name.candidates.join(", ")
                ),
            };
            let mut diagnostic = PassDiagnostic::new(
                "I402",
                PassSeverity::Info,
                format!(
                    "the {} `{}` names `{}`, {why}",
                    holder.kind, holder.id, name.id
                ),
            )
            .with_entity(&holder.id);
            if let Some(near) = &name.did_you_mean {
                diagnostic = diagnostic.with_suggestion(format!("did you mean `{near}`?"));
            }
            if let Some(span) = &name.span {
                diagnostic = diagnostic.with_span(span.clone());
            }
            found.push(diagnostic);
        }
    }
    found
}

/// The E101 of each reference the fact `entity` holds that answers nothing: a dangling one (no entity, several, or
/// one of another kind), and an unheld one in a field whose target one collector answers with it (a run's test). An
/// unheld reference to another collector's kind (an item's test that no runner collected) is no finding, and a
/// dangling name is I402's (the `proof` pass). The pack's collector wrote both the fact and the reference, so its
/// facts disagree: no spec author can fix it.
fn disagreeing(entity: &PassEntity) -> Vec<PassDiagnostic> {
    let unheld = entity
        .unheld
        .iter()
        .filter(|item| answered_together(&entity.kind, &item.field))
        .map(|item| (&item.id, None, &item.span));
    let dangling = entity
        .dangling
        .iter()
        .filter(|item| item.field != NAMES)
        .map(|item| {
            let mistyped = (item.reason == DanglingReason::Mistyped)
                .then_some(item.actual.as_deref())
                .flatten()
                .map(|actual| (item.field.as_str(), actual));
            (&item.id, mistyped, &item.span)
        });
    unheld
        .chain(dangling)
        .map(|(id, mistyped, span)| {
            let fact = &entity.id;
            let message = match mistyped {
                Some((field, actual)) => match takes(&entity.kind, field) {
                    Some(target) => format!(
                        "the fact '{fact}' names '{id}' in '{field}', a {actual}, not a {target}"
                    ),
                    None => format!(
                        "the fact '{fact}' names '{id}' in '{field}', a {actual}, which '{field}' does not take"
                    ),
                },
                None => format!(
                    "the fact '{fact}' names '{id}', which no collected fact holds (its collector's facts \
                     disagree)"
                ),
            };
            let diagnostic = PassDiagnostic::new("E101", PassSeverity::Error, message)
                .with_suggestion(
                    "run `specforge collect` again; if it stays, the runner pack mints it wrong",
                )
                .with_entity(fact);
            at(diagnostic, span)
        })
        .collect()
}

/// The kind the reference field `field` of `kind` takes, when `@code/core` declares it.
fn takes(kind: &str, field: &str) -> Option<&'static str> {
    reference(kind, field).map(|row| row.target)
}

/// Whether one collector answers the fact of `kind` and what its `field` names.
fn answered_together(kind: &str, field: &str) -> bool {
    reference(kind, field).is_some_and(|row| row.together)
}
