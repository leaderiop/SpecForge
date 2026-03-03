use specforge_common::{CompilerConfig, Diagnostic, SpecForgeJsonConfig, ValidationCode};
use specforge_graph::{build_graph, FileIndex, SpecGraph};
use specforge_parser::{parse, SpecFile};
use specforge_resolver::{resolve_with_config, FileGraph};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// How the project root was discovered.
pub enum ProjectRoot {
    /// `specforge.json` found — contains the JSON config path.
    Json(PathBuf),
    /// `specforge.spec` found — legacy discovery (config extracted from spec block).
    Spec(PathBuf),
}

/// The result of running the compilation pipeline (parse → resolve → build graph → validate).
#[allow(dead_code)]
pub struct PipelineResult {
    pub files: Vec<SpecFile>,
    pub graph: SpecGraph,
    pub file_index: FileIndex,
    pub file_graph: FileGraph,
    pub diagnostics: Vec<Diagnostic>,
    pub config: CompilerConfig,
    pub sources: HashMap<String, String>,
}

/// Run the full compilation pipeline on a directory path.
///
/// Returns `Ok(PipelineResult)` on success, or `Err(exit_code)` if file discovery/read fails.
pub fn run_pipeline(path: &Path) -> Result<PipelineResult, i32> {
    let project_root = find_project_root(path);

    // Determine spec file root directory and optional external config
    let (spec_root_dir, external_config) = match &project_root {
        Some(ProjectRoot::Json(json_path)) => {
            let config = match load_json_config(json_path) {
                Ok(c) => c,
                Err(msg) => {
                    eprintln!("specforge: {msg}");
                    return Err(1);
                }
            };
            let project_dir = json_path.parent().unwrap_or(Path::new("."));
            let spec_root = project_dir.join(&config.spec_root);
            let compiler_config = config.to_compiler_config();
            (spec_root, Some(compiler_config))
        }
        Some(ProjectRoot::Spec(spec_path)) => {
            let dir = spec_path
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf();
            (dir, None)
        }
        None => (path.to_path_buf(), None),
    };

    let spec_files = discover_spec_files(&spec_root_dir);

    if spec_files.is_empty() {
        eprintln!(
            "specforge: no .spec files found in {}",
            spec_root_dir.display()
        );
        return Err(1);
    }

    // Step 2: Read and parse all files
    let mut sources: HashMap<String, String> = HashMap::new();
    let mut parsed_files = Vec::new();

    for file_path in &spec_files {
        let source = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("specforge: error reading {}: {e}", file_path.display());
                return Err(1);
            }
        };
        let path_str = file_path.to_string_lossy().to_string();
        let parsed = parse(&source, &path_str);
        sources.insert(path_str, source);
        parsed_files.push(parsed);
    }

    // Step 2b: Convert parse errors to diagnostics (before resolve consumes files)
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

    // Step 3: Resolve
    let spec_root_str = spec_root_dir.to_string_lossy().to_string();
    let resolved = resolve_with_config(parsed_files, &spec_root_str, external_config);

    // Step 4: Build graph
    let graph_result = build_graph(&resolved.files);

    // Step 5: Validate
    let config = resolved.config;
    let file_graph = resolved.file_graph;
    let validation_bag =
        specforge_validator::validate(&resolved.files, &graph_result.graph, &config, spec_root_dir.as_path());

    // Step 6: Merge parse + resolver + validator diagnostics, sort
    let mut all_diagnostics = resolved.diagnostics.sorted();
    all_diagnostics.extend(validation_bag.sorted());
    all_diagnostics.extend(parse_diagnostics);
    all_diagnostics.sort();

    Ok(PipelineResult {
        files: resolved.files,
        graph: graph_result.graph,
        file_index: graph_result.file_index,
        file_graph,
        diagnostics: all_diagnostics,
        config,
        sources,
    })
}

/// Walk up from a directory to find the project root.
///
/// At each level, checks for `specforge.json` first, then `specforge.spec`.
/// Returns `Some(ProjectRoot::Json)` or `Some(ProjectRoot::Spec)` on match,
/// or `None` if neither is found.
pub fn find_project_root(start: &Path) -> Option<ProjectRoot> {
    let start = if start.is_file() {
        start.parent()?
    } else {
        start
    };

    let mut current = start.canonicalize().ok()?;
    loop {
        let json_candidate = current.join("specforge.json");
        if json_candidate.exists() {
            return Some(ProjectRoot::Json(json_candidate));
        }
        let spec_candidate = current.join("specforge.spec");
        if spec_candidate.exists() {
            return Some(ProjectRoot::Spec(spec_candidate));
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Backward-compatible wrapper: walk up to find `specforge.spec`.
pub fn find_spec_root(start: &Path) -> Option<PathBuf> {
    let start = if start.is_file() {
        start.parent()?
    } else {
        start
    };

    let mut current = start.canonicalize().ok()?;
    loop {
        let candidate = current.join("specforge.spec");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Load and parse `specforge.json` into a `SpecForgeJsonConfig`.
pub fn load_json_config(path: &Path) -> Result<SpecForgeJsonConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("error reading {}: {e}", path.display()))?;
    serde_json::from_str::<SpecForgeJsonConfig>(&content)
        .map_err(|e| format!("error parsing {}: {e}", path.display()))
}

/// Discover all .spec files in a directory, sorted for deterministic processing.
pub fn discover_spec_files(dir: &Path) -> Vec<PathBuf> {
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
