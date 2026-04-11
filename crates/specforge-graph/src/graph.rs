use specforge_common::{SourceSpan, Sym};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct Node {
    pub id: EntityId,
    pub kind: EntityKind,
    pub title: Option<String>,
    pub fields: FieldMap,
    pub source_span: SourceSpan,
}

#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub source: Sym,
    pub target: Sym,
    pub label: Sym,
}

#[derive(Debug, Clone)]
pub struct Graph {
    nodes: HashMap<Sym, Node>,
    edges: Vec<Edge>,
    /// Index: source sym -> indices into `edges`
    source_index: HashMap<Sym, Vec<usize>>,
    /// Index: target sym -> indices into `edges`
    target_index: HashMap<Sym, Vec<usize>>,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            source_index: HashMap::new(),
            target_index: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id.raw, node);
    }

    pub fn remove_node(&mut self, id: &str) {
        let sym = Sym::new(id);
        self.nodes.remove(&sym);
        // Rebuild edges and indexes, removing any edge touching this node
        let old_edges = std::mem::take(&mut self.edges);
        self.source_index.clear();
        self.target_index.clear();
        for edge in old_edges {
            if edge.source != sym && edge.target != sym {
                let idx = self.edges.len();
                self.source_index.entry(edge.source).or_default().push(idx);
                self.target_index.entry(edge.target).or_default().push(idx);
                self.edges.push(edge);
            }
        }
    }

    pub fn add_edge(&mut self, edge: Edge) {
        let idx = self.edges.len();
        self.source_index.entry(edge.source).or_default().push(idx);
        self.target_index.entry(edge.target).or_default().push(idx);
        self.edges.push(edge);
    }

    pub fn clear_edges(&mut self) {
        self.edges.clear();
        self.source_index.clear();
        self.target_index.clear();
    }

    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.get(&Sym::new(id))
    }

    pub fn nodes(&self) -> Vec<&Node> {
        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by_key(|n| n.id.raw);
        nodes
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn nodes_in_file(&self, file: &str) -> Vec<&Node> {
        self.nodes
            .values()
            .filter(|n| n.source_span.file == file)
            .collect()
    }

    /// Filter nodes by a predicate function.
    /// Returns all nodes for which the predicate returns true.
    pub fn filter_nodes<F>(&self, predicate: F) -> Vec<&Node>
    where
        F: Fn(&Node) -> bool,
    {
        self.nodes.values().filter(|n| predicate(n)).collect()
    }

    /// Get all nodes of a specific entity kind.
    pub fn nodes_by_kind(&self, kind: &str) -> Vec<&Node> {
        self.filter_nodes(|n| n.kind.raw == kind)
    }

    pub fn edges_from(&self, id: &str) -> Vec<&Edge> {
        let sym = Sym::new(id);
        self.source_index
            .get(&sym)
            .map(|indices| indices.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }

    pub fn edges_to(&self, id: &str) -> Vec<&Edge> {
        let sym = Sym::new(id);
        self.target_index
            .get(&sym)
            .map(|indices| indices.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Extract the subgraph reachable from `root_id` following edges in both directions.
    /// Returns None if `root_id` is not in the graph.
    pub fn subgraph(&self, root_id: &str) -> Option<Graph> {
        let root_sym = Sym::new(root_id);
        if !self.nodes.contains_key(&root_sym) {
            return None;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert(root_sym);
        queue.push_back(root_sym);

        while let Some(id) = queue.pop_front() {
            for edge in &self.edges {
                let neighbor = if edge.source == id {
                    edge.target
                } else if edge.target == id {
                    edge.source
                } else {
                    continue;
                };
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }

        let mut sub = Graph::new();
        for id in &visited {
            if let Some(node) = self.nodes.get(id) {
                sub.add_node(node.clone());
            }
        }
        for edge in &self.edges {
            if visited.contains(&edge.source) && visited.contains(&edge.target) {
                sub.add_edge(*edge);
            }
        }
        Some(sub)
    }

    /// Extract the subgraph reachable from `root_id` within `max_depth` hops (both directions).
    /// Depth 0 returns only the root. Returns None if `root_id` is not in the graph.
    pub fn subgraph_depth(&self, root_id: &str, max_depth: usize) -> Option<Graph> {
        let root_sym = Sym::new(root_id);
        if !self.nodes.contains_key(&root_sym) {
            return None;
        }

        let mut visited: HashMap<Sym, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        visited.insert(root_sym, 0);
        queue.push_back((root_sym, 0usize));

        while let Some((id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for edge in &self.edges {
                let neighbor = if edge.source == id {
                    edge.target
                } else if edge.target == id {
                    edge.source
                } else {
                    continue;
                };
                if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(neighbor) {
                    e.insert(depth + 1);
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        let mut sub = Graph::new();
        for id in visited.keys() {
            if let Some(node) = self.nodes.get(id) {
                sub.add_node(node.clone());
            }
        }
        for edge in &self.edges {
            if visited.contains_key(&edge.source) && visited.contains_key(&edge.target) {
                sub.add_edge(*edge);
            }
        }
        Some(sub)
    }

    /// Compute the set of files that need rebuilding when `changed_file` changes.
    /// `import_dag` maps each file to the files it imports (dependencies).
    /// Returns the changed file plus all transitive reverse-dependents.
    pub fn invalidation_set(
        &self,
        changed_file: &str,
        import_dag: &[(String, Vec<String>)],
    ) -> HashSet<String> {
        // Build reverse dependency map: file -> files that import it
        let mut reverse_deps: HashMap<&str, Vec<&str>> = HashMap::new();
        for (file, deps) in import_dag {
            for dep in deps {
                reverse_deps
                    .entry(dep.as_str())
                    .or_default()
                    .push(file.as_str());
            }
        }

        // BFS from changed_file through reverse dependencies
        let mut affected = HashSet::new();
        let mut queue = VecDeque::new();
        affected.insert(changed_file.to_string());
        queue.push_back(changed_file);

        while let Some(file) = queue.pop_front() {
            if let Some(dependents) = reverse_deps.get(file) {
                for dep in dependents {
                    if affected.insert(dep.to_string()) {
                        queue.push_back(dep);
                    }
                }
            }
        }

        affected
    }

    /// Returns true if the directed edge set contains at least one cycle.
    pub fn has_cycles(&self) -> bool {
        !self.detect_cycles().is_empty()
    }

    /// Detect all cycles in the directed edge set using DFS.
    /// Returns a list of cycles, where each cycle is a Vec of node IDs forming the path.
    pub fn detect_cycles(&self) -> Vec<Vec<Sym>> {
        #[derive(Clone, Copy, PartialEq)]
        enum Color {
            White,
            Gray,
            Black,
        }

        let mut color: HashMap<Sym, Color> = self.nodes.keys().map(|&k| (k, Color::White)).collect();
        let mut path: Vec<Sym> = Vec::new();
        let mut cycles: Vec<Vec<Sym>> = Vec::new();

        fn dfs(
            node: Sym,
            color: &mut HashMap<Sym, Color>,
            path: &mut Vec<Sym>,
            cycles: &mut Vec<Vec<Sym>>,
            source_index: &HashMap<Sym, Vec<usize>>,
            edges: &[Edge],
        ) {
            color.insert(node, Color::Gray);
            path.push(node);

            if let Some(indices) = source_index.get(&node) {
                for &idx in indices {
                    let target = edges[idx].target;
                    match color.get(&target).copied().unwrap_or(Color::White) {
                        Color::Gray => {
                            // Found a cycle — extract the cycle path from the stack
                            if let Some(pos) = path.iter().position(|&n| n == target) {
                                let mut cycle: Vec<Sym> = path[pos..].to_vec();
                                cycle.push(target); // close the loop
                                cycles.push(cycle);
                            }
                        }
                        Color::White => {
                            dfs(target, color, path, cycles, source_index, edges);
                        }
                        Color::Black => {}
                    }
                }
            }

            path.pop();
            color.insert(node, Color::Black);
        }

        let node_ids: Vec<Sym> = self.nodes.keys().copied().collect();
        for &node in &node_ids {
            if color.get(&node).copied() == Some(Color::White) {
                dfs(node, &mut color, &mut path, &mut cycles, &self.source_index, &self.edges);
            }
        }

        cycles
    }
}
