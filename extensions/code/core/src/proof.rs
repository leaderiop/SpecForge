//! The check pass `proof` (ADR 0329 D6, ADR 0347 D11): a linked test that fails, and a test or item naming no entity.

use std::collections::{BTreeSet, HashMap};

use specforge_extension_sdk::prelude::*;

use crate::{ITEM_KIND, KINDS, NAMES, RESULT, TEST_KIND};

/// W401 on each entity outside `@code/core`'s kinds that is not pending and links a `test` whose result ([`RESULT`])
/// is `failed`, once per test it links; I402 on each name a `test` or an `item` holds in `names` that is dangling: no
/// entity of the project, or the short name of several.
pub(crate) fn proof(input: &PassInput) -> Vec<PassDiagnostic> {
    let by_id: HashMap<&str, &PassEntity> =
        input.entities.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut found: Vec<PassDiagnostic> = failing_proofs(input, &by_id);
    found.extend(dangling_names(input));
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
