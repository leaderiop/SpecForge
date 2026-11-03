//! The check pass `proof` (ADR 0329 D6): a linked test that fails, and a test naming no entity.

use std::collections::{BTreeSet, HashMap};

use specforge_extension_sdk::prelude::*;

use crate::{NAMES, RESULT, RUN_KIND, TEST_KIND};

/// W401 on each entity outside `@code/core`'s kinds that is not pending and links a `test` whose result ([`RESULT`])
/// is `failed`, once per test it links; I402 on each `test` whose `names` holds an id no entity of the project has.
pub(crate) fn proof(input: &PassInput) -> Vec<PassDiagnostic> {
    let by_id: HashMap<&str, &PassEntity> =
        input.entities.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut found: Vec<PassDiagnostic> = failing_proofs(input, &by_id);
    found.extend(dangling_names(input, &by_id));
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
        let cites = ![TEST_KIND, RUN_KIND].contains(&source.kind.as_str()) && !source.pending;
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

/// Each `names` item of a `test` that is no entity's id.
fn dangling_names(input: &PassInput, by_id: &HashMap<&str, &PassEntity>) -> Vec<PassDiagnostic> {
    let mut found = Vec::new();
    for test in input.entities.iter().filter(|e| e.kind == TEST_KIND) {
        let names = test.values.get(NAMES).and_then(|v| v.as_array());
        for name in names.into_iter().flatten().filter_map(|n| n.as_str()) {
            if !by_id.contains_key(name) {
                found.push(
                    PassDiagnostic::new(
                        "I402",
                        PassSeverity::Info,
                        format!(
                            "the test `{}` names `{name}`, which is no entity of the project",
                            test.id
                        ),
                    )
                    .with_entity(&test.id),
                );
            }
        }
    }
    found
}
