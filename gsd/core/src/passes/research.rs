//! The research pass: `gsd/W204`, an open question of a phase's research that no decision resolves. The plan checker's
//! research-resolution dimension reads the phase's `RESEARCH.md` only, so the project's research is exempt.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("research", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let phase_researches: BTreeSet<&str> = input
        .entities
        .iter()
        .filter(|e| e.kind == "research" && e.fields.contains_key("phase"))
        .map(|e| e.id.as_str())
        .collect();
    input
        .entities
        .iter()
        .filter(|e| e.kind == "open_question" && !e.fields.contains_key("resolved_by"))
        .filter(|q| {
            // An item's id is its research's id and its name.
            q.id.rsplit_once('/')
                .is_some_and(|(research, _)| phase_researches.contains(research))
        })
        .map(|q| {
            PassDiagnostic::new(
                "W204",
                PassSeverity::Warning,
                format!("open question '{}' is resolved by no decision", q.id),
            )
            .with_suggestion(
                "record the answer as a decision of the phase and name it in resolved_by (GSD: mark it RESOLVED)",
            )
            .with_entity(&q.id)
        })
        .collect()
}

#[cfg(test)]
mod tests;
