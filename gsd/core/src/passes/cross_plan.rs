//! The cross-plan pass: the rules that read the plans of a phase together. `gsd/E301` (a phase requirement in no
//! plan, plan checker D1), `gsd/E310` (a context decision no plan cites, `gate-decision-coverage-plan`: upstream's gate
//! reads the plans' text for the id or a paraphrase, SpecForge for the id alone, read with its path and resolved in
//! the plan's phase, `passes::citations`) and `gsd/I321` (the deterministic half of D3b: a justified coupling). Each
//! judges a phase only once it has a plan: GSD runs them at `plan:post`, and before that a phase has nothing to cover.
//!
//! Also ports gsd-core bin/lib modules: gate-gap-analysis-plan-post.cjs.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::citations;
use super::plans::{children, context_decisions, name_of, of_kind, rows, targets, written_target};
use crate::scope::scope_of;

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

/// E310: every trackable decision of the phase's context, a `decision` with `origin context`, is cited by its id in the
/// text of one of its plans (`citations::cited_by`): a bare id resolves in the plan's own phase, so another phase's
/// decision is cited by its path (`01/D-01`) and a paraphrase cites nothing. `settled` plays no part; discretion items
/// are not decisions. The finding is on the phase, as E301's is: it judges the phase's plans together, and a plan the
/// write creates under the phase is the step that settles it (ADR 0385).
fn uncited_decisions(
    input: &PassInput,
    phase: &PassEntity,
    plans: &[&PassEntity],
) -> Vec<PassDiagnostic> {
    let cited = citations::cited_by(input, plans.iter().copied());
    context_decisions(input, &phase.id)
        .into_iter()
        .filter(|decision| !cited.contains(decision.id.as_str()))
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
                "cite {} by its id in a task or must-have of the plan that implements it (a decision of another phase by its path, such as 01/{}), or move it to the context's discretion if it should not be tracked",
                name_of(&decision.id),
                name_of(&decision.id)
            ))
            .with_entity(&phase.id)
        })
        .collect()
}

/// I321: a `coupling_justified` item names the peer it exempts. The exemption holds for two plans of one phase and
/// one wave; GSD reports the applied exemption as its own info advisory (`agents/gsd-plan-checker.md`, D3b). A peer is
/// the plan the kernel resolved the item to from its plan; one it did not resolve (E003) is reported by the kernel.
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
                    let peer = written_target(&peers, &plan.id, named)?;
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
