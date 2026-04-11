use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

/// Tracks file-level import dependencies.
/// Maps each file path to the set of files it imports.
#[derive(Debug, Clone, Default)]
pub struct ImportDag {
    /// file -> files it imports (forward edges)
    deps: HashMap<String, BTreeSet<String>>,
}

impl ImportDag {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the imports for a file (replaces any previous entry).
    /// Import paths are stored as-is. Use `set_imports_resolved` to resolve
    /// import paths against known file keys before storing.
    pub fn set_imports(&mut self, file: &str, imports: Vec<String>) {
        self.deps.insert(file.to_string(), imports.into_iter().collect());
    }

    /// Set imports for a file, resolving each import path against known file keys.
    /// E.g., import "a" matches file key "a.spec" if "a" is not a known key.
    pub fn set_imports_resolved(&mut self, file: &str, imports: Vec<String>) {
        let resolved: BTreeSet<String> = imports
            .into_iter()
            .map(|imp| self.resolve_import(&imp).unwrap_or(imp))
            .collect();
        self.deps.insert(file.to_string(), resolved);
    }

    /// Resolve an import path to a known file key.
    fn resolve_import(&self, import: &str) -> Option<String> {
        if self.deps.contains_key(import) {
            return Some(import.to_string());
        }
        let candidate = format!("{}.spec", import);
        if self.deps.contains_key(&candidate) {
            return Some(candidate);
        }
        None
    }

    /// Remove a file from the DAG entirely.
    pub fn remove_file(&mut self, file: &str) {
        self.deps.remove(file);
    }

    /// Get the files that `file` imports (forward deps).
    pub fn imports_of(&self, file: &str) -> Vec<&str> {
        self.deps
            .get(file)
            .map(|s| s.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Compute the invalidation set: the changed files plus all transitive
    /// reverse dependents (files that directly or indirectly import the changed files).
    pub fn invalidation_set(&self, changed_files: &[String]) -> HashSet<String> {
        // Build reverse dependency map
        let mut reverse: HashMap<&str, Vec<&str>> = HashMap::new();
        for (file, imports) in &self.deps {
            for imp in imports {
                reverse.entry(imp.as_str()).or_default().push(file.as_str());
            }
        }

        let mut affected = HashSet::new();
        let mut queue = VecDeque::new();

        for file in changed_files {
            if affected.insert(file.clone()) {
                queue.push_back(file.as_str());
            }
        }

        while let Some(file) = queue.pop_front() {
            if let Some(dependents) = reverse.get(file) {
                for &dep in dependents {
                    if affected.insert(dep.to_string()) {
                        queue.push_back(dep);
                    }
                }
            }
        }

        affected
    }

    /// Detect import cycles using Tarjan's SCC algorithm.
    /// Returns a list of cycles, where each cycle is a vec of file paths.
    /// Only SCCs with more than one node (actual cycles) are returned.
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        struct TarjanState {
            index_counter: usize,
            stack: Vec<String>,
            on_stack: HashSet<String>,
            indices: HashMap<String, usize>,
            lowlinks: HashMap<String, usize>,
            result: Vec<Vec<String>>,
        }

        fn strongconnect(
            node: &str,
            deps: &HashMap<String, BTreeSet<String>>,
            state: &mut TarjanState,
        ) {
            state.indices.insert(node.to_string(), state.index_counter);
            state.lowlinks.insert(node.to_string(), state.index_counter);
            state.index_counter += 1;
            state.stack.push(node.to_string());
            state.on_stack.insert(node.to_string());

            if let Some(successors) = deps.get(node) {
                for succ in successors {
                    if !state.indices.contains_key(succ.as_str()) {
                        strongconnect(succ, deps, state);
                        let succ_low = state.lowlinks[succ.as_str()];
                        let node_low = state.lowlinks.get_mut(node).unwrap();
                        if succ_low < *node_low {
                            *node_low = succ_low;
                        }
                    } else if state.on_stack.contains(succ.as_str()) {
                        let succ_idx = state.indices[succ.as_str()];
                        let node_low = state.lowlinks.get_mut(node).unwrap();
                        if succ_idx < *node_low {
                            *node_low = succ_idx;
                        }
                    }
                }
            }

            if state.lowlinks[node] == state.indices[node] {
                let mut scc = Vec::new();
                loop {
                    let w = state.stack.pop().unwrap();
                    state.on_stack.remove(&w);
                    scc.push(w.clone());
                    if w == node {
                        break;
                    }
                }
                if scc.len() > 1 {
                    scc.sort();
                    state.result.push(scc);
                }
            }
        }

        let mut state = TarjanState {
            index_counter: 0,
            stack: Vec::new(),
            on_stack: HashSet::new(),
            indices: HashMap::new(),
            lowlinks: HashMap::new(),
            result: Vec::new(),
        };

        let all_nodes: BTreeSet<&str> = self.deps.keys().map(|s| s.as_str())
            .chain(self.deps.values().flat_map(|v| v.iter().map(|s| s.as_str())))
            .collect();

        for node in &all_nodes {
            if !state.indices.contains_key(*node) {
                strongconnect(node, &self.deps, &mut state);
            }
        }

        state.result.sort();
        state.result
    }

    /// Convert to the slice format used by Graph::invalidation_set for compatibility.
    pub fn as_pairs(&self) -> Vec<(String, Vec<String>)> {
        self.deps
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
            .collect()
    }
}
