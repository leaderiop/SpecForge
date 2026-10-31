//! Halt propagation over a phase's `depends_on` graph: a port of `computeHaltPropagation`
//! (`src/plan-dependency-graph.cts:222-292`, gsd-core v1.19.0, MIT, `LICENSE-GSD`).
//!
//! A plan whose summary says `halted` blocks every plan that depends on it, at any depth. A plan on a dependency
//! cycle, or downstream of one, never reaches a valid order, so it fails closed: it is blocked too (upstream gives it
//! the plans it depends on as its blockers, or itself).

use std::collections::{BTreeMap, BTreeSet};

/// One plan of a phase: its in-phase dependencies, and whether its own summary says `halted`.
pub struct Node<'a> {
    pub id: &'a str,
    pub depends_on: Vec<&'a str>,
    pub halted: bool,
}

/// The ids of the plans that a halted plan blocks, directly or through other plans, and of those a cycle stops.
pub fn blocked<'a>(nodes: &[Node<'a>]) -> BTreeSet<&'a str> {
    let by_id: BTreeMap<&str, &Node> = nodes.iter().map(|n| (n.id, n)).collect();
    // Kahn's algorithm: dependencies before dependents.
    let mut in_degree: BTreeMap<&str, usize> = BTreeMap::new();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in nodes {
        in_degree.entry(node.id).or_insert(0);
        for dep in node.depends_on.iter().filter(|d| by_id.contains_key(**d)) {
            dependents.entry(dep).or_default().push(node.id);
            *in_degree.entry(node.id).or_insert(0) += 1;
        }
    }
    let mut order: Vec<&str> = nodes
        .iter()
        .filter(|n| in_degree[n.id] == 0)
        .map(|n| n.id)
        .collect();
    let mut head = 0;
    while head < order.len() {
        let current = order[head];
        head += 1;
        for &dependent in dependents.get(current).into_iter().flatten() {
            if let Some(degree) = in_degree.get_mut(dependent) {
                *degree -= 1;
                if *degree == 0 {
                    order.push(dependent);
                }
            }
        }
    }
    // One forward pass: a plan is blocked when a dependency is halted or itself blocked.
    let mut blocked: BTreeSet<&str> = BTreeSet::new();
    for &id in &order {
        let node = by_id[id];
        let stopped = node
            .depends_on
            .iter()
            .filter_map(|d| by_id.get(d))
            .any(|dep| dep.halted || blocked.contains(dep.id));
        if stopped {
            blocked.insert(id);
        }
    }
    // Fail closed: an unordered plan (a cycle, or downstream of one) cannot be shown safe to run, unless it is the
    // halted plan itself.
    let ordered: BTreeSet<&str> = order.iter().copied().collect();
    blocked.extend(
        nodes
            .iter()
            .filter(|n| !ordered.contains(n.id) && !n.halted)
            .map(|n| n.id),
    );
    blocked
}

#[cfg(test)]
mod tests;
