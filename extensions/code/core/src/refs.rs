//! The check pass `refs` (ADR 0329 D5): the references a spec writes in a test fact's scheme that no fact holds, and
//! the references its own facts hold that nothing answers (ADR 0346 D4).

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::UnheldReference;

use crate::{RUN_KIND, RUN_SCHEME, TEST_KIND, TEST_SCHEME};

/// The schemes whose references `@code/core` judges: its kinds'.
const SCHEMES: [&str; 2] = [TEST_SCHEME, RUN_SCHEME];

/// Each unheld reference in [`SCHEMES`], once, at the span it is written at: nothing while its entity is pending
/// (a test not written yet is TDD), E101 when a served fact store holds facts of its scheme (with the closest held
/// id of the same file), W102 when none does (nothing to check it against). A held reference is an edge, never a
/// finding, and neither is a fact nothing cites. A reference one of its own facts holds that no fact or entity
/// answers is E101 addressed to the collector that wrote it: see [`disagreeing`].
pub(crate) fn refs(input: &PassInput) -> Vec<PassDiagnostic> {
    let served: BTreeSet<&str> = input
        .fact_stores
        .iter()
        .flat_map(|store| store.schemes.iter().map(String::as_str))
        .collect();
    let mut found = Vec::new();
    for entity in input.entities.iter().filter(|e| !e.pending) {
        if [TEST_KIND, RUN_KIND].contains(&entity.kind.as_str()) {
            found.extend(disagreeing(entity));
            continue;
        }
        for item in &entity.unheld {
            let Some(scheme) = scheme(&item.id).filter(|s| SCHEMES.contains(s)) else {
                continue;
            };
            let diagnostic = if served.contains(scheme) {
                missing(input, item, scheme)
            } else {
                PassDiagnostic::warning(
                    "W102",
                    format!(
                        "`{}` names `{}`, but no `{scheme}` facts were collected at this commit: run `specforge \
                         collect`",
                        item.field, item.id
                    ),
                )
            };
            found.push(at(diagnostic.with_entity(&entity.id), &item.span));
        }
    }
    found
}

/// The E101 of each reference the fact `entity` holds that answers nothing: an unheld one (no store holds its
/// scheme's id) or a dangling one (no entity, several, or one of another kind). The runner pack's collector wrote
/// both the fact and the reference, so its facts disagree: no spec author can fix it.
fn disagreeing(entity: &PassEntity) -> Vec<PassDiagnostic> {
    let unheld = entity
        .unheld
        .iter()
        .map(|item| (&item.id, None, &item.span));
    let dangling = entity.dangling.iter().map(|item| {
        let mistyped = (item.reason == DanglingReason::Mistyped)
            .then_some(item.actual.as_deref())
            .flatten();
        (&item.id, mistyped, &item.span)
    });
    unheld
        .chain(dangling)
        .map(|(id, mistyped, span)| {
            let fact = &entity.id;
            let message = match mistyped {
                Some(actual) => format!("the fact '{fact}' names '{id}', a {actual}, not a test"),
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

/// `diagnostic` at `span`, when there is one.
fn at(diagnostic: PassDiagnostic, span: &Option<PassSpan>) -> PassDiagnostic {
    match span {
        Some(span) => diagnostic.with_span(span.clone()),
        None => diagnostic,
    }
}

/// The E101 of `item`, which names no fact of the served `scheme`.
fn missing(input: &PassInput, item: &UnheldReference, scheme: &str) -> PassDiagnostic {
    let diagnostic = PassDiagnostic::new(
        "E101",
        PassSeverity::Error,
        format!(
            "`{}` names `{}`, which no collected `{scheme}` fact holds",
            item.field, item.id
        ),
    );
    match closest(input, &item.id) {
        Some(near) => diagnostic.with_suggestion(format!("did you mean `{near}`?")),
        None => diagnostic,
    }
}

/// The held id nearest `id` by edit distance (the first by id among equally near ones) that shares its
/// `<scheme>:<package>/<file>#` prefix: a test of the same file. `None` when `id` has no `#` or the file holds none.
fn closest<'a>(input: &'a PassInput, id: &str) -> Option<&'a str> {
    let file = &id[..=id.find('#')?];
    input
        .entities
        .iter()
        .map(|e| e.id.as_str())
        .filter(|held| held.starts_with(file))
        .min_by_key(|held| (levenshtein(id, held), *held))
}

/// The edit distance between `a` and `b`, by character: the fewest insertions, deletions and substitutions.
pub(crate) fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(ca != *cb));
            diagonal = above;
        }
    }
    row[b.len()]
}

/// The scheme of a reference: the text before its first `:`.
fn scheme(id: &str) -> Option<&str> {
    id.split_once(':').map(|(scheme, _)| scheme)
}
