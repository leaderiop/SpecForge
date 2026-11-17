//! Which nodes of a directed graph lie on a cycle: the answer `gsd/E102` reads for phases, plans and milestones.

use std::collections::{BTreeMap, BTreeSet};

/// The cycles of `graph` (each node -> its successors): every strongly connected component of more than one node,
/// and a node with an edge to itself, each as its members (Tarjan's algorithm, iterative so a long chain cannot
/// exhaust the guest's stack). Members of different cycles are disjoint.
pub fn cycles<'a>(graph: &BTreeMap<&'a str, Vec<&'a str>>) -> Vec<BTreeSet<&'a str>> {
    let mut nodes: BTreeSet<&str> = graph.keys().copied().collect();
    nodes.extend(graph.values().flatten().copied());
    let (mut index, mut low): (BTreeMap<&str, usize>, BTreeMap<&str, usize>) = Default::default();
    let (mut stack, mut on_stack) = (Vec::new(), BTreeSet::new());
    let mut found = Vec::new();
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
                let mut component = BTreeSet::new();
                while let Some(member) = stack.pop() {
                    on_stack.remove(member);
                    component.insert(member);
                    if member == node {
                        break;
                    }
                }
                let alone = component.len() == 1;
                if !alone || graph.get(node).is_some_and(|n| n.contains(&node)) {
                    found.push(component);
                }
            }
            if let Some(&(parent, _)) = work.last() {
                low.insert(parent, low[parent].min(low[node]));
            }
        }
    }
    found
}

/// The nodes on a cycle of `graph`, each once.
pub fn on_a_cycle<'a>(graph: &BTreeMap<&'a str, Vec<&'a str>>) -> BTreeSet<&'a str> {
    cycles(graph).into_iter().flatten().collect()
}
