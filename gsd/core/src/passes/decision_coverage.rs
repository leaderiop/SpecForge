//! `gsd/W506`: every settled decision of a verified phase maps to something shipped. GSD's verify gate
//! (`gate-decision-coverage-verify`) reads the shipped files; here a decision of the phase's context (`origin
//! context`, as E310 counts them) must be mentioned by one of the phase's summaries (their text fields) or by the
//! subject of one of its commits. It matches the way E310 does (`cross_plan::mentions`). `workflow.context_coverage_gate`
//! off turns it off (the setting gates the code). Reading the shipped files is not done (U39).
//!
//! Also ports gsd-core bin/lib modules: decision-coverage-support.cjs, decisions.cjs.
//!
//! gsd-core modules this file ports: coverage.cjs.

use specforge_extension_sdk::prelude::*;

use super::cross_plan::mentions;
use super::plans::{children, link, of_kind, text, texts};

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
        let mut shipped: Vec<String> = Vec::new();
        for plan in children(input, phase, "plan") {
            for summary in children(input, &plan.id, "summary") {
                shipped.push(texts(summary).join("\n"));
            }
            shipped.extend(
                children(input, &plan.id, "commit")
                    .into_iter()
                    .filter_map(|c| text(c, "subject").map(String::from)),
            );
        }
        for decision in children(input, phase, "decision")
            .into_iter()
            .filter(|d| text(d, "origin") == Some("context"))
            .filter(|d| !mentions(d, &shipped))
        {
            found.push(
                PassDiagnostic::new(
                    "W506",
                    PassSeverity::Warning,
                    format!(
                        "decision '{}' of verified phase '{phase}' is mentioned by no summary or commit of the phase",
                        decision.id
                    ),
                )
                .with_suggestion("name the decision (its id, or its first words) in the summary of the plan that shipped it")
                .with_entity(&decision.id),
            );
        }
    }
    found
}

#[cfg(test)]
mod tests;
