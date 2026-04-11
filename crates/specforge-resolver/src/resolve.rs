use crate::{FileScope, ReexportDeclaration, ResolvedFile, ResolvedProject};
use specforge_common::{find_close_match, Diagnostic, Severity};
use specforge_parser::{parse, SpecFile};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Component, Path, PathBuf};

/// Configuration for import path resolution.
#[derive(Debug, Clone, Default)]
pub struct ResolveConfig {
    pub path_aliases: Vec<PathAlias>,
}

/// A path alias mapping (e.g., `@shared` → `lib/shared` relative to spec_root).
#[derive(Debug, Clone)]
pub struct PathAlias {
    pub alias: String,
    pub target: String,
}

/// Result of resolving a single import path.
enum ImportResolution {
    Found(PathBuf),
    ExtensionStub { scope: String, name: String },
    NotFound,
}

/// Resolve a project using default config (backward-compatible entry point).
#[must_use]
pub fn resolve_project(spec_root: &Path) -> ResolvedProject {
    resolve_project_with_config(spec_root, &ResolveConfig::default())
}

/// Resolve a project with the given configuration.
#[must_use]
pub fn resolve_project_with_config(spec_root: &Path, config: &ResolveConfig) -> ResolvedProject {
    let mut diagnostics = Vec::new();
    let mut parsed_files: HashMap<PathBuf, SpecFile> = HashMap::new();
    let mut import_graph: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    // Discover all .spec files
    let spec_files = discover_spec_files(spec_root);

    // Build candidate list for fuzzy suggestions (relative stems without .spec)
    let candidates: Vec<String> = spec_files
        .iter()
        .filter_map(|p| {
            p.strip_prefix(spec_root).ok().map(|rel| {
                let s = rel.to_string_lossy();
                s.strip_suffix(".spec").unwrap_or(&s).to_string()
            })
        })
        .collect();

    // Parse all files
    for path in &spec_files {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    code: "E025".to_string(),
                    severity: Severity::Error,
                    message: format!("cannot read file: {}", e),
                    span: None,
                    suggestion: None,
                });
                continue;
            }
        };
        let rel = path
            .strip_prefix(spec_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let spec_file = parse(&source, &rel);
        parsed_files.insert(path.clone(), spec_file);
    }

    // Resolve imports and build the file dependency graph
    // Also track pub use reexports per file
    let mut reexport_map: HashMap<PathBuf, Vec<ReexportDeclaration>> = HashMap::new();

    for (path, spec_file) in &parsed_files {
        let mut deps = Vec::new();
        let mut reexports = Vec::new();
        for import in &spec_file.imports {
            match resolve_import_path(spec_root, path, import.path.as_str(), config) {
                ImportResolution::Found(ref target) => {
                    deps.push(target.clone());
                    if import.is_pub {
                        let target_rel = target
                            .strip_prefix(spec_root)
                            .unwrap_or(target)
                            .to_string_lossy()
                            .to_string();
                        reexports.push(ReexportDeclaration {
                            target_path: target_rel,
                            bindings: import.bindings.clone(),
                            span: import.span.clone(),
                        });
                    }
                }
                ImportResolution::ExtensionStub { scope, name } => {
                    diagnostics.push(Diagnostic {
                        code: "I004".to_string(),
                        severity: Severity::Info,
                        message: format!(
                            "extension import @{}/{} — extension not installed",
                            scope, name
                        ),
                        span: Some(import.span.clone()),
                        suggestion: None,
                    });
                }
                ImportResolution::NotFound => {
                    let suggestion = find_close_match(
                        import.path.as_str(),
                        candidates.iter().map(|s| s.as_str()),
                    )
                    .map(|m| format!("did you mean '{}'?", m));
                    diagnostics.push(Diagnostic {
                        code: "E025".to_string(),
                        severity: Severity::Error,
                        message: format!("import target not found: {}", import.path),
                        span: Some(import.span.clone()),
                        suggestion,
                    });
                }
            }
        }
        reexport_map.insert(path.clone(), reexports);
        import_graph.insert(path.clone(), deps);
    }

    // Detect import cycles
    let cycle_participants = detect_cycles(&import_graph);
    for cycle in &cycle_participants {
        let names: Vec<String> = cycle
            .iter()
            .map(|p| {
                p.strip_prefix(spec_root)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        diagnostics.push(Diagnostic {
            code: "W003".to_string(),
            severity: Severity::Warning,
            message: format!("circular import detected: {}", names.join(" -> ")),
            span: None,
            suggestion: None,
        });
    }

    let cycle_set: HashSet<&PathBuf> = cycle_participants.iter().flatten().collect();

    // Build topological order for non-cyclic files
    let order = topological_sort(&import_graph, &cycle_set);

    // Collect resolved files
    let mut files = Vec::new();
    // First add ordered (non-cyclic) files
    for path in &order {
        if let Some(spec_file) = parsed_files.remove(path) {
            let rel_path = path
                .strip_prefix(spec_root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            let reexports = reexport_map.remove(path).unwrap_or_default();
            files.push(ResolvedFile {
                path: rel_path,
                spec_file,
                import_targets: import_graph
                    .get(path)
                    .map(|deps| {
                        deps.iter()
                            .map(|d| {
                                d.strip_prefix(spec_root)
                                    .unwrap_or(d)
                                    .to_string_lossy()
                                    .to_string()
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                reexports,
            });
        }
    }
    // Then add cyclic files (still present, just flagged)
    for (path, spec_file) in parsed_files {
        let rel_path = path
            .strip_prefix(spec_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let reexports = reexport_map.remove(&path).unwrap_or_default();
        files.push(ResolvedFile {
            path: rel_path,
            spec_file,
            import_targets: Vec::new(),
            reexports,
        });
    }

    // Compute file scopes (declared + re-exported entities per file)
    let (file_scopes, scope_diagnostics) = compute_file_scopes(&files);
    diagnostics.extend(scope_diagnostics);

    ResolvedProject {
        files,
        diagnostics,
        file_scopes,
    }
}

fn discover_spec_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false) // Do not follow symlinks — prevents path traversal attacks
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "spec") && !entry.path_is_symlink() {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    files
}

/// 5-step import resolution cascade:
/// 1. Relative paths (`./` or `../`) — resolve from importing file's parent
/// 2. `@`-prefixed: check path aliases first, then extension stub
/// 3. Bare paths — resolve from spec_root
///
/// Each step applies index fallback: `path.spec` wins over `path/index.spec`.
fn resolve_import_path(
    spec_root: &Path,
    importing_file: &Path,
    import_path: &str,
    config: &ResolveConfig,
) -> ImportResolution {
    // Step 1: Relative paths
    if import_path.starts_with("./") || import_path.starts_with("../") {
        return try_resolve_relative(spec_root, importing_file, import_path);
    }

    // Step 2: @-prefixed paths (aliases or extension stubs)
    if let Some(rest) = import_path.strip_prefix('@') {
        // Try alias first
        if let Some(result) = try_resolve_alias(spec_root, rest, config) {
            return result;
        }
        // Fall through to extension stub
        return try_resolve_extension(rest);
    }

    // Step 3: Bare paths — resolve from spec_root
    try_resolve_bare(spec_root, import_path)
}

/// Resolve a relative import (`./foo` or `../bar`) from the importing file's directory.
/// The resolved path must stay within spec_root (path traversal protection).
fn try_resolve_relative(
    spec_root: &Path,
    importing_file: &Path,
    import_path: &str,
) -> ImportResolution {
    let base = importing_file.parent().unwrap_or(spec_root);
    let raw = base.join(import_path);
    let normalized = normalize_path(&raw);

    // Path traversal check: normalized path must start with spec_root
    if !normalized.starts_with(spec_root) {
        return ImportResolution::NotFound;
    }

    apply_index_fallback(&normalized)
}

/// Try to resolve an `@alias/rest` path via configured path aliases.
/// Returns `None` if the alias is not found (caller falls through to extension).
fn try_resolve_alias(
    spec_root: &Path,
    at_rest: &str,
    config: &ResolveConfig,
) -> Option<ImportResolution> {
    let (alias, rest) = match at_rest.split_once('/') {
        Some((a, r)) => (a, Some(r)),
        None => (at_rest, None),
    };

    let matched = config.path_aliases.iter().find(|a| a.alias == alias)?;
    let target_dir = spec_root.join(&matched.target);
    let full = match rest {
        Some(r) => target_dir.join(r),
        None => target_dir,
    };
    Some(apply_index_fallback(&full))
}

/// Recognize `@scope/name` as an extension import and return an ExtensionStub.
fn try_resolve_extension(at_rest: &str) -> ImportResolution {
    match at_rest.split_once('/') {
        Some((scope, name)) => ImportResolution::ExtensionStub {
            scope: scope.to_string(),
            name: name.to_string(),
        },
        None => ImportResolution::NotFound,
    }
}

/// Resolve a bare import path from spec_root.
fn try_resolve_bare(spec_root: &Path, import_path: &str) -> ImportResolution {
    apply_index_fallback(&spec_root.join(import_path))
}

/// Try `path.spec` first, then `path/index.spec`. Returns `NotFound` if neither exists.
fn apply_index_fallback(base: &Path) -> ImportResolution {
    let with_ext = base.with_extension("spec");
    if with_ext.is_file() {
        return ImportResolution::Found(with_ext);
    }
    // Check for path as directory with index.spec
    let index = base.join("index.spec");
    if index.is_file() {
        return ImportResolution::Found(index);
    }
    // Also try: maybe base already has the extension baked in by caller
    if base.extension().is_some_and(|e| e == "spec") && base.is_file() {
        return ImportResolution::Found(base.to_path_buf());
    }
    ImportResolution::NotFound
}

/// Lexically normalize a path (resolve `.` and `..` without requiring filesystem existence).
fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            other => result.push(other),
        }
    }
    result
}

fn detect_cycles(graph: &HashMap<PathBuf, Vec<PathBuf>>) -> Vec<Vec<PathBuf>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut on_stack = HashSet::new();
    let mut stack = Vec::new();

    for node in graph.keys() {
        if !visited.contains(node) {
            dfs_cycle(node, graph, &mut visited, &mut on_stack, &mut stack, &mut cycles);
        }
    }
    cycles
}

fn dfs_cycle(
    node: &PathBuf,
    graph: &HashMap<PathBuf, Vec<PathBuf>>,
    visited: &mut HashSet<PathBuf>,
    on_stack: &mut HashSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
    cycles: &mut Vec<Vec<PathBuf>>,
) {
    visited.insert(node.clone());
    on_stack.insert(node.clone());
    stack.push(node.clone());

    if let Some(deps) = graph.get(node) {
        for dep in deps {
            if !visited.contains(dep) {
                if graph.contains_key(dep) {
                    dfs_cycle(dep, graph, visited, on_stack, stack, cycles);
                }
            } else if on_stack.contains(dep) {
                // Found a cycle — extract it from the stack
                let start = stack.iter().position(|n| n == dep).unwrap();
                cycles.push(stack[start..].to_vec());
            }
        }
    }

    stack.pop();
    on_stack.remove(node);
}

fn topological_sort(
    graph: &HashMap<PathBuf, Vec<PathBuf>>,
    cycle_set: &HashSet<&PathBuf>,
) -> Vec<PathBuf> {
    // Kahn's algorithm: in_degree = number of imports each file has (to non-cyclic files)
    let mut in_deg: HashMap<&PathBuf, usize> = HashMap::new();
    for node in graph.keys() {
        if !cycle_set.contains(node) {
            in_deg.entry(node).or_insert(0);
        }
    }
    for (node, deps) in graph {
        if cycle_set.contains(node) {
            continue;
        }
        let dep_count = deps
            .iter()
            .filter(|d| !cycle_set.contains(d) && graph.contains_key(*d))
            .count();
        in_deg.insert(node, dep_count);
    }

    let mut queue: VecDeque<&PathBuf> = in_deg
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(node, _)| *node)
        .collect();

    // Sort queue for deterministic output
    let mut sorted_queue: Vec<&PathBuf> = queue.drain(..).collect();
    sorted_queue.sort();
    queue.extend(sorted_queue);

    let mut result = Vec::new();
    while let Some(node) = queue.pop_front() {
        result.push(node.clone());
        // Find all nodes that depend on this node
        for (other, deps) in graph {
            if cycle_set.contains(other) || result.contains(other) {
                continue;
            }
            if deps.contains(node)
                && let Some(deg) = in_deg.get_mut(other) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(other);
                    }
                }
        }
    }

    result
}

