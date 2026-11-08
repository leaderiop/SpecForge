//! The check pass `refs` (ADR 0329 D5, ADR 0347 D9): the references a spec writes to `@code/core`'s facts that no fact
//! holds, and the references its own facts hold that nothing answers (ADR 0346 D4).

use specforge_code_facts::{PackageId, TestId};
use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::UnheldReference;

use crate::{ITEM_KIND, KINDS, NAMES, PACKAGE_KIND, RUN_KIND, TEST_KIND};

/// Each reference field `@code/core` declares with a target kind: its kind, its name, the kind it takes, and whether
/// one collector answers the holder and its target together. What a mistyped reference one of its facts holds is told
/// it should have named; an unheld one is its collector's disagreement only when that collector answers both.
pub(crate) const TAKES: [(&str, &str, &str, bool); 4] = [
    (ITEM_KIND, "package", PACKAGE_KIND, false),
    (ITEM_KIND, "test_id", TEST_KIND, false),
    (PACKAGE_KIND, "depends_on", PACKAGE_KIND, true),
    (RUN_KIND, "test", TEST_KIND, true),
];

/// The pack that collects each kind of `@code/core`, by the scheme of a test's or an item's id and the purl type of a
/// package's: what W102 tells to run.
const PACKS: [(&str, &str, &str); 6] = [
    (TEST_KIND, "js", "@javascript/node-test"),
    (TEST_KIND, "rust", "@rust/test"),
    (ITEM_KIND, "rust", "@rust/source"),
    (ITEM_KIND, "js", "@javascript/source"),
    (PACKAGE_KIND, "cargo", "@rust/cargo"),
    (PACKAGE_KIND, "npm", "@javascript/source"),
];

/// Each unheld reference whose collected kinds (`UnheldReference.kinds`) meet `@code/core`'s, once, at the span it is
/// written at: nothing while its entity is pending (a test not written yet is TDD), E101 when a served fact store holds
/// a fact of one of those kinds in its scheme (a test reference told the closest held test of its container), W102
/// when none does (nothing to check it against), naming the kind and its pack. A held reference is an edge, never a
/// finding, and neither is a fact nothing cites. A reference one of its own facts holds that no fact or entity answers
/// is E101 addressed to the collector that wrote it: see [`disagreeing`].
pub(crate) fn refs(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = Vec::new();
    for entity in input.entities.iter().filter(|e| !e.pending) {
        if KINDS.contains(&entity.kind.as_str()) {
            found.extend(disagreeing(entity));
            continue;
        }
        for item in &entity.unheld {
            let kinds: Vec<&str> = item
                .kinds
                .iter()
                .map(String::as_str)
                .filter(|kind| KINDS.contains(kind))
                .collect();
            let Some(scheme) = scheme(&item.id).filter(|_| !kinds.is_empty()) else {
                continue;
            };
            let held = held(input, scheme, &kinds);
            let diagnostic = if held.is_empty() {
                uncollected(item, scheme, &kinds)
            } else {
                missing(input, item, scheme, &held)
            };
            found.push(at(diagnostic.with_entity(&entity.id), &item.span));
        }
    }
    found
}

/// The kinds of `kinds` a served store holds facts of in `scheme`, in `kinds`' order.
fn held<'k>(input: &PassInput, scheme: &str, kinds: &[&'k str]) -> Vec<&'k str> {
    kinds
        .iter()
        .copied()
        .filter(|kind| {
            input.fact_stores.iter().any(|store| {
                store
                    .held
                    .get(scheme)
                    .is_some_and(|held| held.iter().any(|h| h == kind))
            })
        })
        .collect()
}

/// The W102 of `item`, whose `kinds` no served store holds in `scheme`.
fn uncollected(item: &UnheldReference, scheme: &str, kinds: &[&str]) -> PassDiagnostic {
    let named: Vec<String> = kinds.iter().map(|kind| format!("`{kind}`")).collect();
    let packs: Vec<String> = kinds
        .iter()
        .filter_map(|kind| pack(scheme, &item.id, kind))
        .map(|pack| format!("`{pack}`"))
        .collect();
    let remedy = match packs.is_empty() {
        true => "run `specforge collect`".to_string(),
        false => format!("run `specforge collect` with {}", packs.join(" or ")),
    };
    PassDiagnostic::warning(
        "W102",
        format!(
            "`{}` names `{}`, but no `{scheme}` {} facts were collected: {remedy}",
            item.field,
            item.id,
            named.join(" or ")
        ),
    )
}

/// The pack that writes `kind` facts like `id`, in `scheme`: a run's is its test's, a package's its purl type's.
fn pack(scheme: &str, id: &str, kind: &str) -> Option<&'static str> {
    let (kind, family) = match kind {
        RUN_KIND => (TEST_KIND, TestId::of_run(id).ok()?.scheme().as_str()),
        PACKAGE_KIND => (kind, id.parse::<PackageId>().ok()?.purl_type()),
        _ => (kind, scheme),
    };
    PACKS
        .iter()
        .find(|(k, f, _)| *k == kind && *f == family)
        .map(|(_, _, pack)| *pack)
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

/// The kind the reference field `field` of `kind` takes, when `@code/core` declares it ([`TAKES`]).
fn takes(kind: &str, field: &str) -> Option<&'static str> {
    TAKES
        .iter()
        .find(|(of, name, _, _)| *of == kind && *name == field)
        .map(|(_, _, target, _)| *target)
}

/// Whether one collector answers the fact of `kind` and what its `field` names ([`TAKES`]).
fn answered_together(kind: &str, field: &str) -> bool {
    TAKES
        .iter()
        .any(|(of, name, _, together)| *of == kind && *name == field && *together)
}

/// `diagnostic` at `span`, when there is one.
fn at(diagnostic: PassDiagnostic, span: &Option<PassSpan>) -> PassDiagnostic {
    match span {
        Some(span) => diagnostic.with_span(span.clone()),
        None => diagnostic,
    }
}

/// The E101 of `item`, which names no fact of the `kinds` a served store holds in `scheme`.
fn missing(
    input: &PassInput,
    item: &UnheldReference,
    scheme: &str,
    kinds: &[&str],
) -> PassDiagnostic {
    let named: Vec<String> = kinds.iter().map(|kind| format!("`{kind}`")).collect();
    let diagnostic = PassDiagnostic::new(
        "E101",
        PassSeverity::Error,
        format!(
            "`{}` names `{}`, which no collected `{scheme}` {} fact holds",
            item.field,
            item.id,
            named.join(" or ")
        ),
    );
    match closest(input, &item.id, kinds) {
        Some(near) => diagnostic.with_suggestion(format!("did you mean `{near}`?")),
        None => diagnostic,
    }
}

/// The held test or run nearest `id` by edit distance (the first by id among equally near ones) in its container
/// ([`TestId::container`]): a test of the same file or binary. `None` for an item or package reference (the pass reads
/// test facts only), an id that is no test's, or a container that holds none.
fn closest<'a>(input: &'a PassInput, id: &str, kinds: &[&str]) -> Option<&'a str> {
    let (wanted, container) = match TestId::of_run(id) {
        Ok(test) => (RUN_KIND, test.run_container()),
        Err(_) => (TEST_KIND, id.parse::<TestId>().ok()?.container()),
    };
    if !kinds.contains(&wanted) {
        return None;
    }
    input
        .entities
        .iter()
        .filter(|e| e.kind == wanted && e.id.starts_with(&container))
        .map(|e| e.id.as_str())
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
