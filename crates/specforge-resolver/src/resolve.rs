use crate::{FileScope, ReexportDeclaration, ResolvedFile, ResolvedProject};
use specforge_common::{Diagnostic, Severity, find_close_match};
use specforge_parser::{SpecFile, parse};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

/// Configuration for import path resolution.
#[derive(Debug, Clone, Default)]
pub struct ResolveConfig {
    pub path_aliases: Vec<PathAlias>,
    /// `exclude` entries of `specforge.json`: a `.spec` file whose path
    /// relative to the spec root contains one is not discovered. Plain
    /// substrings, not globs (ADR 0004 D1-b).
    pub exclude: Vec<String>,
}

/// A path alias mapping (e.g., `@shared` → `lib/shared` relative to spec_root).
#[derive(Debug, Clone)]
pub struct PathAlias {
    pub alias: String,
    pub target: String,
}

/// Result of resolving a single import path.
enum Target {
    Found(PathBuf),
    ExtensionStub { scope: String, name: String },
    NotFound,
}

/// Resolve a project using default config (backward-compatible entry point).
#[must_use]
pub fn resolve_project(spec_root: &Path) -> ResolvedProject {
    resolve_project_with_config(spec_root, &ResolveConfig::default())
}

/// Discover, parse and resolve every `.spec` file under `spec_root` that no
/// `exclude` entry matches. Files come back in [`Resolution::order`].
#[must_use]
pub fn resolve_project_with_config(spec_root: &Path, config: &ResolveConfig) -> ResolvedProject {
    let mut diagnostics = Vec::new();
    let mut parsed: Vec<(String, SpecFile)> = Vec::new();
    for path in specforge_common::discover_spec_files(spec_root, &config.exclude) {
        let source = match std::fs::read_to_string(&path) {
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
        let rel = relative(spec_root, &path);
        let spec_file = parse(&source, &rel);
        parsed.push((rel, spec_file));
    }

    let mut resolution = {
        let files: Vec<(&str, &SpecFile)> = parsed.iter().map(|(p, f)| (p.as_str(), f)).collect();
        resolve_parsed(spec_root, &files, config, &|p: &Path| p.is_file())
    };
    diagnostics.append(&mut resolution.diagnostics);

    // Each file after the files it imports; the cyclic ones last.
    let mut parsed: HashMap<String, SpecFile> = parsed.into_iter().collect();
    let files = resolution
        .order
        .into_iter()
        .filter_map(|path| {
            let spec_file = parsed.remove(&path)?;
            Some(ResolvedFile {
                import_targets: resolution.import_targets.remove(&path).unwrap_or_default(),
                reexports: resolution.reexports.remove(&path).unwrap_or_default(),
                path,
                spec_file,
            })
        })
        .collect();

    ResolvedProject {
        files,
        diagnostics,
        file_scopes: resolution.file_scopes,
    }
}

/// What resolving the imports of a set of parsed files yields.
#[derive(Debug, Default)]
pub struct Resolution {
    /// Per file in path order, each import's E025 or I004; then one W113
    /// per import cycle, in sorted order; then the W027s of selective
    /// re-exports.
    pub diagnostics: Vec<Diagnostic>,
    /// Each file's resolved import targets (spec-root-relative).
    pub import_targets: BTreeMap<String, Vec<String>>,
    /// Each file's `pub use` re-exports.
    pub reexports: BTreeMap<String, Vec<ReexportDeclaration>>,
    pub file_scopes: HashMap<String, FileScope>,
    /// Every file, each after the files it imports (smallest path first
    /// among the ready ones), then the files in import cycles, by path.
    pub order: Vec<String>,
}

/// Resolve the imports of files that are already parsed, keyed by their
/// path relative to `spec_root`: E025, I004, W113 and W027, without
/// walking the directory. `exists` says whether an import's candidate
/// target file exists. Watch and the LSP run this over their cached
/// parses after every change; `resolve_project` over what it discovered.
#[must_use]
pub fn resolve_parsed(
    spec_root: &Path,
    files: &[(&str, &SpecFile)],
    config: &ResolveConfig,
    exists: &dyn Fn(&Path) -> bool,
) -> Resolution {
    let mut files: Vec<(&str, &SpecFile)> = files.to_vec();
    files.sort_by(|a, b| a.0.cmp(b.0));
    let mut diagnostics = Vec::new();

    // Candidates for fuzzy suggestions (relative stems without .spec).
    let candidates: Vec<&str> = files
        .iter()
        .map(|(path, _)| path.strip_suffix(".spec").unwrap_or(path))
        .collect();

    // Resolve imports and build the file dependency graph, tracking each
    // file's `pub use` re-exports.
    let mut import_graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut reexport_map: BTreeMap<String, Vec<ReexportDeclaration>> = BTreeMap::new();
    for (path, spec_file) in &files {
        let importing_file = spec_root.join(path);
        let mut deps = Vec::new();
        let mut reexports = Vec::new();
        for import in &spec_file.imports {
            match resolve_import_path(
                spec_root,
                &importing_file,
                import.path.as_str(),
                config,
                exists,
            ) {
                Target::Found(ref target) => {
                    let target_rel = relative(spec_root, target);
                    if import.is_pub {
                        reexports.push(ReexportDeclaration {
                            target_path: target_rel.clone(),
                            bindings: import.bindings.clone(),
                            span: import.span.clone(),
                        });
                    }
                    deps.push(target_rel);
                }
                Target::ExtensionStub { scope, name } => {
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
                Target::NotFound => {
                    let suggestion =
                        find_close_match(import.path.as_str(), candidates.iter().copied())
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
        reexport_map.insert(path.to_string(), reexports);
        import_graph.insert(path.to_string(), deps);
    }

    // Import cycles: each named from its smallest path, in sorted order.
    let cycles = detect_cycles(&import_graph);
    for cycle in &cycles {
        diagnostics.push(Diagnostic {
            code: "W113".to_string(),
            severity: Severity::Warning,
            message: format!("circular import detected: {}", cycle.join(" -> ")),
            span: None,
            suggestion: Some("break the cycle by removing one of the `use` imports or extracting shared entities into a separate file".to_string()),
        });
    }

    // File scopes (declared + re-exported entities per file), computed in
    // topological order with the cyclic files last.
    let cyclic: BTreeSet<&str> = cycles.iter().flatten().map(String::as_str).collect();
    let mut order = topological_sort(&import_graph, &cyclic);
    order.extend(cyclic.iter().copied());
    let by_path: HashMap<&str, &SpecFile> = files.iter().copied().collect();
    let scope_inputs: Vec<(&str, &SpecFile, &[ReexportDeclaration])> = order
        .iter()
        .filter_map(|path| {
            Some((
                *path,
                *by_path.get(path)?,
                reexport_map.get(*path).map(Vec::as_slice).unwrap_or(&[]),
            ))
        })
        .collect();
    let (file_scopes, scope_diagnostics) = compute_file_scopes(&scope_inputs);
    diagnostics.extend(scope_diagnostics);

    let order = order.into_iter().map(str::to_string).collect();
    Resolution {
        diagnostics,
        import_targets: import_graph,
        reexports: reexport_map,
        file_scopes,
        order,
    }
}

/// `path` relative to `spec_root`, as diagnostics and file keys print it.
fn relative(spec_root: &Path, path: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(spec_root) {
        return rel.to_string_lossy().to_string();
    }
    // A relative import's target is normalized; the spec root may not be.
    let path = normalize_path(path);
    path.strip_prefix(normalize_path(spec_root))
        .unwrap_or(&path)
        .to_string_lossy()
        .to_string()
}

/// The file `use "<import_path>"` in `importing_file` (relative to
/// `spec_root`) names, relative to `spec_root`, by the cascade the compile
/// resolves imports with. `None` when it names no file under the spec
/// root (E025) or an extension (I004).
#[must_use]
pub fn resolve_import(
    spec_root: &Path,
    importing_file: &str,
    import_path: &str,
    config: &ResolveConfig,
) -> Option<String> {
    match resolve_import_path(
        spec_root,
        &spec_root.join(importing_file),
        import_path,
        config,
        &|p: &Path| p.is_file(),
    ) {
        Target::Found(target) => Some(relative(spec_root, &target)),
        Target::ExtensionStub { .. } | Target::NotFound => None,
    }
}

/// 5-step import resolution cascade:
/// 1. Relative paths (`./` or `../`) — resolve from importing file's parent
/// 2. `@`-prefixed: check path aliases first, then extension stub
/// 3. Bare paths — resolve from spec_root
///
/// Each step applies index fallback: `path.spec` wins over `path/index.spec`.
/// A target outside the spec root is not found, whichever step names it.
fn resolve_import_path(
    spec_root: &Path,
    importing_file: &Path,
    import_path: &str,
    config: &ResolveConfig,
    exists: &dyn Fn(&Path) -> bool,
) -> Target {
    match cascade(spec_root, importing_file, import_path, config, exists) {
        Target::Found(target)
            if !normalize_path(&target).starts_with(normalize_path(spec_root)) =>
        {
            Target::NotFound
        }
        other => other,
    }
}

fn cascade(
    spec_root: &Path,
    importing_file: &Path,
    import_path: &str,
    config: &ResolveConfig,
    exists: &dyn Fn(&Path) -> bool,
) -> Target {
    // Step 1: Relative paths
    if import_path.starts_with("./") || import_path.starts_with("../") {
        return try_resolve_relative(spec_root, importing_file, import_path, exists);
    }

    // Step 2: @-prefixed paths (aliases or extension stubs)
    if let Some(rest) = import_path.strip_prefix('@') {
        // Try alias first
        if let Some(result) = try_resolve_alias(spec_root, rest, config, exists) {
            return result;
        }
        // Fall through to extension stub
        return try_resolve_extension(rest);
    }

    // Step 3: Bare paths — resolve from spec_root
    try_resolve_bare(spec_root, import_path, exists)
}

/// Resolve a relative import (`./foo` or `../bar`) from the importing file's directory.
fn try_resolve_relative(
    spec_root: &Path,
    importing_file: &Path,
    import_path: &str,
    exists: &dyn Fn(&Path) -> bool,
) -> Target {
    let base = importing_file.parent().unwrap_or(spec_root);
    // Normalized first, so `..` cannot reach a file outside the spec root
    // that the lexical path would not.
    apply_index_fallback(&normalize_path(&base.join(import_path)), exists)
}

/// Try to resolve an `@alias/rest` path via configured path aliases.
/// Returns `None` if the alias is not found (caller falls through to extension).
fn try_resolve_alias(
    spec_root: &Path,
    at_rest: &str,
    config: &ResolveConfig,
    exists: &dyn Fn(&Path) -> bool,
) -> Option<Target> {
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
    Some(apply_index_fallback(&full, exists))
}

/// Recognize `@scope/name` as an extension import and return an ExtensionStub.
fn try_resolve_extension(at_rest: &str) -> Target {
    match at_rest.split_once('/') {
        Some((scope, name)) => Target::ExtensionStub {
            scope: scope.to_string(),
            name: name.to_string(),
        },
        None => Target::NotFound,
    }
}

/// Resolve a bare import path from spec_root.
fn try_resolve_bare(spec_root: &Path, import_path: &str, exists: &dyn Fn(&Path) -> bool) -> Target {
    apply_index_fallback(&spec_root.join(import_path), exists)
}

/// Try `path.spec` first, then `path/index.spec`. Returns `NotFound` if neither exists.
fn apply_index_fallback(base: &Path, exists: &dyn Fn(&Path) -> bool) -> Target {
    let with_ext = base.with_extension("spec");
    if exists(&with_ext) {
        return Target::Found(with_ext);
    }
    // Check for path as directory with index.spec
    let index = base.join("index.spec");
    if exists(&index) {
        return Target::Found(index);
    }
    // Also try: maybe base already has the extension baked in by caller
    if base.extension().is_some_and(|e| e == "spec") && exists(base) {
        return Target::Found(base.to_path_buf());
    }
    Target::NotFound
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

/// Every import cycle found by a depth-first walk in path order, each
/// rotated to start at its smallest path, sorted and without repeats.
fn detect_cycles(graph: &BTreeMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let mut cycles = BTreeSet::new();
    let mut visited = HashSet::new();
    let mut on_stack = HashSet::new();
    let mut stack = Vec::new();

    for node in graph.keys() {
        if !visited.contains(node.as_str()) {
            dfs_cycle(
                node,
                graph,
                &mut visited,
                &mut on_stack,
                &mut stack,
                &mut cycles,
            );
        }
    }
    cycles.into_iter().collect()
}

fn dfs_cycle<'a>(
    node: &'a str,
    graph: &'a BTreeMap<String, Vec<String>>,
    visited: &mut HashSet<&'a str>,
    on_stack: &mut HashSet<&'a str>,
    stack: &mut Vec<&'a str>,
    cycles: &mut BTreeSet<Vec<String>>,
) {
    visited.insert(node);
    on_stack.insert(node);
    stack.push(node);

    if let Some(deps) = graph.get(node) {
        for dep in deps {
            if !visited.contains(dep.as_str()) {
                if graph.contains_key(dep) {
                    dfs_cycle(dep, graph, visited, on_stack, stack, cycles);
                }
            } else if on_stack.contains(dep.as_str()) {
                // Found a cycle: extract it from the stack and start it at
                // its smallest path.
                let start = stack.iter().position(|n| *n == dep).unwrap();
                let mut cycle: Vec<String> = stack[start..].iter().map(|s| s.to_string()).collect();
                let smallest = (0..cycle.len()).min_by_key(|&i| &cycle[i]).unwrap_or(0);
                cycle.rotate_left(smallest);
                cycles.insert(cycle);
            }
        }
    }

    stack.pop();
    on_stack.remove(node);
}

/// The non-cyclic files, each after the files it imports (Kahn's
/// algorithm, smallest path first among the ready ones).
fn topological_sort<'a>(
    graph: &'a BTreeMap<String, Vec<String>>,
    cyclic: &BTreeSet<&str>,
) -> Vec<&'a str> {
    let mut in_deg: BTreeMap<&str, usize> = BTreeMap::new();
    let mut importers: HashMap<&str, BTreeSet<&str>> = HashMap::new();
    for (node, deps) in graph {
        if cyclic.contains(node.as_str()) {
            continue;
        }
        let deps: BTreeSet<&str> = deps
            .iter()
            .map(String::as_str)
            .filter(|d| !cyclic.contains(d) && graph.contains_key(*d))
            .collect();
        in_deg.insert(node, deps.len());
        for dep in deps {
            importers.entry(dep).or_default().insert(node);
        }
    }

    let mut ready: BTreeSet<&str> = in_deg
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(node, _)| *node)
        .collect();
    let mut result = Vec::new();
    while let Some(node) = ready.pop_first() {
        result.push(node);
        for importer in importers.get(node).into_iter().flatten() {
            if let Some(deg) = in_deg.get_mut(importer) {
                *deg -= 1;
                if *deg == 0 {
                    ready.insert(importer);
                }
            }
        }
    }
    result
}

/// Compute file scopes: for each file, determine which entity IDs are declared
/// and which are exported (declared + re-exported via `pub use`).
///
/// Files are processed in the order given (topologically sorted for
/// non-cyclic files, cyclic files appended at the end). For cycle participants
/// whose scope hasn't been computed yet, only their `declared` set is used
/// (no transitive resolution).
fn compute_file_scopes(
    files: &[(&str, &SpecFile, &[ReexportDeclaration])],
) -> (HashMap<String, FileScope>, Vec<Diagnostic>) {
    let mut scopes: HashMap<String, FileScope> = HashMap::new();
    let mut diagnostics = Vec::new();

    // First pass: build declared sets for all files
    for (path, spec_file, _) in files {
        let declared: HashSet<String> = spec_file
            .entities
            .iter()
            .map(|e| e.id.raw.to_string())
            .collect();
        scopes.insert(
            path.to_string(),
            FileScope {
                exported: declared.clone(),
                declared,
            },
        );
    }

    // Second pass: process re-exports (files are in topological order,
    // so targets should already have their scopes computed)
    for (path, _, reexports) in files {
        if reexports.is_empty() {
            continue;
        }

        let mut additional_exports = HashSet::new();

        for reexport in reexports.iter() {
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

        if let Some(scope) = scopes.get_mut(*path) {
            scope.exported.extend(additional_exports);
        }
    }

    (scopes, diagnostics)
}
