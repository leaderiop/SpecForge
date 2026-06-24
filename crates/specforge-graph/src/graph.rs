use specforge_common::{find_close_match, Diagnostic, SourceSpan, Sym};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue};
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
    /// Known bidirectional edge pairs where A->B with label X and B->A with label Y
    /// represent complementary relationships, not real circular dependencies.
    /// Each pair is (label_forward, label_reverse).
    /// Populated by callers (e.g., from extension edge types) rather than hardcoded.
    bidirectional_pairs: Vec<(String, String)>,
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
            bidirectional_pairs: Vec::new(),
        }
    }

    /// Create a new graph with the given bidirectional edge pairs.
    /// These pairs are used to suppress false-positive cycle detection
    /// for complementary relationships (e.g., `invariants`/`enforced_by`).
    pub fn with_bidirectional_pairs(bidirectional_pairs: Vec<(String, String)>) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            source_index: HashMap::new(),
            target_index: HashMap::new(),
            bidirectional_pairs,
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

    /// Like [`add_edge`] but checks that both `source` and `target` exist as
    /// nodes in the graph. Returns `Some(Diagnostic)` (W011) and does **not**
    /// insert the edge when either endpoint is missing. Returns `None` on
    /// success (edge was added).
    pub fn add_edge_checked(&mut self, edge: Edge) -> Option<Diagnostic> {
        let source_exists = self.nodes.contains_key(&edge.source);
        let target_exists = self.nodes.contains_key(&edge.target);

        if !source_exists || !target_exists {
            let missing: Vec<&str> = [
                (!source_exists).then_some(edge.source.as_str()),
                (!target_exists).then_some(edge.target.as_str()),
            ]
            .into_iter()
            .flatten()
            .collect();

            return Some(Diagnostic::warning(
                "W011",
                format!(
                    "edge '{}' --[{}]--> '{}': node(s) not found: {}",
                    edge.source.as_str(),
                    edge.label.as_str(),
                    edge.target.as_str(),
                    missing.join(", "),
                ),
            ));
        }

        self.add_edge(edge);
        None
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

    /// Collect all neighbor node syms for a given node, using the source and target indexes.
    fn neighbors(&self, id: Sym) -> Vec<Sym> {
        let mut result = Vec::new();
        if let Some(indices) = self.source_index.get(&id) {
            for &idx in indices {
                result.push(self.edges[idx].target);
            }
        }
        if let Some(indices) = self.target_index.get(&id) {
            for &idx in indices {
                result.push(self.edges[idx].source);
            }
        }
        result
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
            for neighbor in self.neighbors(id) {
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }

        let mut sub = Graph::with_bidirectional_pairs(self.bidirectional_pairs.clone());
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
            for neighbor in self.neighbors(id) {
                if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(neighbor) {
                    e.insert(depth + 1);
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        let mut sub = Graph::with_bidirectional_pairs(self.bidirectional_pairs.clone());
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

    /// Resolve reference fields into graph edges and return E001
    /// diagnostics for any unresolved references.
    ///
    /// This is the **single source of truth** for reference resolution.
    /// Both the CLI (via `build_graph_with_config`) and the LSP call this
    /// method so that diagnostics are identical.
    ///
    /// The method clears all existing edges, then iterates every node's
    /// `ReferenceList` fields plus any `Identifier` fields that match
    /// `single_ref_fields` (a set of `(entity_kind, field_name)` pairs
    /// declaring which Identifier fields are single-reference fields).
    /// For each target that exists in the graph an edge is created; for
    /// each target that does *not* exist an E001 diagnostic is emitted
    /// (with a fuzzy-match suggestion when possible).
    pub fn resolve_references(&mut self) -> Vec<Diagnostic> {
        self.resolve_references_with_singles(&HashSet::new())
    }

    /// Like [`resolve_references`] but also resolves single-reference
    /// `Identifier` fields when `(entity_kind, field_name)` is in the set.
    pub fn resolve_references_with_singles(
        &mut self,
        single_ref_fields: &HashSet<(String, String)>,
    ) -> Vec<Diagnostic> {
        self.clear_edges();

        let entity_ids: HashSet<Sym> = self.nodes.keys().copied().collect();

        // Snapshot node data so we can mutate edges while iterating.
        let all_nodes: Vec<(Sym, Sym, FieldMap, SourceSpan)> = self
            .nodes
            .values()
            .map(|n| (n.id.raw, n.kind.raw, n.fields.clone(), n.source_span.clone()))
            .collect();

        let mut diagnostics = Vec::new();

        for (node_id, node_kind, fields, span) in &all_nodes {
            for entry in fields.entries() {
                match &entry.value {
                    FieldValue::ReferenceList(refs) => {
                        for target_id in refs {
                            let target_sym = Sym::new(target_id);
                            if entity_ids.contains(&target_sym) {
                                self.add_edge(Edge {
                                    source: *node_id,
                                    target: target_sym,
                                    label: entry.key,
                                });
                            } else {
                                let suggestion = find_close_match(
                                    target_id,
                                    entity_ids.iter().map(|s| s.as_str()),
                                );
                                let mut diag = Diagnostic::error(
                                    "E003",
                                    format!(
                                        "unresolved reference '{}' in entity '{}'",
                                        target_id, node_id
                                    ),
                                )
                                .with_span(span.clone());
                                if let Some(s) = suggestion {
                                    diag = diag
                                        .with_suggestion(format!("did you mean '{}'?", s));
                                }
                                diagnostics.push(diag);
                            }
                        }
                    }
                    FieldValue::Identifier(target_id)
                        if single_ref_fields.contains(&(
                            node_kind.as_str().to_string(),
                            entry.key.as_str().to_string(),
                        )) =>
                    {
                        let target_sym = Sym::new(target_id);
                        if entity_ids.contains(&target_sym) {
                            self.add_edge(Edge {
                                source: *node_id,
                                target: target_sym,
                                label: entry.key,
                            });
                        }
                        // Single refs don't emit E001 — the value might be
                        // a valid enum/identifier rather than a broken ref.
                    }
                    _ => {}
                }
            }
        }

        diagnostics
    }

    /// Returns true if the directed edge set contains at least one cycle.
    pub fn has_cycles(&self) -> bool {
        !self.detect_cycles().is_empty()
    }

    /// Returns true if a 2-hop cycle (A -> B -> A) consists of a known
    /// bidirectional edge pair and should NOT be reported as a real cycle.
    fn is_bidirectional_pair_cycle(&self, a: Sym, b: Sym) -> bool {
        // Collect edge labels from a -> b
        let labels_ab: Vec<Sym> = self
            .source_index
            .get(&a)
            .into_iter()
            .flatten()
            .filter(|&&idx| self.edges[idx].target == b)
            .map(|&idx| self.edges[idx].label)
            .collect();

        // Collect edge labels from b -> a
        let labels_ba: Vec<Sym> = self
            .source_index
            .get(&b)
            .into_iter()
            .flatten()
            .filter(|&&idx| self.edges[idx].target == a)
            .map(|&idx| self.edges[idx].label)
            .collect();

        // Check if any (ab_label, ba_label) combination matches a known pair
        for &lab_ab in &labels_ab {
            for &lab_ba in &labels_ba {
                let ab_str = lab_ab.as_str();
                let ba_str = lab_ba.as_str();
                for (fwd, rev) in &self.bidirectional_pairs {
                    if (ab_str == fwd && ba_str == rev)
                        || (ab_str == rev && ba_str == fwd)
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Detect all cycles in the directed edge set using DFS.
    /// Returns a list of cycles, where each cycle is a Vec of node IDs forming the path.
    ///
    /// Two-hop cycles that consist entirely of known bidirectional edge pairs
    /// (e.g., `invariants`/`enforced_by`) are excluded, since they represent
    /// complementary relationships rather than real circular dependencies.
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
                            // Found a cycle -- extract the cycle path from the stack
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

        // Filter out 2-hop cycles that are known bidirectional pairs.
        // A 2-hop cycle is represented as [A, B, A] (3 elements, first == last).
        cycles.retain(|cycle| {
            if cycle.len() == 3 && cycle[0] == cycle[2] {
                // This is a 2-hop cycle: A -> B -> A
                !self.is_bidirectional_pair_cycle(cycle[0], cycle[1])
            } else {
                true // keep all longer cycles and self-loops
            }
        });

        cycles
    }
}
