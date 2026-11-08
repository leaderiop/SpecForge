//! The requirement thread's gap rules (ADR 0351), each off in gsd-compat and switched by its `specforge.*` setting:
//! `gsd/E519`, a truth of a phase's plan proves one of its plan's requirements.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::plans::{link, links, of_kind};

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("traceability", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    truth_requirements(input)
}

/// E519, on the truth: a truth whose plan is in a phase (not a quick plan) names a requirement, and one its plan
/// claims in `requirements`.
fn truth_requirements(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "truth")
        .filter_map(|truth| {
            let plan = input
                .entities
                .iter()
                .find(|e| Some(e.id.as_str()) == link(input, truth, "plan"))?;
            link(input, plan, "phase")?;
            let (message, suggestion) = match link(input, truth, "requirement") {
                None => (
                    format!("truth '{}' names no requirement it proves", truth.id),
                    "set its requirement to the one of its plan's requirements it proves".to_string(),
                ),
                Some(requirement) => {
                    let claimed: BTreeSet<&str> =
                        links(input, plan, "requirements").into_iter().collect();
                    if claimed.contains(requirement) {
                        return None;
                    }
                    (
                        format!(
                            "truth '{}' proves '{requirement}', which is not a requirement of plan '{}'",
                            truth.id, plan.id
                        ),
                        format!(
                            "name one of the plan's requirements, or add '{requirement}' to the plan's requirements"
                        ),
                    )
                }
            };
            Some(
                PassDiagnostic::new("E519", PassSeverity::Error, message)
                    .with_suggestion(suggestion)
                    .with_entity(&truth.id),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
