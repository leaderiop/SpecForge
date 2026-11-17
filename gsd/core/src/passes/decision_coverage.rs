//! `gsd/W506`: every settled decision of a verified phase maps to something shipped. GSD's verify gate
//! (`gate-decision-coverage-verify`) reads the plans' text, the summaries, the shipped files and the phase's commits;
//! here a decision of the phase's context (`origin context`, as E310 counts them) is shipped when it is cited by id
//! (`passes::citations::shipped_by`, ADR 0371 D4) in the text of a complete plan of the phase, in one of its summaries
//! or a coverage item or deviation such a summary scopes, or in the subject of a recorded commit of such a plan. It
//! reads no file content and no commit body (`[decision-shipped-reach]`). `workflow.context_coverage_gate` off turns
//! it off (the setting gates the code).
//!
//! Also ports gsd-core bin/lib modules: decision-coverage-support.cjs, decisions.cjs.
//!
//! gsd-core modules this file ports: coverage.cjs.

use specforge_extension_sdk::prelude::*;

use super::citations;
use super::plans::{children, context_decisions, link, name_of, of_kind, text};

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
        let shipped = citations::shipped_by(
            input,
            children(input, phase, "plan")
                .into_iter()
                .filter(|plan| text(plan, "status") == Some("complete")),
        );
        for decision in context_decisions(input, phase)
            .into_iter()
            .filter(|d| !shipped.contains(d.id.as_str()))
        {
            found.push(
                PassDiagnostic::new(
                    "W506",
                    PassSeverity::Warning,
                    format!(
                        "decision '{}' of verified phase '{phase}' is cited by no complete plan of the phase, its summaries or its commits",
                        decision.id
                    ),
                )
                .with_suggestion(format!(
                    "cite {} by its id in the plan, the summary or the commit subject that shipped it",
                    name_of(&decision.id)
                ))
                .with_entity(&decision.id),
            );
        }
    }
    found
}

#[cfg(test)]
mod tests;
