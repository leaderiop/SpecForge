use crate::{ResolvedFile, ResolvedProject};
use specforge_common::{Diagnostic, Severity};
use specforge_parser::{parse, SpecFile};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

pub fn resolve_project(spec_root: &Path) -> ResolvedProject {
    let mut diagnostics = Vec::new();
    let mut parsed_files: HashMap<PathBuf, SpecFile> = HashMap::new();
    let mut import_graph: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    // Discover all .spec files
    let spec_files = discover_spec_files(spec_root);

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
    for (path, spec_file) in &parsed_files {
        let mut deps = Vec::new();
        for import in &spec_file.imports {
            let target = resolve_import_path(spec_root, &import.path);
            if target.exists() {
                deps.push(target);
            } else {
                diagnostics.push(Diagnostic {
                    code: "E025".to_string(),
                    severity: Severity::Error,
                    message: format!("import target not found: {}", import.path),
                    span: Some(import.span.clone()),
                    suggestion: None,
                });
            }
        }
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
            files.push(ResolvedFile {
                path: path
                    .strip_prefix(spec_root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string(),
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
            });
        }
    }
    // Then add cyclic files (still present, just flagged)
    for (path, spec_file) in parsed_files {
        files.push(ResolvedFile {
            path: path
                .strip_prefix(spec_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string(),
            spec_file,
            import_targets: Vec::new(),
        });
    }

    ResolvedProject {
        files,
        diagnostics,
    }
}

fn discover_spec_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "spec") {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    files
}

fn resolve_import_path(spec_root: &Path, import_path: &str) -> PathBuf {
    let with_ext = format!("{}.spec", import_path);
    spec_root.join(with_ext)
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
            if deps.contains(node) {
                if let Some(deg) = in_deg.get_mut(other) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(other);
                    }
                }
            }
        }
    }

    result
}
