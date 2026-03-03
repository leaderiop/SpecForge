use specforge_common::{CompilerConfig, Diagnostic, SpecForgeJsonConfig, ValidationCode};
use specforge_graph::{FileIndex, SpecGraph};
use specforge_parser::{parse, SpecFile};
use specforge_resolver::{FileGraph, SymbolTable};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Persistent server state holding the full compilation result.
pub struct ServerState {
    pub files: Vec<SpecFile>,
    pub graph: SpecGraph,
    pub file_index: FileIndex,
    pub file_graph: FileGraph,
    pub symbols: SymbolTable,
    pub diagnostics: Vec<Diagnostic>,
    pub config: CompilerConfig,
    pub sources: HashMap<String, String>,
    pub spec_root: PathBuf,
}

impl ServerState {
    /// Full cold build: discover all .spec files, parse, resolve, build graph, validate.
    pub fn cold_build(workspace_root: &Path) -> Self {
        let (spec_root_dir, external_config) = find_project_root(workspace_root);

        let spec_files = discover_spec_files(&spec_root_dir);
        let mut sources: HashMap<String, String> = HashMap::new();
        let mut parsed_files = Vec::new();

        for file_path in &spec_files {
            if let Ok(source) = std::fs::read_to_string(file_path) {
                let path_str = file_path.to_string_lossy().to_string();
                let parsed = parse(&source, &path_str);
                sources.insert(path_str, source);
                parsed_files.push(parsed);
            }
        }

        let mut parse_diagnostics = Vec::new();
        for file in &parsed_files {
            for error in &file.errors {
                parse_diagnostics.push(Diagnostic::new(
                    ValidationCode::E010,
                    error.span.clone(),
                    error.message.clone(),
                ));
            }
        }

        let spec_root_str = spec_root_dir.to_string_lossy().to_string();
        let resolved =
            specforge_resolver::resolve_with_config(parsed_files, &spec_root_str, external_config);
        let graph_result = specforge_graph::build_graph(&resolved.files);
        let validation_bag = specforge_validator::validate(
            &resolved.files,
            &graph_result.graph,
            &resolved.config,
            &spec_root_dir,
        );

        let mut all_diagnostics = resolved.diagnostics.sorted();
        all_diagnostics.extend(validation_bag.sorted());
        all_diagnostics.extend(parse_diagnostics);
        all_diagnostics.sort();

        Self {
            files: resolved.files,
            graph: graph_result.graph,
            file_index: graph_result.file_index,
            file_graph: resolved.file_graph,
            symbols: resolved.symbols,
            diagnostics: all_diagnostics,
            config: resolved.config,
            sources,
            spec_root: spec_root_dir,
        }
    }

    /// Incremental rebuild after file changes.
    /// Re-parses invalidated files and re-runs the full resolve/graph/validate pipeline.
    pub fn incremental_rebuild(&mut self, changed_uris: &[String]) {
        // Compute invalidation set
        let mut invalidated_files = HashSet::new();
        for changed in changed_uris {
            invalidated_files.extend(self.file_graph.invalidation_set(changed));
            invalidated_files.insert(changed.clone());
        }

        // Keep non-invalidated files
        let mut updated_files: Vec<SpecFile> = self
            .files
            .iter()
            .filter(|f| !invalidated_files.contains(&f.path))
            .cloned()
            .collect();

        let mut updated_sources = self.sources.clone();

        // Discover current spec files (handles created/deleted)
        let current_spec_files = discover_spec_files(&self.spec_root);

        for file_path in &current_spec_files {
            let path_str = file_path.to_string_lossy().to_string();
            if invalidated_files.contains(&path_str) {
                // Use in-memory source if available (from did_change), else read from disk
                let source = if let Some(mem_source) = updated_sources.get(&path_str) {
                    mem_source.clone()
                } else {
                    match std::fs::read_to_string(file_path) {
                        Ok(s) => s,
                        Err(_) => {
                            updated_sources.remove(&path_str);
                            continue;
                        }
                    }
                };
                let parsed = parse(&source, &path_str);
                updated_sources.insert(path_str, source);
                updated_files.push(parsed);
            }
        }

        // Remove sources for files that no longer exist
        let current_paths: HashSet<String> = current_spec_files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        updated_sources.retain(|k, _| current_paths.contains(k));

        // Re-run pipeline
        let mut parse_diagnostics = Vec::new();
        for file in &updated_files {
            for error in &file.errors {
                parse_diagnostics.push(Diagnostic::new(
                    ValidationCode::E010,
                    error.span.clone(),
                    error.message.clone(),
                ));
            }
        }

        let spec_root_str = self.spec_root.to_string_lossy().to_string();
        let resolved = specforge_resolver::resolve(updated_files, &spec_root_str);
        let graph_result = specforge_graph::build_graph(&resolved.files);
        let validation_bag = specforge_validator::validate(
            &resolved.files,
            &graph_result.graph,
            &resolved.config,
            &self.spec_root,
        );

        let mut all_diagnostics = resolved.diagnostics.sorted();
        all_diagnostics.extend(validation_bag.sorted());
        all_diagnostics.extend(parse_diagnostics);
        all_diagnostics.sort();

        self.files = resolved.files;
        self.graph = graph_result.graph;
        self.file_index = graph_result.file_index;
        self.file_graph = resolved.file_graph;
        self.symbols = resolved.symbols;
        self.diagnostics = all_diagnostics;
        self.config = resolved.config;
        self.sources = updated_sources;
    }

    /// Update source text for a file (from did_change) without rebuilding yet.
    pub fn update_source(&mut self, path: &str, source: String) {
        self.sources.insert(path.to_string(), source);
    }
}

/// Find the project root, returning the spec files directory and an optional external config.
///
/// Walks up from `start` looking for `specforge.json` (preferred) then `specforge.spec`.
fn find_project_root(start: &Path) -> (PathBuf, Option<CompilerConfig>) {
    let start_dir = if start.is_file() {
        start.parent().unwrap_or(start)
    } else {
        start
    };

    if let Some(mut current) = start_dir.canonicalize().ok() {
        loop {
            // Prefer specforge.json
            let json_candidate = current.join("specforge.json");
            if json_candidate.exists() {
                if let Ok(content) = std::fs::read_to_string(&json_candidate) {
                    if let Ok(json_config) = serde_json::from_str::<SpecForgeJsonConfig>(&content) {
                        let spec_root = current.join(&json_config.spec_root);
                        let config = json_config.to_compiler_config();
                        return (spec_root, Some(config));
                    }
                }
            }
            // Fall back to specforge.spec
            let spec_candidate = current.join("specforge.spec");
            if spec_candidate.exists() {
                return (current, None);
            }
            if !current.pop() {
                break;
            }
        }
    }

    (start.to_path_buf(), None)
}

/// Discover all .spec files in a directory, sorted for deterministic processing.
fn discover_spec_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "spec")
        })
        .map(|entry| entry.into_path())
        .collect();
    files.sort();
    files
}
