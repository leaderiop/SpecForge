//! The roadmap pass: `gsd/E101`, `gsd/E102`, `gsd/W103`, `gsd/W104`, `gsd/E108` and `gsd/E206` over the project, requirements,
//! phases and their references. Pure functions over the pass input; the plan half of E102 lands with the plan kinds
//! (r37) and E106 is a gate guard (r39).

mod criteria;
mod numbering;
mod one_per_phase;

use std::collections::{BTreeMap, BTreeSet};

use numbering::Number;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("roadmap", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = one_project(input);
    found.extend(unmapped_requirements(input));
    found.extend(dependencies(input));
    found.extend(criteria::check(input));
    found.extend(numbering::check(input));
    found.extend(one_per_phase::check(input));
    found
}

fn of_kind<'a>(input: &'a PassInput, kind: &'a str) -> impl Iterator<Item = &'a PassEntity> {
    input.entities.iter().filter(move |e| e.kind == kind)
}

fn kind_of(input: &PassInput) -> BTreeMap<&str, &str> {
    input
        .entities
        .iter()
        .map(|e| (e.id.as_str(), e.kind.as_str()))
        .collect()
}

/// The edges from an entity of kind `from` to an entity of kind `to`, as `(source, target)`.
fn edges_between<'a>(input: &'a PassInput, from: &str, to: &str) -> Vec<(&'a str, &'a str)> {
    let kinds = kind_of(input);
    input
        .edges
        .iter()
        .filter(|e| {
            kinds.get(e.source.as_str()) == Some(&from) && kinds.get(e.target.as_str()) == Some(&to)
        })
        .map(|e| (e.source.as_str(), e.target.as_str()))
        .collect()
}

/// E108: a project has one `project` entity. The kernel keeps `singleton` as metadata (it only reaches the LSP hover),
/// so the second and later ones, by id, are reported here.
fn one_project(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut ids: Vec<&str> = of_kind(input, "project").map(|p| p.id.as_str()).collect();
    ids.sort_unstable();
    let Some((first, rest)) = ids.split_first() else {
        return Vec::new();
    };
    rest.iter()
        .map(|id| {
            PassDiagnostic::new(
                "E108",
                PassSeverity::Error,
                format!(
                    "project '{id}' is a second project; this repository's project is '{first}'"
                ),
            )
            .with_suggestion(format!(
                "keep one project: merge '{id}' into '{first}' or remove it"
            ))
            .with_entity(*id)
        })
        .collect()
}

/// E101: every `v1` requirement is in exactly one phase's `requirements` (upstream `REQUIREMENT_UNMAPPED`, the
/// roadmapper's coverage check). Requirements written before the roadmap are not unmapped yet: with no phase at all
/// there is nothing to map them to, so the rule waits for the first phase.
fn unmapped_requirements(input: &PassInput) -> Vec<PassDiagnostic> {
    if of_kind(input, "phase").next().is_none() {
        return Vec::new();
    }
    let mut phases_of: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (phase, requirement) in edges_between(input, "phase", "requirement") {
        phases_of.entry(requirement).or_default().insert(phase);
    }
    of_kind(input, "requirement")
        .filter(|r| r.fields.get("tier").map(String::as_str) == Some("v1"))
        .filter_map(|r| {
            let phases: Vec<&str> = phases_of
                .get(r.id.as_str())
                .map(|p| p.iter().copied().collect())
                .unwrap_or_default();
            let (message, suggestion) = match phases.as_slice() {
                [_] => return None,
                [] => (
                    format!("requirement '{}' is in no phase", r.id),
                    format!(
                        "add '{}' to the requirements of the phase that delivers it",
                        r.id
                    ),
                ),
                many => (
                    format!(
                        "requirement '{}' is in {} phases ({})",
                        r.id,
                        many.len(),
                        many.join(", ")
                    ),
                    format!("keep '{}' in one phase and remove it from the others", r.id),
                ),
            };
            Some(
                PassDiagnostic::new("E101", PassSeverity::Error, message)
                    .with_suggestion(suggestion)
                    .with_entity(&r.id),
            )
        })
        .collect()
}

/// The milestone a phase is in: its id without the last segment.
fn milestone_of(id: &str) -> &str {
    id.rsplit_once('/').map_or("", |(milestone, _)| milestone)
}

/// E102: a phase's `depends_on` is acyclic and names earlier phases of its milestone only.
fn dependencies(input: &PassInput) -> Vec<PassDiagnostic> {
    let depends = edges_between(input, "phase", "phase");
    let mut graph: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (phase, on) in &depends {
        graph.entry(phase).or_default().push(on);
    }
    let mut found = Vec::new();
    for (phase, on) in &depends {
        let here_and_there = (Number::of(phase), Number::of(on));
        let forward = milestone_of(phase) == milestone_of(on)
            && matches!(here_and_there, (Some(here), Some(there)) if there >= here);
        if forward {
            found.push(
                PassDiagnostic::new(
                    "E102",
                    PassSeverity::Error,
                    format!("phase '{phase}' depends on '{on}', which is not an earlier phase"),
                )
                .with_suggestion(format!(
                    "a phase depends on earlier phases only: remove '{on}' from depends_on"
                ))
                .with_entity(*phase),
            );
        }
    }
    for phase in on_a_cycle(&graph) {
        found.push(
            PassDiagnostic::new(
                "E102",
                PassSeverity::Error,
                format!("phase '{phase}' is on a dependency cycle"),
            )
            .with_suggestion("break the cycle: remove one depends_on")
            .with_entity(phase),
        );
    }
    found
}

/// The nodes on a cycle: every member of a strongly connected component of more than one node, and a node that
/// depends on itself (Tarjan's algorithm, iterative so a long chain cannot exhaust the guest's stack).
fn on_a_cycle<'a>(graph: &BTreeMap<&'a str, Vec<&'a str>>) -> BTreeSet<&'a str> {
    let mut nodes: BTreeSet<&str> = graph.keys().copied().collect();
    nodes.extend(graph.values().flatten().copied());
    let (mut index, mut low): (BTreeMap<&str, usize>, BTreeMap<&str, usize>) = Default::default();
    let (mut stack, mut on_stack) = (Vec::new(), BTreeSet::new());
    let mut cyclic = BTreeSet::new();
    for root in nodes {
        if index.contains_key(root) {
            continue;
        }
        // (node, how many of its successors are done)
        let mut work = vec![(root, 0usize)];
        while let Some((node, done)) = work.pop() {
            if done == 0 {
                index.insert(node, index.len());
                low.insert(node, index[node]);
                stack.push(node);
                on_stack.insert(node);
            }
            let next = graph.get(node).and_then(|n| n.get(done)).copied();
            if let Some(next) = next {
                work.push((node, done + 1));
                if !index.contains_key(next) {
                    work.push((next, 0));
                } else if on_stack.contains(next) {
                    low.insert(node, low[node].min(index[next]));
                }
                continue;
            }
            if low[node] == index[node] {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    on_stack.remove(member);
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                let alone = component.len() == 1;
                if !alone || graph.get(node).is_some_and(|n| n.contains(&node)) {
                    cyclic.extend(component);
                }
            }
            if let Some(&(parent, _)) = work.last() {
                low.insert(parent, low[parent].min(low[node]));
            }
        }
    }
    cyclic
}

#[cfg(test)]
mod tests;
