//! E206: a phase has at most one context, phase spec, discussion log and research, and the project at most one
//! research. GSD keeps one such file in a phase directory; the kernel's scopes would allow two entities of a kind in
//! a phase, and the drift pass needs "the phase's context".

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

/// The kinds a phase (or, for `research`, the project) holds one of.
const ONE_PER_SCOPE: &[&str] = &["context", "phase_spec", "discussion", "research"];

/// The scope an entity is in: its id without the last `/`-segment, empty for the project.
fn scope_of(id: &str) -> &str {
    id.rsplit_once('/').map_or("", |(scope, _)| scope)
}

pub fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut by_scope: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
    for entity in &input.entities {
        if let Some(kind) = ONE_PER_SCOPE.iter().find(|k| **k == entity.kind) {
            by_scope
                .entry((kind, scope_of(&entity.id)))
                .or_default()
                .push(&entity.id);
        }
    }
    let mut found = Vec::new();
    for ((kind, scope), mut ids) in by_scope {
        ids.sort_unstable();
        let Some((first, rest)) = ids.split_first() else {
            continue;
        };
        let place = if scope.is_empty() {
            "the project".to_string()
        } else {
            format!("phase '{scope}'")
        };
        let kind_name = kind.replace('_', " ");
        for id in rest {
            found.push(
                PassDiagnostic::new(
                    "E206",
                    PassSeverity::Error,
                    format!("{kind_name} '{id}' is a second {kind_name} of {place}; its {kind_name} is '{first}'"),
                )
                .with_suggestion(format!(
                    "keep one {kind_name} for {place}: merge '{id}' into '{first}' or remove it"
                ))
                .with_entity(*id),
            );
        }
    }
    found
}
