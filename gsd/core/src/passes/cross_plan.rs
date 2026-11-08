//! The cross-plan pass: the rules that read the plans of a phase together. `gsd/E301` (a phase requirement in no
//! plan, plan checker D1), `gsd/E310` (a context decision no plan lists in its `decisions`,
//! `gate-decision-coverage-plan`, which matches the plans' text instead) and
//! `gsd/I321` (the deterministic half of D3b: a justified coupling). Each judges a phase only once it has a plan:
//! GSD runs them at `plan:post`, and before that a phase has nothing to cover.
//!
//! Also ports gsd-core bin/lib modules: gate-gap-analysis-plan-post.cjs.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::plans::{children, name_of, of_kind, rows};
use crate::digest::scope_of;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("cross_plan", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = Vec::new();
    for phase in of_kind(input, "phase") {
        let plans = children(input, &phase.id, "plan");
        if !plans.is_empty() {
            found.extend(uncovered_requirements(input, phase, &plans));
            found.extend(uncited_decisions(input, phase, &plans));
        }
    }
    found.extend(couplings(input));
    found
}

/// The targets of the `label` edges of `source`.
fn targets<'a>(input: &'a PassInput, source: &str, label: &str) -> BTreeSet<&'a str> {
    input
        .edges
        .iter()
        .filter(|e| e.source == source && e.label == label)
        .map(|e| e.target.as_str())
        .collect()
}

/// E301: each requirement a phase names is in the `requirements` of at least one of its plans (D1).
fn uncovered_requirements(
    input: &PassInput,
    phase: &PassEntity,
    plans: &[&PassEntity],
) -> Vec<PassDiagnostic> {
    let covered: BTreeSet<&str> = plans
        .iter()
        .flat_map(|plan| targets(input, &plan.id, "requirements"))
        .collect();
    targets(input, &phase.id, "requirements")
        .into_iter()
        .filter(|requirement| !covered.contains(requirement))
        .map(|requirement| {
            PassDiagnostic::new(
                "E301",
                PassSeverity::Error,
                format!(
                    "requirement '{requirement}' of phase '{}' is in no plan of the phase",
                    phase.id
                ),
            )
            .with_suggestion(format!(
                "add '{requirement}' to the requirements of the plan that delivers it, or write that plan"
            ))
            .with_entity(&phase.id)
        })
        .collect()
}

/// E310: every trackable decision of the phase's context, a `decision` with `origin context`, is listed in the
/// `decisions` of one of its plans: a typed reference resolved in the plan's phase, so another phase's namesake covers
/// nothing and a name in prose is no claim. `settled` plays no part; discretion items are not decisions.
fn uncited_decisions(
    input: &PassInput,
    phase: &PassEntity,
    plans: &[&PassEntity],
) -> Vec<PassDiagnostic> {
    let listed: BTreeSet<&str> = plans
        .iter()
        .flat_map(|plan| targets(input, &plan.id, "decisions"))
        .collect();
    children(input, &phase.id, "decision")
        .into_iter()
        .filter(|d| d.fields.get("origin").map(String::as_str) == Some("context"))
        .filter(|decision| !listed.contains(decision.id.as_str()))
        .map(|decision| {
            PassDiagnostic::new(
                "E310",
                PassSeverity::Error,
                format!(
                    "decision '{}' is cited by no plan of phase '{}'",
                    decision.id, phase.id
                ),
            )
            .with_suggestion(format!(
                "list {} in the decisions of the plan that implements it, or move it to the context's discretion if it should not be tracked",
                name_of(&decision.id)
            ))
            .with_entity(&decision.id)
        })
        .collect()
}

/// I321: a `coupling_justified` item names the peer it exempts. The exemption holds for two plans of one phase and
/// one wave; GSD reports the applied exemption as its own info advisory (`agents/gsd-plan-checker.md`, D3b).
fn couplings(input: &PassInput) -> Vec<PassDiagnostic> {
    let wave_of = |id: &str| {
        input
            .entities
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.fields.get("wave"))
    };
    of_kind(input, "plan")
        .flat_map(|plan| {
            let peers = targets(input, &plan.id, "coupling_justified.peer");
            rows(plan, "coupling_justified")
                .into_iter()
                .filter_map(|row| {
                    let named = row.get("peer")?;
                    let peer = peers
                        .iter()
                        .find(|p| *p == named || p.ends_with(&format!("/{named}")))?;
                    let reason = row.get("reason").map_or("", String::as_str);
                    let here = wave_of(&plan.id);
                    let same = scope_of(&plan.id) == scope_of(peer)
                        && !scope_of(&plan.id).is_empty()
                        && here.is_some()
                        && here == wave_of(peer);
                    let message = if same {
                        format!(
                            "coupling between '{}' and '{peer}' is justified: {reason}",
                            plan.id
                        )
                    } else {
                        format!(
                            "plan '{}' names '{peer}' in coupling_justified, but '{peer}' is not in its wave: the exemption covers nothing",
                            plan.id
                        )
                    };
                    Some(
                        PassDiagnostic::new("I321", PassSeverity::Info, message)
                            .with_entity(&plan.id),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests;
