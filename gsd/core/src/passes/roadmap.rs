//! The roadmap pass: `gsd/E101`, `gsd/E102` and `gsd/W104` over the requirements, phases and their references.
//! Pure functions over the pass input; the plan half of E102 lands with the plan kinds (r37), W103 with the list
//! items the kernel hands a pass (K13, r35) and E106 is a gate guard (r39).

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("roadmap", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = unmapped_requirements(input);
    found.extend(dependencies(input));
    found.extend(numbering(input));
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

/// E101: every `v1` requirement is in exactly one phase's `requirements` (upstream `REQUIREMENT_UNMAPPED`).
fn unmapped_requirements(input: &PassInput) -> Vec<PassDiagnostic> {
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

/// A phase's place in its milestone: `03` is `(3, 0)`, `03.1` is `(3, 1)`. `None` for a name that is no number.
fn number(id: &str) -> Option<(u32, u32)> {
    let name = id.rsplit('/').next()?;
    let (whole, part) = name.split_once('.').map_or((name, "0"), |(w, p)| (w, p));
    Some((whole.parse().ok()?, part.parse().ok()?))
}

/// The milestone a phase is in: its id without the last segment.
fn milestone_of(id: &str) -> &str {
    id.rsplit_once('/').map_or("", |(milestone, _)| milestone)
}

/// A backlog phase is a `999.x`: outside the numbering.
fn backlog((whole, _): (u32, u32)) -> bool {
    whole == 999
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
        let here_and_there = (number(phase), number(on));
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

/// The nodes that can reach themselves, by depth-first search with the nodes of the current path marked.
fn on_a_cycle<'a>(graph: &BTreeMap<&'a str, Vec<&'a str>>) -> BTreeSet<&'a str> {
    fn walk<'a>(
        node: &'a str,
        graph: &BTreeMap<&'a str, Vec<&'a str>>,
        path: &mut Vec<&'a str>,
        done: &mut BTreeSet<&'a str>,
        cyclic: &mut BTreeSet<&'a str>,
    ) {
        if let Some(at) = path.iter().position(|n| *n == node) {
            cyclic.extend(&path[at..]);
            return;
        }
        if !done.insert(node) {
            return;
        }
        path.push(node);
        for next in graph.get(node).into_iter().flatten() {
            walk(next, graph, path, done, cyclic);
        }
        path.pop();
    }
    let (mut done, mut cyclic) = (BTreeSet::new(), BTreeSet::new());
    for node in graph.keys() {
        walk(node, graph, &mut Vec::new(), &mut done, &mut cyclic);
    }
    cyclic
}

/// W104: a milestone's integer phases are continuous, and a decimal phase has the integer phase it follows.
fn numbering(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut by_milestone: BTreeMap<&str, Vec<(u32, u32, &str)>> = BTreeMap::new();
    for phase in of_kind(input, "phase") {
        if let Some(n) = number(&phase.id).filter(|n| !backlog(*n)) {
            by_milestone
                .entry(milestone_of(&phase.id))
                .or_default()
                .push((n.0, n.1, phase.id.as_str()));
        }
    }
    let mut found = Vec::new();
    for phases in by_milestone.values() {
        let wholes: BTreeSet<u32> = phases.iter().filter(|p| p.1 == 0).map(|p| p.0).collect();
        let (Some(first), Some(last)) = (wholes.first(), wholes.last()) else {
            continue;
        };
        for missing in (*first..*last).filter(|n| !wholes.contains(n)) {
            // Reported on the phase that follows the gap, which is the one the numbering skipped to.
            let next = phases
                .iter()
                .filter(|p| p.1 == 0 && p.0 > missing)
                .min_by_key(|p| p.0);
            if let Some((_, _, id)) = next {
                found.push(
                    PassDiagnostic::new(
                        "W104",
                        PassSeverity::Warning,
                        format!("phase numbers skip {missing:02}, before '{id}'"),
                    )
                    .with_suggestion("renumber with /gsd-phase, or insert the missing phase")
                    .with_entity(*id),
                );
            }
        }
        for (whole, part, id) in phases.iter().filter(|p| p.1 != 0) {
            if !wholes.contains(whole) {
                found.push(
                    PassDiagnostic::new(
                        "W104",
                        PassSeverity::Warning,
                        format!("inserted phase '{id}' follows phase {whole:02}, which does not exist"),
                    )
                    .with_suggestion(format!("an inserted phase sits after the phase it is numbered from; add phase {whole:02}"))
                    .with_entity(*id),
                );
            } else if *part > 1 && !phases.iter().any(|p| p.0 == *whole && p.1 == part - 1) {
                found.push(
                    PassDiagnostic::new(
                        "W104",
                        PassSeverity::Warning,
                        format!("inserted phase '{id}' skips {whole:02}.{}", part - 1),
                    )
                    .with_suggestion("inserted phases are numbered .1, .2, … in order")
                    .with_entity(*id),
                );
            }
        }
    }
    found
}

#[cfg(test)]
mod tests;