/// Compute file scopes: for each file, determine which entity IDs are declared
/// and which are exported (declared + re-exported via `pub use`).
///
/// Files are processed in the order given (assumed topologically sorted for
/// non-cyclic files, cyclic files appended at the end). For cycle participants
/// whose scope hasn't been computed yet, only their `declared` set is used
/// (no transitive resolution).
fn compute_file_scopes(files: &[ResolvedFile]) -> (HashMap<String, FileScope>, Vec<Diagnostic>) {
    let mut scopes: HashMap<String, FileScope> = HashMap::new();
    let mut diagnostics = Vec::new();

    // First pass: build declared sets for all files
    for file in files {
        let declared: HashSet<String> = file
            .spec_file
            .entities
            .iter()
            .map(|e| e.id.raw.to_string())
            .collect();
        scopes.insert(
            file.path.clone(),
            FileScope {
                exported: declared.clone(),
                declared,
            },
        );
    }

    // Second pass: process re-exports (files are in topological order,
    // so targets should already have their scopes computed)
    for file in files {
        if file.reexports.is_empty() {
            continue;
        }

        let mut additional_exports = HashSet::new();

        for reexport in &file.reexports {
            let target_exported = scopes
                .get(&reexport.target_path)
                .map(|s| &s.exported)
                .cloned()
                .unwrap_or_default();

            match &reexport.bindings {
                None => {
                    // pub use "target" — re-export all
                    additional_exports.extend(target_exported);
                }
                Some(bindings) => {
                    // pub use { A, B } from "target" — re-export selected
                    for binding in bindings {
                        if target_exported.contains(&binding.name) {
                            // Use alias if present, otherwise use the original name
                            let export_name = binding.alias.as_ref().unwrap_or(&binding.name);
                            additional_exports.insert(export_name.clone());
                        } else {
                            diagnostics.push(Diagnostic {
                                code: "W027".to_string(),
                                severity: Severity::Warning,
                                message: format!(
                                    "selective re-export '{}' not found in target '{}'",
                                    binding.name, reexport.target_path
                                ),
                                span: Some(reexport.span.clone()),
                                suggestion: None,
                            });
                        }
                    }
                }
            }
        }

        if let Some(scope) = scopes.get_mut(&file.path) {
            scope.exported.extend(additional_exports);
        }
    }

    (scopes, diagnostics)
}
