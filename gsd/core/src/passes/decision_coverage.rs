//! `gsd/W506`: every settled decision of a verified phase maps to something shipped. GSD's verify gate
//! (`gate-decision-coverage-verify`) reads the shipped files and the plans' text; here a decision of the phase's
//! context (`origin context`, as E310 counts them) is shipped when a complete plan of the phase lists it in its
//! `decisions`, the typed form of "this plan delivered it". `workflow.context_coverage_gate` off turns it off (the
//! setting gates the code). Reading the shipped files is not done (U39).
//!
//! Also ports gsd-core bin/lib modules: decision-coverage-support.cjs, decisions.cjs.
//!
//! gsd-core modules this file ports: coverage.cjs.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::plans::{children, link, links, of_kind, text};

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("decision_coverage", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = Vec::new();
    for verification in
        of_kind(input, "verification").filter(|v| text(v, "status") == Some("passed"))
    {
        let Some(phase) = link(input, verification, "phase") else {
            continue;
        };
        let shipped: BTreeSet<&str> = children(input, phase, "plan")
            .into_iter()
            .filter(|plan| text(plan, "status") == Some("complete"))
            .flat_map(|plan| links(input, plan, "decisions"))
            .collect();
        for decision in children(input, phase, "decision")
            .into_iter()
            .filter(|d| text(d, "origin") == Some("context"))
            .filter(|d| !shipped.contains(d.id.as_str()))
        {
            found.push(
                PassDiagnostic::new(
                    "W506",
                    PassSeverity::Warning,
                    format!(
                        "decision '{}' of verified phase '{phase}' is listed by no complete plan of the phase",
                        decision.id
                    ),
                )
                .with_suggestion("list it in the decisions of the plan that shipped it")
                .with_entity(&decision.id),
            );
        }
    }
    found
}

#[cfg(test)]
mod tests;
