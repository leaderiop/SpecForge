use specforge_common::SourceSpan;
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

#[derive(Debug, Clone)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub label: String,
}

#[derive(Debug)]
pub struct Graph {
    nodes: HashMap<String, Node>,
    edges: Vec<Edge>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id.raw.clone(), node);
    }

    pub fn remove_node(&mut self, id: &str) {
        self.nodes.remove(id);
        self.edges
            .retain(|e| e.source != id && e.target != id);
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }

    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn nodes(&self) -> Vec<&Node> {
        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by_key(|n| &n.id.raw);
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

    pub fn edges_from(&self, id: &str) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.source == id).collect()
    }

    pub fn edges_to(&self, id: &str) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.target == id).collect()
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
        if !self.nodes.contains_key(root_id) {
            return None;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert(root_id.to_string());
        queue.push_back(root_id.to_string());

        while let Some(id) = queue.pop_front() {
            for edge in &self.edges {
                let neighbor = if edge.source == id {
                    &edge.target
                } else if edge.target == id {
                    &edge.source
                } else {
                    continue;
                };
                if visited.insert(neighbor.clone()) {
                    queue.push_back(neighbor.clone());
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
                sub.add_edge(edge.clone());
            }
        }
        Some(sub)
    }

    /// Extract the subgraph reachable from `root_id` within `max_depth` hops (both directions).
    /// Depth 0 returns only the root. Returns None if `root_id` is not in the graph.
    pub fn subgraph_depth(&self, root_id: &str, max_depth: usize) -> Option<Graph> {
        if !self.nodes.contains_key(root_id) {
            return None;
        }

        let mut visited: HashMap<String, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        visited.insert(root_id.to_string(), 0);
        queue.push_back((root_id.to_string(), 0usize));

        while let Some((id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for edge in &self.edges {
                let neighbor = if edge.source == id {
                    &edge.target
                } else if edge.target == id {
                    &edge.source
                } else {
                    continue;
                };
                if !visited.contains_key(neighbor) {
                    visited.insert(neighbor.clone(), depth + 1);
                    queue.push_back((neighbor.clone(), depth + 1));
                }
            }
        }

        let mut sub = Graph::new();
        for (id, _) in &visited {
            if let Some(node) = self.nodes.get(id) {
                sub.add_node(node.clone());
            }
        }
        for edge in &self.edges {
            if visited.contains_key(&edge.source) && visited.contains_key(&edge.target) {
                sub.add_edge(edge.clone());
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
}
