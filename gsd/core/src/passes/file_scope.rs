//! The file-scope pass: `gsd/W305` and `gsd/W306`, over the paths of plans and tasks read as list items (K13), so a
//! path holding a comma stays one path.
//!
//! W305: plans of one wave share no path in `files_modified ∪ files_deleted` (the planner's rule,
//! `agents/gsd-planner.md:753-761`; execute-phase then runs the wave sequentially, `workflows/execute-phase.md:548-578`).
//! The wave is the computed `plan.wave` (a check pass sees the snapshot merged with computed values); a plan with no
//! wave (a quick plan, a phase with a cycle) is not compared. W306 is SpecForge's own: a task's files are among its
//! plan's.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::plans::{of_kind, strings};
use crate::digest::scope_of;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("file_scope", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = same_wave_overlap(input);
    found.extend(task_files(input));
    found
}

/// `files_modified ∪ files_deleted`, exact strings (`partitionByFileOverlap` compares them as written).
fn touched(plan: &PassEntity) -> BTreeSet<String> {
    let mut paths: BTreeSet<String> = strings(plan, "files_modified").into_iter().collect();
    paths.extend(strings(plan, "files_deleted"));
    paths
}

fn same_wave_overlap(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut plans: Vec<(&PassEntity, &str, BTreeSet<String>)> = of_kind(input, "plan")
        .filter_map(|p| Some((p, p.fields.get("wave")?.as_str(), touched(p))))
        .collect();
    plans.sort_by(|a, b| a.0.id.cmp(&b.0.id));
    let mut found = Vec::new();
    for (at, (later, wave, paths)) in plans.iter().enumerate() {
        for (earlier, other_wave, other_paths) in &plans[..at] {
            if wave != other_wave || scope_of(&earlier.id) != scope_of(&later.id) {
                continue;
            }
            for path in paths.intersection(other_paths) {
                found.push(
                    PassDiagnostic::new(
                        "W305",
                        PassSeverity::Warning,
                        format!(
                            "plans '{}' and '{}' of wave {wave} both modify '{path}': under gsd-compat the wave runs sequentially",
                            earlier.id, later.id
                        ),
                    )
                    .with_suggestion("give the plans different waves (depends_on), or keep the file in one of them")
                    .with_entity(&later.id),
                );
            }
        }
    }
    found
}

fn task_files(input: &PassInput) -> Vec<PassDiagnostic> {
    let plans: Vec<(&str, BTreeSet<String>)> = of_kind(input, "plan")
        .map(|p| (p.id.as_str(), touched(p)))
        .collect();
    of_kind(input, "task")
        .flat_map(|task| {
            let allowed = plans
                .iter()
                .find(|(id, _)| *id == scope_of(&task.id))
                .map(|(_, paths)| paths);
            strings(task, "files")
                .into_iter()
                .filter(|file| allowed.is_some_and(|paths| !paths.contains(file)))
                .map(|file| {
                    PassDiagnostic::new(
                        "W306",
                        PassSeverity::Warning,
                        format!(
                            "task '{}' names '{file}', which is not among the files of plan '{}'",
                            task.id,
                            scope_of(&task.id)
                        ),
                    )
                    .with_suggestion(
                        "add the file to the plan's files_modified, or take it out of the task",
                    )
                    .with_entity(&task.id)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests;
