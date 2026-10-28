//! The plan half of `gsd/E102`: a plan's `depends_on` names plans of its own phase and is acyclic. A token that does
//! not resolve in the phase is dropped by GSD ("edge dropped, wave placement for this plan may be unreliable",
//! `src/phase.cts:1084-1093`), and a cycle errors the whole phase (`:1040-1046`). A quick plan is in no phase: only
//! its cycles are reported.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use super::on_a_cycle;
use crate::digest::scope_of;

pub fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let plans: BTreeMap<&str, &str> = input
        .entities
        .iter()
        .filter(|e| e.kind == "plan")
        .map(|e| (e.id.as_str(), scope_of(&e.id)))
        .collect();
    let depends: Vec<(&str, &str)> = input
        .edges
        .iter()
        .filter(|e| e.label == "depends_on")
        .filter_map(|e| {
            let (source, target) = (e.source.as_str(), e.target.as_str());
            plans.contains_key(source).then_some(())?;
            plans.contains_key(target).then_some((source, target))
        })
        .collect();
    let mut found = Vec::new();
    let mut graph: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (plan, on) in &depends {
        if plans[plan] == plans[on] {
            graph.entry(plan).or_default().push(on);
        } else if !plans[plan].is_empty() {
            found.push(
                PassDiagnostic::new(
                    "E102",
                    PassSeverity::Error,
                    format!(
                        "plan '{plan}' depends on '{on}', which is not a plan of its phase: GSD drops the edge"
                    ),
                )
                .with_suggestion("order the phases instead: a phase's depends_on names the phases it follows")
                .with_entity(*plan),
            );
        }
    }
    for plan in on_a_cycle(&graph) {
        found.push(
            PassDiagnostic::new(
                "E102",
                PassSeverity::Error,
                format!("plan '{plan}' is on a dependency cycle"),
            )
            .with_suggestion("break the cycle: remove one depends_on")
            .with_entity(plan),
        );
    }
    found
}

#[cfg(test)]
mod tests;
