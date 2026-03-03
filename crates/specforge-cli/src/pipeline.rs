use specforge_common::{
    CompilerConfig, ConflictResolution, Diagnostic, FieldRegistry, KindConflictResolution,
    KindRegistry, SourceSpan, SpecForgeJsonConfig, ValidationCode,
};
use specforge_graph::{build_graph, FileIndex, SpecGraph};
use specforge_parser::{parse, SpecFile};
use specforge_resolver::{resolve_with_config, FileGraph};
use specforge_wasm::WasmRuntime;
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
    pub field_registry: FieldRegistry,
    /// External config from specforge.json (None = legacy spec-block mode).
    pub external_config: Option<CompilerConfig>,
    /// Wasm plugin runtime (populated when Wasm plugins are configured).
    pub wasm_runtime: Option<WasmRuntime>,
    /// Entity kind registry (built-in + plugin-registered + define-block).
    pub kind_registry: KindRegistry,
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
    let saved_external_config = external_config.clone();
    let resolved = resolve_with_config(parsed_files, &spec_root_str, external_config);

    // Step 3b: Build field registry
    let mut registry = build_field_registry(&resolved.config);

    // Step 3c: Load Wasm plugins (if any local path plugins configured)
    let (mut wasm_runtime, wasm_diagnostics, wasm_entities) = load_wasm_packages(
        &resolved.config,
        &mut registry,
        &project_root,
    );
    let mut wasm_plugin_diagnostics = wasm_diagnostics;

    // Step 3d: Build kind registry from builtins + define kinds + wasm plugin entities
    let mut kind_registry = KindRegistry::with_builtins();

    // Register define-block kinds from config
    let define_kind_names: Vec<String> = resolved.config.custom_entities.keys().cloned().collect();
    kind_registry.add_define_kinds(&define_kind_names);

    // Register kinds from Wasm plugins (entities registered during initialize)
    // Group registered entities by source plugin
    {
        let mut by_plugin: HashMap<String, Vec<(String, bool)>> = HashMap::new();
        for entity in &wasm_entities.entities {
            by_plugin
                .entry(entity.source_plugin.clone())
                .or_default()
                .push((entity.name.clone(), entity.testable));
        }
        for (plugin_package, kinds) in &by_plugin {
            if !plugin_package.is_empty() {
                kind_registry.register_plugin(
                    plugin_package,
                    kinds,
                    &resolved.config.entity_kind_policy,
                    &resolved.config.entity_kind_overrides,
                );
            }
        }
    }

    // Emit kind conflict diagnostics (E022/E023/W027)
    let mut kind_conflict_diagnostics = Vec::new();
    for conflict in kind_registry.conflicts() {
        let span = SourceSpan::file_start("specforge.json");
        match &conflict.resolution {
            KindConflictResolution::Unresolved => {
                if conflict.first_source == "(built-in)" {
                    kind_conflict_diagnostics.push(
                        Diagnostic::new(
                            ValidationCode::E023,
                            span,
                            format!(
                                "plugin `{}` cannot register entity kind `{}`: shadows built-in keyword",
                                conflict.second_source, conflict.kind_name
                            ),
                        )
                        .with_help("entity kind names cannot shadow built-in keywords"),
                    );
                } else if conflict.first_source == "(define)" {
                    kind_conflict_diagnostics.push(
                        Diagnostic::new(
                            ValidationCode::E022,
                            span,
                            format!(
                                "entity kind conflict: `{}` already defined by a `define` block, cannot be registered by `{}`",
                                conflict.kind_name, conflict.second_source
                            ),
                        )
                        .with_help("remove the define block or rename the plugin's entity kind"),
                    );
                } else {
                    kind_conflict_diagnostics.push(
                        Diagnostic::new(
                            ValidationCode::E022,
                            span,
                            format!(
                                "entity kind conflict: `{}` registered by both `{}` and `{}`",
                                conflict.kind_name, conflict.first_source, conflict.second_source
                            ),
                        )
                        .with_help(format!(
                            "add to specforge.json: \"entity_kinds\": {{ \"{}\": \"{}\" }}",
                            conflict.kind_name, conflict.first_source
                        )),
                    );
                }
            }
            KindConflictResolution::LoadOrder { winner } => {
                kind_conflict_diagnostics.push(
                    Diagnostic::new(
                        ValidationCode::W027,
                        span,
                        format!(
                            "entity kind `{}` resolved to `{}` by load order (priority policy)",
                            conflict.kind_name, winner
                        ),
                    )
                    .with_help("use entity_kinds overrides for explicit resolution"),
                );
            }
            _ => {}
        }
    }

    // Check for unresolved enhancement conflicts → emit E017/E018 diagnostics
    let mut enhancement_diagnostics = Vec::new();
    for conflict in registry.conflicts() {
        let span = SourceSpan::file_start("specforge.json");
        match &conflict.resolution {
            ConflictResolution::Unresolved => {
                if conflict.first_plugin == "(built-in)" {
                    enhancement_diagnostics.push(
                        Diagnostic::new(
                            ValidationCode::E018,
                            span,
                            format!(
                                "plugin `{}` cannot shadow built-in field `{}` on entity `{}`",
                                conflict.second_plugin, conflict.field_name, conflict.entity_kind
                            ),
                        )
                        .with_help("built-in fields cannot be overridden by plugins"),
                    );
                } else {
                    enhancement_diagnostics.push(
                        Diagnostic::new(
                            ValidationCode::E017,
                            span,
                            format!(
                                "enhancement conflict: field `{}` on entity `{}` claimed by `{}` and `{}`",
                                conflict.field_name, conflict.entity_kind,
                                conflict.first_plugin, conflict.second_plugin
                            ),
                        )
                        .with_help(format!(
                            "add to specforge.json: \"enhancement_overrides\": {{ \"{}.{}\": \"{}\" }}",
                            conflict.entity_kind, conflict.field_name, conflict.first_plugin
                        )),
                    );
                }
            }
            ConflictResolution::LoadOrder { winner } => {
                enhancement_diagnostics.push(
                    Diagnostic::new(
                        ValidationCode::W023,
                        span,
                        format!(
                            "field `{}` on entity `{}` resolved to `{}` by load order (priority policy)",
                            conflict.field_name, conflict.entity_kind, winner
                        ),
                    )
                    .with_help("use enhancement_overrides for explicit resolution"),
                );
            }
            _ => {}
        }
    }

    // Step 4: Build graph
    let graph_result = build_graph(&resolved.files, &registry);

    // Step 5: Validate
    let config = resolved.config;
    let file_graph = resolved.file_graph;
    let validation_bag =
        specforge_validator::validate(&resolved.files, &graph_result.graph, &config, spec_root_dir.as_path(), &registry);

    // Step 5b: Run Wasm plugin validation (if plugins loaded)
    if let Some(ref mut runtime) = wasm_runtime {
        let graph_json = serialize_graph_for_wasm(&graph_result.graph);
        let wasm_validate = runtime.validate_all(&graph_json);
        wasm_plugin_diagnostics.extend(wasm_validate.diagnostics);
        for err in wasm_validate.errors {
            wasm_plugin_diagnostics.push(Diagnostic::new(
                ValidationCode::E019,
                SourceSpan::file_start("specforge.json"),
                err.to_string(),
            ));
        }
    }

    // Step 6: Merge parse + resolver + validator + enhancement + kind conflict + wasm diagnostics, sort
    let mut all_diagnostics = resolved.diagnostics.sorted();
    all_diagnostics.extend(validation_bag.sorted());
    all_diagnostics.extend(parse_diagnostics);
    all_diagnostics.extend(enhancement_diagnostics);
    all_diagnostics.extend(kind_conflict_diagnostics);
    all_diagnostics.extend(wasm_plugin_diagnostics);
    all_diagnostics.sort();

    Ok(PipelineResult {
        files: resolved.files,
        graph: graph_result.graph,
        file_index: graph_result.file_index,
        file_graph,
        diagnostics: all_diagnostics,
        config,
        sources,
        field_registry: registry,
        external_config: saved_external_config,
        wasm_runtime,
        kind_registry,
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

/// Registered entities collected during Wasm package initialization.
pub struct WasmPackageEntities {
    pub entities: Vec<specforge_wasm::host_functions::RegisteredEntity>,
}

/// Load Wasm packages from the project config.
///
/// Discovers local-path package specifiers, loads manifests, validates peer deps,
/// performs topological sort, and initializes the runtime.
/// Returns the runtime, diagnostics, and registered entities for kind registry building.
fn load_wasm_packages(
    config: &CompilerConfig,
    registry: &mut FieldRegistry,
    project_root: &Option<ProjectRoot>,
) -> (Option<WasmRuntime>, Vec<Diagnostic>, WasmPackageEntities) {
    let mut diagnostics = Vec::new();

    // Collect Wasm plugin specifiers from config
    // These are plugin names NOT matched by Module::from_package_name() — i.e., local paths
    let wasm_specifiers: Vec<String> = config
        .wasm_package_specifiers
        .clone();

    if wasm_specifiers.is_empty() {
        return (None, diagnostics, WasmPackageEntities { entities: Vec::new() });
    }

    let root_dir = match project_root {
        Some(ProjectRoot::Json(p)) => p.parent().unwrap_or(Path::new(".")).to_path_buf(),
        Some(ProjectRoot::Spec(p)) => p.parent().unwrap_or(Path::new(".")).to_path_buf(),
        None => PathBuf::from("."),
    };

    // Discover manifests
    let (manifests, discover_errors) =
        specforge_wasm::discover::discover_packages(&wasm_specifiers, &root_dir);

    for err in discover_errors {
        diagnostics.push(Diagnostic::new(
            ValidationCode::E019,
            SourceSpan::file_start("specforge.json"),
            err.to_string(),
        ));
    }

    if manifests.is_empty() {
        return (None, diagnostics, WasmPackageEntities { entities: Vec::new() });
    }

    // Validate peer dependencies
    let peer_errors = specforge_wasm::peer_deps::validate_peer_dependencies(&manifests);
    for err in &peer_errors {
        let code = match err {
            specforge_wasm::WasmError::CycleDetected { .. } => ValidationCode::E021,
            _ => ValidationCode::E020,
        };
        diagnostics.push(Diagnostic::new(
            code,
            SourceSpan::file_start("specforge.json"),
            err.to_string(),
        ));
    }

    // Topological sort
    let sorted_manifests = match specforge_wasm::peer_deps::topological_sort(&manifests) {
        Ok(order) => order.into_iter().map(|i| manifests[i].clone()).collect(),
        Err(err) => {
            diagnostics.push(Diagnostic::new(
                ValidationCode::E021,
                SourceSpan::file_start("specforge.json"),
                err.to_string(),
            ));
            manifests
        }
    };

    // Register plugin enhancements in the field registry
    for manifest in &sorted_manifests {
        if !manifest.enhancements.is_empty() {
            registry.register_plugin(
                &manifest.package,
                &manifest.enhancements,
                &manifest.dynamic_edge_types,
                &config.enhancement_policy,
                &config.enhancement_overrides,
            );
        }
    }

    // Load Wasm runtime
    let mut runtime = WasmRuntime::new();
    let load_errors = runtime.load_packages(sorted_manifests);
    for err in load_errors {
        diagnostics.push(Diagnostic::new(
            ValidationCode::E019,
            SourceSpan::file_start("specforge.json"),
            err.to_string(),
        ));
    }

    // Initialize plugins
    let init_result = runtime.initialize_all();
    let wasm_entities = WasmPackageEntities {
        entities: init_result.entities,
    };
    diagnostics.extend(init_result.diagnostics);
    for err in init_result.errors {
        diagnostics.push(Diagnostic::new(
            ValidationCode::E019,
            SourceSpan::file_start("specforge.json"),
            err.to_string(),
        ));
    }

    if runtime.package_count() > 0 {
        (Some(runtime), diagnostics, wasm_entities)
    } else {
        (None, diagnostics, wasm_entities)
    }
}

/// Serialize a SpecGraph to JSON for the `query_graph` host function.
fn serialize_graph_for_wasm(graph: &SpecGraph) -> String {
    let nodes: Vec<serde_json::Value> = graph
        .nodes()
        .map(|n| {
            serde_json::json!({
                "id": n.id.raw(),
                "kind": format!("{}", n.kind),
                "title": n.title,
                "file": n.file,
            })
        })
        .collect();

    let edges: Vec<serde_json::Value> = graph
        .edges()
        .map(|(src, tgt, edge)| {
            serde_json::json!({
                "from": src.id.raw(),
                "to": tgt.id.raw(),
                "edge_type": format!("{}", edge.edge_type),
                "field_name": edge.field_name,
            })
        })
        .collect();

    serde_json::json!({
        "nodes": nodes,
        "edges": edges,
    })
    .to_string()
}

/// Build a `FieldRegistry` from the compiler config.
///
/// Built-in plugins have no enhancements to register — their fields are
/// already wired via `FieldRegistry::with_builtins()`. Wasm package
/// enhancements are registered separately in `load_wasm_packages()`.
pub fn build_field_registry(_config: &CompilerConfig) -> FieldRegistry {
    FieldRegistry::with_builtins()
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
