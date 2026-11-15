//! The journeys pass: gsd/W530 (ADR 0357 D24). A journey lists every milestone it ships in; each requirement it names
//! (a step's, or its own `requirements`, `passes::derive::product::Names`) that no phase of such a milestone lists in
//! `requirements` means the milestone cannot deliver the journey.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::derive::product::{Named, Names};
use super::plans::{link, links, of_kind, targets};

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("journeys", |p| {
        p.phase("check").run(check);
    });
}

/// Where a journey names a requirement: its 1-based steps, and whether its own `requirements` list it.
#[derive(Default)]
struct Need {
    steps: Vec<usize>,
    whole: bool,
}

impl Need {
    /// "step 1", "steps 1 and 3", "the journey's requirements", or the steps and the journey's requirements.
    fn by(&self) -> String {
        let steps = match self.steps.as_slice() {
            [] => None,
            [one] => Some(format!("step {one}")),
            [first @ .., last] => {
                let first: Vec<String> = first.iter().map(usize::to_string).collect();
                Some(format!("steps {} and {last}", first.join(", ")))
            }
        };
        match (steps, self.whole) {
            (Some(steps), true) => format!("{steps} and the journey's requirements"),
            (Some(steps), false) => steps,
            (None, _) => "the journey's requirements".to_string(),
        }
    }
}

/// W530: for each milestone a journey lists in `milestones`, one warning on the journey per requirement it names that
/// no phase of that milestone lists, naming the requirement, the places that need it and the milestone. An unphased
/// requirement is outside every milestone; a journey that lists no milestone is silent, and so is a step that names
/// no requirement (`journey.covered` says that) or one that does not resolve (the host's E003).
fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    // Each requirement and the milestones whose phases list it.
    let mut held: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for phase in of_kind(input, "phase") {
        let Some(milestone) = link(input, phase, "milestone") else {
            continue;
        };
        for requirement in links(input, phase, "requirements") {
            held.entry(requirement).or_default().insert(milestone);
        }
    }
    let mut journeys: Vec<&PassEntity> = of_kind(input, "journey").collect();
    journeys.sort_by(|a, b| a.id.cmp(&b.id));
    let mut found = Vec::new();
    for journey in journeys {
        let milestones = targets(input, &journey.id, "milestones");
        if milestones.is_empty() {
            continue;
        }
        let names = Names::of(input, journey);
        let mut needs: BTreeMap<&str, Need> = BTreeMap::new();
        for (at, named) in names.steps.iter().enumerate() {
            if let Named::Requirement(requirement) = named {
                needs.entry(requirement).or_default().steps.push(at + 1);
            }
        }
        for named in &names.whole {
            if let Named::Requirement(requirement) = named {
                needs.entry(requirement).or_default().whole = true;
            }
        }
        for milestone in &milestones {
            for (requirement, need) in &needs {
                if held.get(requirement).is_some_and(|m| m.contains(milestone)) {
                    continue;
                }
                found.push(
                    PassDiagnostic::new(
                        "W530",
                        PassSeverity::Warning,
                        format!(
                            "journey '{}' lists milestone '{milestone}', but no phase of it lists {requirement}, \
                             named by {}",
                            journey.id,
                            need.by()
                        ),
                    )
                    .with_suggestion(format!(
                        "map {requirement} to a phase of '{milestone}', or take '{milestone}' out of the journey's \
                         `milestones`"
                    ))
                    .with_entity(journey.id.as_str()),
                );
            }
        }
    }
    found
}

#[cfg(test)]
mod tests;
