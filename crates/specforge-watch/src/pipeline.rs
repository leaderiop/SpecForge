use crate::delta::{DeltaConfig, GraphDelta, compute_graph_delta_with_config};
use crate::import_dag::ImportDag;
use specforge_common::{Diagnostic, Severity, SourceSpan, Sym};
use specforge_graph::{Graph, GraphConfig, build_graph_with_config};
use specforge_parser::{SpecFile, parse_incremental};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Result of an incremental rebuild cycle.
#[derive(Debug)]
pub struct IncrementalResult {
    pub delta: GraphDelta,
    pub diagnostics: Vec<Diagnostic>,
    pub rebuilt_files: Vec<String>,
    /// Files whose diagnostic set changed in this cycle (sorted). Consumers
    /// that publish per-file diagnostics (watch JSON output, the LSP) use
    /// this to only touch files that actually changed.
    pub changed_diagnostic_files: Vec<String>,
    /// When verify_incremental is enabled, contains the result of comparing
    /// the incremental rebuild against a cold rebuild. None if verification
    /// was not performed, Some(Ok(())) if it passed, Some(Err(msg)) if it failed.
    pub verification: Option<Result<(), String>>,
}

/// Synthesize the [`tree_sitter::InputEdit`] for a whole-file replacement and
/// return an edited copy of `old_tree`, so `parse_incremental` can reuse
/// unchanged subtrees. Computes the longest common byte prefix/suffix of the
/// two texts — the standard strategy when only before/after texts are
/// available (watch reloads, editor buffer swaps).
fn edited_tree_for_replacement(
    old_tree: &tree_sitter::Tree,
    old_src: &str,
    new_src: &str,
) -> tree_sitter::Tree {
    let mut prefix = 0;
    let max_prefix = old_src.len().min(new_src.len());
    while prefix < max_prefix && old_src.as_bytes()[prefix] == new_src.as_bytes()[prefix] {
        prefix += 1;
    }
    while prefix > 0 && !old_src.is_char_boundary(prefix) {
        prefix -= 1;
    }

    let mut suffix = 0;
    let max_suffix = max_prefix - prefix;
    while suffix < max_suffix
        && old_src.as_bytes()[old_src.len() - 1 - suffix]
            == new_src.as_bytes()[new_src.len() - 1 - suffix]
    {
        suffix += 1;
    }
    while suffix > 0 && !new_src.is_char_boundary(new_src.len() - suffix) {
        suffix -= 1;
    }

    let start_byte = prefix;
    let old_end_byte = old_src.len() - suffix;
    let new_end_byte = new_src.len() - suffix;

    let point = |src: &str, byte: usize| {
        let before = &src[..byte];
        let row = before.matches('\n').count();
        let column = byte - before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        tree_sitter::Point { row, column }
    };

    let mut tree = old_tree.clone();
    tree.edit(&tree_sitter::InputEdit {
        start_byte,
        old_end_byte,
        new_end_byte,
        start_position: point(old_src, start_byte),
        old_end_position: point(old_src, old_end_byte),
        new_end_position: point(new_src, new_end_byte),
    });
    tree
}

/// Manages the incremental compilation state.
///
/// This is the single incremental core shared by `specforge watch` and the
/// LSP: per-file parse cache with retained tree-sitter trees, the import
/// DAG, the graph rebuilt through `build_graph_with_config` (so watch/LSP
/// diagnostics match the CLI byte for byte), and per-file diagnostics.
pub struct IncrementalPipeline {
    graph: Graph,
    /// Cached source texts: path -> content, kept so whole-file replacements
    /// can be diffed against the previous text to synthesize tree edits.
    sources: HashMap<String, String>,
    import_dag: ImportDag,
    /// Cached parsed files: path -> SpecFile
    parsed_files: HashMap<String, SpecFile>,
    /// Retained tree-sitter trees: path -> tree, fed back into
    /// `parse_incremental` so keystroke-sized edits reuse unchanged subtrees.
    trees: HashMap<String, tree_sitter::Tree>,
    /// Diagnostics keyed by source file
    file_diagnostics: HashMap<String, Vec<Diagnostic>>,
    delta_config: DeltaConfig,
    /// Graph build configuration (installed extension keywords, bidirectional
    /// edge pairs, ...). Rebuilds run through `build_graph_with_config` with
    /// this config so incremental results match the CLI cold build.
    graph_config: GraphConfig,
    /// When true, rebuild() performs a cold rebuild comparison to verify correctness.
    verify_incremental: bool,
}

