use crate::delta::{compute_graph_delta_with_config, DeltaConfig, GraphDelta};
use crate::import_dag::ImportDag;
use specforge_common::{Diagnostic, Severity, SourceSpan, Sym};
use specforge_graph::{build_graph, Graph};
use specforge_parser::{parse, SpecFile};
use std::collections::HashMap;

/// Result of an incremental rebuild cycle.
#[derive(Debug)]
pub struct IncrementalResult {
    pub delta: GraphDelta,
    pub diagnostics: Vec<Diagnostic>,
    pub rebuilt_files: Vec<String>,
    /// When verify_incremental is enabled, contains the result of comparing
    /// the incremental rebuild against a cold rebuild. None if verification
    /// was not performed, Some(Ok(())) if it passed, Some(Err(msg)) if it failed.
    pub verification: Option<Result<(), String>>,
}

/// Manages the incremental compilation state.
pub struct IncrementalPipeline {
    graph: Graph,
    import_dag: ImportDag,
    /// Cached parsed files: path -> SpecFile
    parsed_files: HashMap<String, SpecFile>,
    /// Diagnostics keyed by source file
    file_diagnostics: HashMap<String, Vec<Diagnostic>>,
    delta_config: DeltaConfig,
    /// When true, rebuild() performs a cold rebuild comparison to verify correctness.
    verify_incremental: bool,
}

impl IncrementalPipeline {
    /// Create a pipeline from a cold build.
    pub fn from_cold_build(
        spec_files: Vec<(String, SpecFile)>,
        graph: Graph,
        import_dag: ImportDag,
        diagnostics: Vec<Diagnostic>,
    ) -> Self {
        let mut parsed_files = HashMap::new();
        for (path, spec_file) in spec_files {
            parsed_files.insert(path, spec_file);
        }

        // Partition diagnostics by file
        let mut file_diagnostics: HashMap<String, Vec<Diagnostic>> = HashMap::new();
        for diag in diagnostics {
            let file = diag
                .span
                .as_ref()
                .map(|s| s.file.to_string())
                .unwrap_or_default();
            file_diagnostics.entry(file).or_default().push(diag);
        }

        // Run cycle detection on initial import DAG
        let cycles = import_dag.detect_cycles();
        for cycle in &cycles {
            let cycle_desc = cycle.join(" -> ");
            let file = cycle.first().cloned().unwrap_or_default();
            let diag = Diagnostic {
                code: "W003".to_string(),
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
            };
            file_diagnostics
                .entry(file)
                .or_default()
                .push(diag);
        }

        Self {
            graph,
            import_dag,
            parsed_files,
            file_diagnostics,
            delta_config: DeltaConfig::default(),
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

    pub fn import_dag(&self) -> &ImportDag {
        &self.import_dag
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
        let old_graph = self.graph.clone();

        // Compute invalidation set
        let invalidation_set = self.import_dag.invalidation_set(changed_files);

        // Track which files we actually rebuilt
        let mut rebuilt_files: Vec<String> = Vec::new();

        // Process each invalidated file
        for file in &invalidation_set {
            match read_file(file) {
                Some(content) => {
                    // Re-parse the file
                    let spec_file = parse(&content, file);

                    // Update import DAG
                    let imports: Vec<String> = spec_file
                        .imports
                        .iter()
                        .map(|i| i.path.to_string())
                        .collect();
                    self.import_dag.set_imports_resolved(file, imports);

                    self.parsed_files.insert(file.clone(), spec_file);
                    rebuilt_files.push(file.clone());
                }
                None => {
                    // File was deleted
                    self.parsed_files.remove(file);
                    self.import_dag.remove_file(file);
                    rebuilt_files.push(file.clone());
                }
            }
        }

        // Rebuild full graph from all cached parsed files
        let all_spec_files: Vec<SpecFile> =
            self.parsed_files.values().cloned().collect();
        let (new_graph, build_diagnostics) = build_graph(&all_spec_files);

        // Compute delta
        let delta = compute_graph_delta_with_config(&old_graph, &new_graph, &self.delta_config);

        // build_graph processes ALL files, so its diagnostics are authoritative.
        // Replace the entire diagnostics map with fresh results.
        self.file_diagnostics.clear();
        for diag in &build_diagnostics {
            let file = diag
                .span
                .as_ref()
                .map(|s| s.file.to_string())
                .unwrap_or_default();
            self.file_diagnostics
                .entry(file)
                .or_default()
                .push(diag.clone());
        }

        // Re-run cycle detection across the full import DAG (W003)
        let cycles = self.import_dag.detect_cycles();
        for cycle in &cycles {
            let cycle_desc = cycle.join(" -> ");
            // Emit W003 on the first file in the cycle
            let file = cycle.first().cloned().unwrap_or_default();
            let diag = Diagnostic {
                code: "W003".to_string(),
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
            };
            self.file_diagnostics
                .entry(file)
                .or_default()
                .push(diag);
        }

        self.graph = new_graph;

        rebuilt_files.sort();

        // Verify incremental correctness by comparing against a cold rebuild
        let verification = if self.verify_incremental {
            let cold_specs: Vec<SpecFile> = self.parsed_files.values().cloned().collect();
            let (cold_graph, _) = build_graph(&cold_specs);
            let cold_delta = compute_graph_delta_with_config(&old_graph, &cold_graph, &self.delta_config);
            if delta.added_nodes.len() != cold_delta.added_nodes.len()
                || delta.removed_nodes.len() != cold_delta.removed_nodes.len()
                || self.graph.node_count() != cold_graph.node_count()
                || self.graph.edge_count() != cold_graph.edge_count()
            {
                Some(Err(format!(
                    "incremental/cold mismatch: inc nodes={}/{} edges={}/{}, cold nodes={}/{} edges={}/{}",
                    delta.added_nodes.len(), delta.removed_nodes.len(),
                    self.graph.node_count(), self.graph.edge_count(),
                    cold_delta.added_nodes.len(), cold_delta.removed_nodes.len(),
                    cold_graph.node_count(), cold_graph.edge_count(),
                )))
            } else {
                Some(Ok(()))
            }
        } else {
            None
        };

        IncrementalResult {
            delta,
            diagnostics: self.diagnostics(),
            rebuilt_files,
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
            .finish()
    }
}
