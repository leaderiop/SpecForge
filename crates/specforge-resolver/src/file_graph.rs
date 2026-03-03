use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::tarjan_scc;
use petgraph::Direction;
use std::collections::{HashMap, HashSet, VecDeque};

/// A directed graph of files connected by `use` imports.
///
/// Used for:
/// - E003: circular import detection via Tarjan's SCC
/// - Topological ordering for file processing
#[derive(Debug)]
pub struct FileGraph {
    graph: DiGraph<String, ()>,
    file_to_node: HashMap<String, NodeIndex>,
}

impl FileGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            file_to_node: HashMap::new(),
        }
    }

    /// Add a file node. Returns the node index.
    pub fn add_file(&mut self, path: &str) -> NodeIndex {
        if let Some(&idx) = self.file_to_node.get(path) {
            idx
        } else {
            let idx = self.graph.add_node(path.to_string());
            self.file_to_node.insert(path.to_string(), idx);
            idx
        }
    }

    /// Add an import edge from `from` to `to`.
    pub fn add_import(&mut self, from: &str, to: &str) {
        let from_idx = self.add_file(from);
        let to_idx = self.add_file(to);
        self.graph.add_edge(from_idx, to_idx, ());
    }

    /// Detect circular imports using Tarjan's SCC algorithm.
    /// Returns a list of cycles, where each cycle is a list of file paths.
    pub fn find_cycles(&self) -> Vec<Vec<String>> {
        let sccs = tarjan_scc(&self.graph);
        let mut cycles = Vec::new();
        for scc in sccs {
            match scc.len() {
                0 => {}
                1 => {
                    // Self-loop check
                    let node = scc[0];
                    if self.graph.neighbors(node).any(|n| n == node) {
                        cycles.push(vec![self.graph[node].clone()]);
                    }
                }
                _ => {
                    // Multi-node SCC = cycle
                    let cycle: Vec<String> = scc
                        .iter()
                        .map(|idx| self.graph[*idx].clone())
                        .collect();
                    cycles.push(cycle);
                }
            }
        }
        cycles
    }

    /// Return files in topological order (dependencies first).
    /// Returns `None` if there are cycles.
    pub fn topological_order(&self) -> Option<Vec<String>> {
        petgraph::algo::toposort(&self.graph, None)
            .ok()
            .map(|nodes| {
                nodes
                    .into_iter()
                    .rev() // toposort returns reverse order
                    .map(|idx| self.graph[idx].clone())
                    .collect()
            })
    }

    /// Compute the invalidation set for a changed file.
    /// Returns the set of files that transitively import `file`, plus `file` itself.
    pub fn invalidation_set(&self, file: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        let Some(&start) = self.file_to_node.get(file) else {
            return result;
        };
        result.insert(file.to_string());
        let mut queue = VecDeque::new();
        queue.push_back(start);
        while let Some(node) = queue.pop_front() {
            for importer in self.graph.neighbors_directed(node, Direction::Incoming) {
                let path = &self.graph[importer];
                if result.insert(path.clone()) {
                    queue.push_back(importer);
                }
            }
        }
        result
    }

    pub fn file_count(&self) -> usize {
        self.graph.node_count()
    }
}

impl Default for FileGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_cycles_in_dag() {
        let mut fg = FileGraph::new();
        fg.add_import("a.spec", "b.spec");
        fg.add_import("b.spec", "c.spec");
        assert!(fg.find_cycles().is_empty());
    }

    #[test]
    fn detect_simple_cycle() {
        let mut fg = FileGraph::new();
        fg.add_import("a.spec", "b.spec");
        fg.add_import("b.spec", "a.spec");
        let cycles = fg.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);
    }

    #[test]
    fn detect_three_node_cycle() {
        let mut fg = FileGraph::new();
        fg.add_import("a.spec", "b.spec");
        fg.add_import("b.spec", "c.spec");
        fg.add_import("c.spec", "a.spec");
        let cycles = fg.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn topological_order_dag() {
        let mut fg = FileGraph::new();
        fg.add_import("behaviors.spec", "invariants.spec");
        fg.add_import("behaviors.spec", "types.spec");
        let order = fg.topological_order().unwrap();
        // invariants and types should come before behaviors
        let beh_pos = order.iter().position(|f| f == "behaviors.spec").unwrap();
        let inv_pos = order.iter().position(|f| f == "invariants.spec").unwrap();
        let typ_pos = order.iter().position(|f| f == "types.spec").unwrap();
        assert!(inv_pos < beh_pos);
        assert!(typ_pos < beh_pos);
    }

    #[test]
    fn topological_order_with_cycle() {
        let mut fg = FileGraph::new();
        fg.add_import("a.spec", "b.spec");
        fg.add_import("b.spec", "a.spec");
        assert!(fg.topological_order().is_none());
    }

    #[test]
    fn invalidation_set_includes_self() {
        let mut fg = FileGraph::new();
        fg.add_file("a.spec");
        let set = fg.invalidation_set("a.spec");
        assert!(set.contains("a.spec"));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn invalidation_set_includes_direct_importer() {
        let mut fg = FileGraph::new();
        fg.add_import("a.spec", "b.spec");
        let set = fg.invalidation_set("b.spec");
        assert!(set.contains("b.spec"));
        assert!(set.contains("a.spec"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn invalidation_set_includes_transitive() {
        let mut fg = FileGraph::new();
        fg.add_import("a.spec", "b.spec");
        fg.add_import("b.spec", "c.spec");
        let set = fg.invalidation_set("c.spec");
        assert!(set.contains("c.spec"));
        assert!(set.contains("b.spec"));
        assert!(set.contains("a.spec"));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn invalidation_set_excludes_unrelated() {
        let mut fg = FileGraph::new();
        fg.add_import("a.spec", "b.spec");
        fg.add_import("b.spec", "c.spec");
        fg.add_file("d.spec");
        let set = fg.invalidation_set("c.spec");
        assert!(!set.contains("d.spec"));
        assert_eq!(set.len(), 3);
    }
}