/// Partition diagnostics by source file.
fn partition_by_file(diagnostics: &[Diagnostic]) -> HashMap<String, Vec<Diagnostic>> {
    let mut map: HashMap<String, Vec<Diagnostic>> = HashMap::new();
    for diag in diagnostics {
        let file = diag
            .span
            .as_ref()
            .map(|s| s.file.to_string())
            .unwrap_or_default();
        map.entry(file).or_default().push(diag.clone());
    }
    map
}

/// Content comparison for `--verify-incremental`: every node (ID, kind,
/// source file, title, field values) and every edge (source, target, label)
/// of the incremental graph must match the cold rebuild.
fn compare_graph_contents(incremental: &Graph, cold: &Graph) -> Option<Result<(), String>> {
    let node_sigs = |g: &Graph| -> BTreeMap<String, String> {
        g.nodes()
            .iter()
            .map(|n| {
                let sig = format!(
                    "{} in {} title={:?} fields={}",
                    n.kind.raw,
                    n.source_span.file,
                    n.title,
                    serde_json::to_string(&n.fields).unwrap_or_default()
                );
                (n.id.raw.to_string(), sig)
            })
            .collect()
    };
    let (inc_nodes, cold_nodes) = (node_sigs(incremental), node_sigs(cold));
    if let Some(id) = inc_nodes
        .keys()
        .chain(cold_nodes.keys())
        .find(|id| inc_nodes.get(*id) != cold_nodes.get(*id))
    {
        return Some(Err(format!(
            "incremental/cold mismatch on node '{}': incremental {:?}, cold {:?}",
            id,
            inc_nodes.get(id),
            cold_nodes.get(id)
        )));
    }

    let edges = |g: &Graph| -> BTreeSet<(String, String, String)> {
        g.edges()
            .iter()
            .map(|e| {
                (
                    e.source.to_string(),
                    e.label.to_string(),
                    e.target.to_string(),
                )
            })
            .collect()
    };
    let (inc_edges, cold_edges) = (edges(incremental), edges(cold));
    if let Some((s, l, t)) = inc_edges.symmetric_difference(&cold_edges).next() {
        let side = if inc_edges.contains(&(s.clone(), l.clone(), t.clone())) {
            "only in incremental"
        } else {
            "only in cold"
        };
        return Some(Err(format!(
            "incremental/cold mismatch on edge {} -{}-> {} ({})",
            s, l, t, side
        )));
    }
    Some(Ok(()))
}

/// W113 import-cycle diagnostics for the current DAG.
fn cycle_diagnostics(import_dag: &ImportDag) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for cycle in import_dag.detect_cycles() {
        let cycle_desc = cycle.join(" -> ");
        let file = cycle.first().cloned().unwrap_or_default();
        diags.push(Diagnostic {
            code: "W113".to_string(),
            message: format!("import cycle detected: {}", cycle_desc),
            severity: Severity::Warning,
            span: Some(SourceSpan {
                file: Sym::new(&file),
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 0,
            }),
            suggestion: None,
        });
    }
    diags
}

impl IncrementalPipeline {
    /// Create an empty pipeline (no files, no graph). Used before workspace
    /// indexing; LSP state starts here.
    pub fn empty() -> Self {
        Self {
            graph: Graph::new(),
            import_dag: ImportDag::new(),
            parsed_files: HashMap::new(),
            sources: HashMap::new(),
            trees: HashMap::new(),
            file_diagnostics: HashMap::new(),
            delta_config: DeltaConfig::default(),
            graph_config: GraphConfig::default(),
            verify_incremental: false,
        }
    }

