//! The wave-collision pass: `gsd/W526`, one warning per plan whose strict wave is above its wave, saying where it
//! runs and behind which plan (ADR 0378 D2). It reads the placement of `waves::placed`, never a second level
//! computation.
//!
//! The plan that binds a push is the one it runs after with the highest strict wave, ties to the smaller id. A direct
//! push (the binding plan shares a path with it and is not one of its `depends_on`) names the shared paths, the
//! earlier plan, the tasks of either plan that name those paths and both strict waves; a transitive push names the
//! plan it depends on. A shared path is never exempted: `coupling_justified` covers only non-file coupling.
//! `gsd/W305`, gsd-core's own check of the declared waves, co-fires on a same-wave collision and stays as it is.

use std::cmp::Reverse;
use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::plans::{of_kind, strings};
use super::waves::{Placed, placed};
use crate::scope::scope_of;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("wave_collision", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    placed(input)
        .iter()
        .flat_map(|placement| collisions(input, placement))
        .collect()
}

/// One W526 per plan of `placement` pushed past its wave, in plan id order.
fn collisions(input: &PassInput, placement: &Placed) -> Vec<PassDiagnostic> {
    let mut found = Vec::new();
    for (&plan, &strict) in &placement.strict {
        let wave = placement.wave[plan];
        if strict <= wave {
            continue;
        }
        let Some(&binding) = placement
            .before
            .get(plan)
            .into_iter()
            .flatten()
            .max_by_key(|p| (placement.strict[**p], Reverse(**p)))
        else {
            continue;
        };
        let after = placement.strict[binding];
        let depends = placement
            .depends
            .get(plan)
            .is_some_and(|d| d.contains(binding));
        let diagnostic = match placement.shared.get(&(plan, binding)) {
            Some(paths) if !depends => {
                let quoted: Vec<String> = paths.iter().map(|p| format!("'{p}'")).collect();
                let tasks = naming(input, [plan, binding], paths);
                let tasks = if tasks.is_empty() {
                    "no task names them".to_string()
                } else {
                    format!("tasks: {}", tasks.join(", "))
                };
                PassDiagnostic::new(
                    "W526",
                    PassSeverity::Warning,
                    format!(
                        "plan '{plan}' shares {} with '{binding}' ({tasks}): it runs in strict wave {strict}, after \
                         '{binding}' in strict wave {after}, not in its wave {wave}",
                        quoted.join(", ")
                    ),
                )
                .with_suggestion(
                    "give it a `depends_on` on the earlier plan so its declared wave says where it runs, or keep the \
                     path in one plan",
                )
            }
            _ => PassDiagnostic::new(
                "W526",
                PassSeverity::Warning,
                format!(
                    "plan '{plan}' depends on '{binding}', which runs in strict wave {after}: it runs in strict wave \
                     {strict}, not in its wave {wave}"
                ),
            )
            .with_suggestion("see the gsd/W526 of the plan it depends on"),
        };
        found.push(diagnostic.with_entity(plan));
    }
    found
}

/// The ids of the tasks of `plans` whose `files` name one of `paths`, sorted.
fn naming(input: &PassInput, plans: [&str; 2], paths: &BTreeSet<String>) -> Vec<String> {
    let found: BTreeSet<String> = of_kind(input, "task")
        .filter(|task| plans.contains(&scope_of(&task.id)))
        .filter(|task| strings(task, "files").iter().any(|f| paths.contains(f)))
        .map(|task| task.id.clone())
        .collect();
    found.into_iter().collect()
}

#[cfg(test)]
mod tests;