    /// Create a pipeline from a cold build.
    pub fn from_cold_build(
        spec_files: Vec<(String, SpecFile)>,
        graph: Graph,
        import_dag: ImportDag,
        diagnostics: Vec<Diagnostic>,
        graph_config: GraphConfig,
    ) -> Self {
        let sources: HashMap<String, String> = HashMap::new();
        let mut parsed_files = HashMap::new();
        for (path, spec_file) in spec_files {
            parsed_files.insert(path, spec_file);
        }

        let mut file_diagnostics = partition_by_file(&diagnostics);
        for diag in cycle_diagnostics(&import_dag) {
            let file = diag
                .span
                .as_ref()
                .map(|s| s.file.to_string())
                .unwrap_or_default();
            file_diagnostics.entry(file).or_default().push(diag);
        }

        Self {
            graph,
            import_dag,
            parsed_files,
            sources,
            trees: HashMap::new(),
            file_diagnostics,
            delta_config: DeltaConfig::default(),
            graph_config,
            verify_incremental: false,
        }
    }

    /// Enable or disable automatic cold rebuild verification after each incremental rebuild.
    /// When enabled (e.g. via --verify-incremental), rebuild() performs a full cold rebuild
    /// and compares it against the incremental result.
    pub fn set_verify_incremental(&mut self, enabled: bool) {
        self.verify_incremental = enabled;
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Mutable graph access. The pipeline owns the graph; this is an escape
    /// hatch for feature code that layers edges onto it (and for tests).
    pub fn graph_mut(&mut self) -> &mut Graph {
        &mut self.graph
    }

    pub fn import_dag(&self) -> &ImportDag {
        &self.import_dag
    }

    /// The retained tree-sitter tree for a file, if one is cached.
    pub fn tree(&self, path: &str) -> Option<&tree_sitter::Tree> {
        self.trees.get(path)
    }

    /// Diagnostics produced for a single file (parse errors, duplicates,
    /// unresolved references, cycles — everything `build_graph_with_config`
    /// reports, attributed by span).
    pub fn file_diagnostics(&self, path: &str) -> &[Diagnostic] {
        self.file_diagnostics
            .get(path)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Every file that currently has diagnostics (sorted).
    pub fn diagnostic_files(&self) -> Vec<String> {
        let mut files: Vec<String> = self.file_diagnostics.keys().cloned().collect();
        files.sort();
        files
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut all: Vec<Diagnostic> = self
            .file_diagnostics
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect();
        all.sort_by(|a, b| {
            let a_file = a.span.as_ref().map(|s| &s.file);
            let b_file = b.span.as_ref().map(|s| &s.file);
            a_file.cmp(&b_file).then_with(|| a.code.cmp(&b.code))
        });
        all
    }

    /// Process a batch of changed files. Reads file contents via the provided reader function.
    /// For deleted files, the reader should return None.
    pub fn rebuild<F>(&mut self, changed_files: &[String], read_file: F) -> IncrementalResult
    where
        F: Fn(&str) -> Option<String>,
    {
        let invalidation_set = self.import_dag.invalidation_set(changed_files);
        self.apply_invalidated(&invalidation_set, read_file)
    }

    /// Update one file from an authoritative in-memory source (an editor
    /// buffer), falling back to `read_file` (disk) for every other file the
    /// change invalidates — e.g. importers of the edited file.
    ///
    /// This is the LSP entry point: open buffers are the truth for the file
    /// being edited (disk may lag behind unsaved changes), while transitively
    /// affected files are re-read from disk.
    pub fn update_open_file<F>(
        &mut self,
        path: &str,
        content: Option<&str>,
        read_file: F,
    ) -> IncrementalResult
    where
        F: Fn(&str) -> Option<String>,
    {
        let invalidation_set = self
            .import_dag
            .invalidation_set(std::slice::from_ref(&path.to_string()));
        self.apply_invalidated(&invalidation_set, |file| {
            if file == path {
                content.map(|c| c.to_string())
            } else {
                read_file(file)
            }
        })
    }

    /// Shared rebuild body: re-parse every file in the invalidation set
    /// (reusing retained trees), apply the changes to the live graph
    /// red-green style (C4-01), and diff per-file diagnostics.
    ///
    /// Red-green: every invalidated file's live contribution is stripped
    /// ([`Graph::remove_entities_of_file`]), the re-parsed entities are
    /// re-added in deterministic (sorted-path, first-writer-wins) order,
    /// and reference edges are re-linked over the whole graph. Cross-file
    /// diagnostics (duplicates, unresolved references, cycles) are
    /// recomputed from the cached parses, so they stay authoritative
    /// without cloning every AST or rebuilding the graph from scratch.
    fn apply_invalidated<F>(
        &mut self,
        invalidation_set: &HashSet<String>,
        read_file: F,
    ) -> IncrementalResult
    where
        F: Fn(&str) -> Option<String>,
    {
        let old_graph = self.graph.clone();
        let old_file_diagnostics = self.file_diagnostics.clone();

        // Track which files we actually rebuilt
        let mut rebuilt_files: Vec<String> = Vec::new();

        // Phase 1: re-parse each invalidated file (no graph mutation yet).
        let mut reparsed: Vec<(String, Option<SpecFile>)> = Vec::new();
        for file in invalidation_set {
            match read_file(file) {
                Some(content) => {
                    // Incremental re-parse: edit the retained tree to match
                    // the new text (prefix/suffix diff), then let
                    // tree-sitter reuse unchanged subtrees.
                    let old_tree = match (
                        self.trees.get(file.as_str()),
                        self.sources.get(file.as_str()),
                    ) {
                        (Some(tree), Some(old_src)) if old_src != &content => {
                            Some(edited_tree_for_replacement(tree, old_src, &content))
                        }
                        (Some(tree), _) if self.sources.contains_key(file.as_str()) => {
                            // Content unchanged — reuse the tree as-is.
                            Some(tree.clone())
                        }
                        _ => None,
                    };
                    let (spec_file, new_tree) =
                        parse_incremental(&content, file, old_tree.as_ref());
                    if let Some(tree) = new_tree {
                        self.trees.insert(file.clone(), tree);
                    } else {
                        self.trees.remove(file);
                    }
                    self.sources.insert(file.clone(), content);

                    // Update import DAG
                    let imports: Vec<String> = spec_file
                        .imports
                        .iter()
                        .map(|i| i.path.to_string())
                        .collect();
                    self.import_dag.set_imports_resolved(file, imports);

                    self.parsed_files.insert(file.clone(), spec_file);
                    rebuilt_files.push(file.clone());
                    reparsed.push((file.clone(), self.parsed_files.get(file.as_str()).cloned()));
                }
                None => {
                    // File was deleted
                    self.trees.remove(file);
                    self.sources.remove(file);
                    self.parsed_files.remove(file);
                    self.import_dag.remove_file(file);
                    rebuilt_files.push(file.clone());
                    reparsed.push((file.clone(), None));
                }
            }
        }

        // Phase 2 (red): strip every invalidated file's live contribution.
        for (file, _) in &reparsed {
            self.graph.remove_entities_of_file(Sym::new(file));
        }

        // Phase 3 (green): re-add entities in sorted-path, first-writer-wins
        // order — the same acceptance rule the cold build applies.
        reparsed.sort_by(|a, b| a.0.cmp(&b.0));
        let mut accepted: HashSet<(Sym, Sym)> = HashSet::new();
        for (_, spec_file) in &reparsed {
            let Some(spec_file) = spec_file else { continue };
            for entity in &spec_file.entities {
                if !accepted.insert((entity.kind.raw, entity.id.raw)) {
                    continue;
                }
                self.graph
                    .add_node(specforge_graph::node_from_entity(entity));
            }
        }

        // Phase 4: recompute the authoritative diagnostics over the cached
        // parses (sorted for determinism) and the live graph — no AST clone,
        // no rebuild.
        let mut sorted_paths: Vec<&String> = self.parsed_files.keys().collect();
        sorted_paths.sort();
        let cached: Vec<&SpecFile> = sorted_paths
            .iter()
            .filter_map(|p| self.parsed_files.get(p.as_str()))
            .collect();
        let mut diagnostics =
            specforge_graph::entity_pass(cached.iter().copied(), &self.graph_config, None);
        diagnostics.extend(specforge_graph::link_and_diagnose(
            &mut self.graph,
            &self.graph_config,
        ));

        // build_graph's diagnostics are authoritative: replace the entire
        // diagnostics map with fresh results.
        let mut new_file_diagnostics = partition_by_file(&diagnostics);
        for diag in cycle_diagnostics(&self.import_dag) {
            let file = diag
                .span
                .as_ref()
                .map(|s| s.file.to_string())
                .unwrap_or_default();
            new_file_diagnostics.entry(file).or_default().push(diag);
        }

        // Files whose diagnostic set changed in this cycle
        let mut changed_diagnostic_files: Vec<String> = new_file_diagnostics
            .iter()
            .filter(|(file, diags)| {
                old_file_diagnostics
                    .get(*file)
                    .is_none_or(|old| old.as_slice() != diags.as_slice())
            })
            .map(|(file, _)| file.clone())
            .chain(
                old_file_diagnostics
                    .keys()
                    .filter(|file| !new_file_diagnostics.contains_key(*file))
                    .cloned(),
            )
            .collect();
        changed_diagnostic_files.sort();
        changed_diagnostic_files.dedup();

        let delta = compute_graph_delta_with_config(&old_graph, &self.graph, &self.delta_config);

        self.file_diagnostics = new_file_diagnostics;

        rebuilt_files.sort();

        // Verify incremental correctness by comparing against a cold rebuild
        // (same sorted order the red-green path uses).
        let verification = if self.verify_incremental {
            let cold_specs: Vec<SpecFile> = sorted_paths
                .iter()
                .filter_map(|p| self.parsed_files.get(p.as_str()))
                .cloned()
                .collect();
            let (cold_graph, _) = build_graph_with_config(&cold_specs, &self.graph_config);
            let cold_delta =
                compute_graph_delta_with_config(&old_graph, &cold_graph, &self.delta_config);
            if delta.added_nodes.len() != cold_delta.added_nodes.len()
                || delta.removed_nodes.len() != cold_delta.removed_nodes.len()
                || self.graph.node_count() != cold_graph.node_count()
                || self.graph.edge_count() != cold_graph.edge_count()
            {
                Some(Err(format!(
                    "incremental/cold mismatch: inc delta nodes +{}/-{}, inc nodes={}, inc edges={}, cold nodes={}, cold edges={}",
                    delta.added_nodes.len(),
                    delta.removed_nodes.len(),
                    self.graph.node_count(),
                    self.graph.edge_count(),
                    cold_graph.node_count(),
                    cold_graph.edge_count(),
                )))
            } else {
                // Same counts: the node set (with kind, file, title and field
                // values) and the edge set must also be identical.
                compare_graph_contents(&self.graph, &cold_graph)
            }
        } else {
            None
        };

        IncrementalResult {
            delta,
            diagnostics: self.diagnostics(),
            rebuilt_files,
            changed_diagnostic_files,
            verification,
        }
    }

    /// Clone the current graph (useful for testing delta correctness).
    pub fn clone_graph(&self) -> Graph {
        self.graph.clone()
    }
}

impl std::fmt::Debug for IncrementalPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncrementalPipeline")
            .field("node_count", &self.graph.node_count())
            .field("edge_count", &self.graph.edge_count())
            .field("file_count", &self.parsed_files.len())
            .field("tree_count", &self.trees.len())
            .finish()
    }
}
