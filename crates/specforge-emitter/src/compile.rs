use specforge_common::{load_project_config, Diagnostic, Severity};
use specforge_graph::{build_graph, build_graph_with_config, Graph, GraphConfig};
use specforge_registry::{
    populate_registries, validate_manifest, validate_manifest_consistency,
    compilation::{detect_mistyped_references, detect_unknown_entity_kinds, detect_unknown_entity_fields},
    EdgeRegistry, FieldRegistry, KindRegistry, ManifestV2,
    register_surface_contributions, SurfaceContributions, SurfaceRegistryEntry,
    validation_engine::{
        execute_pattern, parse_all_rule_patterns, ValidationEntity, ValidationRulePattern,
    },
};
use specforge_resolver::{resolve_project, ResolvedProject};
use specforge_validator::{validate_with_config, ValidatorConfig};
use specforge_wasm::WasmRuntime;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Full compilation result with all extension-aware validation.
/// Used by CLI, MCP, and LSP for consistent results.
#[allow(dead_code)]
pub struct CompilationContext {
    pub graph: Graph,
    pub kind_registry: KindRegistry,
    pub field_registry: FieldRegistry,
    pub edge_registry: EdgeRegistry,
    pub diagnostics: Vec<Diagnostic>,
    pub resolved: ResolvedProject,
    pub validation_patterns: Vec<ValidationRulePattern>,
    pub extension_info: Vec<(String, String)>,
    pub surface_entries: Vec<SurfaceRegistryEntry>,
    /// Raw surface contributions from manifests (needed for MCP descriptor generation).
    pub manifest_surfaces: Vec<(String, SurfaceContributions)>,
    /// Raw extension manifests (needed for outline rendering).
    pub manifests: Vec<ManifestV2>,
    pub spec_root: std::path::PathBuf,
}

/// Run the full 14-step compilation pipeline.
///
/// This is the single source of truth for compilation. All consumers
/// (CLI, MCP, LSP) should call this to get consistent results.
///
/// Extensions are loaded from the built-in runtime (native Rust implementations
/// of `@specforge/product`, `@specforge/software`, `@specforge/governance`, `@specforge/formal`).
pub fn compile(path: &Path) -> CompilationContext {
    let runtime = crate::builtins::default_runtime();
    compile_with_runtime(path, Some(&runtime))
}

/// Run the full compilation pipeline with an optional Wasm runtime.
///
/// When `runtime` is `Some`, extensions are loaded via the protocol
/// (`__handshake` / `__describe`). When `None`, no extensions are loaded.
pub fn compile_with_runtime(path: &Path, runtime: Option<&dyn WasmRuntime>) -> CompilationContext {
    let mut diagnostics = Vec::new();

    // 1. Load project config
    let config = load_project_config(path);

    // 2. Load extensions via protocol
    let manifests = match runtime {
        Some(rt) => load_extensions(&config.extensions, rt, &mut diagnostics),
        None => Vec::new(),
    };

    // 3. Populate registries
    let (kind_reg, field_reg, edge_reg, pop_diags) = populate_registries(&manifests);
    diagnostics.extend(pop_diags);

    // 4. Parse validation rules from manifests
    let rule_inputs: Vec<(String, Vec<_>)> = manifests
        .iter()
        .map(|m| (m.name.clone(), m.validation_rules.clone()))
        .collect();
    let (patterns, rule_diags) = parse_all_rule_patterns(&rule_inputs);
    diagnostics.extend(rule_diags);

    // 5. Build keyword->extension index for I004 messages
    let known_extension_keywords: HashMap<String, String> = manifests
        .iter()
        .flat_map(|m| {
            m.entity_kinds
                .iter()
                .map(move |k| (k.keyword.clone(), m.name.clone()))
        })
        .collect();

    let bidirectional_pairs = field_reg.bidirectional_pairs();

    // 6. Build GraphConfig from registries
    let graph_config = GraphConfig {
        installed_keywords: kind_reg.keywords().cloned().collect(),
        known_provider_schemes: HashSet::new(),
        known_extension_keywords,
        bidirectional_pairs,
    };

    // 7. Resolve project (use configured spec_root, default to project root)
    let spec_root = match &config.spec_root {
        Some(sr) => path.join(sr),
        None => path.to_path_buf(),
    };
    let resolved = resolve_project(&spec_root);
    diagnostics.extend(resolved.diagnostics.clone());

    // 8. Build graph
    let spec_files: Vec<_> = resolved.files.iter().map(|f| f.spec_file.clone()).collect();
    let (mut graph, build_diags) = build_graph_with_config(&spec_files, &graph_config);
    diagnostics.extend(build_diags);

    // 8a. Re-resolve references with single-reference field awareness.
    // build_graph_with_config only resolves ReferenceList fields. Now that
    // we have the FieldRegistry, re-resolve to also create edges for single
    // Reference fields (e.g., journey.persona -> persona entity).
    if !field_reg.is_empty() {
        let single_ref_fields: HashSet<(String, String)> = field_reg
            .iter()
            .filter(|(_, _, entry)| entry.field_type == specforge_registry::ManifestFieldType::Reference)
            .map(|(kind, field, _)| (kind.to_string(), field.to_string()))
            .collect();
        if !single_ref_fields.is_empty() {
            let ref_diags = graph.resolve_references_with_singles(&single_ref_fields);
            // Replace the E001 diagnostics from the initial resolution.
            // Remove previously-added E001s from build_diags and add the new ones.
            diagnostics.retain(|d| d.code != "E001");
            diagnostics.extend(ref_diags);
        }
    }

    // 9. Run core validation (with file reference fields from registries)
    let file_ref_fields: Vec<String> = field_reg
        .iter()
        .filter(|(_, _, entry)| entry.file_reference)
        .map(|(_, field_name, _)| field_name.to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let validator_config = ValidatorConfig {
        spec_root: spec_root.clone(),
        file_reference_fields: file_ref_fields,
    };
    let validation_diags = validate_with_config(&graph, &validator_config);
    diagnostics.extend(validation_diags);

    // 10. Run strict field validation against extension registries
    if !kind_reg.is_empty() {
        let entity_kind_info: Vec<_> = graph
            .nodes()
            .iter()
            .map(|n| (n.kind.raw.to_string(), n.id.raw.to_string(), n.source_span.clone()))
            .collect();
        let kind_diags = detect_unknown_entity_kinds(&entity_kind_info, &kind_reg, None);
        diagnostics.extend(kind_diags);

        let entity_field_info: Vec<_> = graph
            .nodes()
            .iter()
            .map(|n| {
                let field_names: Vec<String> = n.fields.entries().iter().map(|e| e.key.to_string()).collect();
                (n.kind.raw.to_string(), n.id.raw.to_string(), field_names, n.source_span.clone())
            })
            .collect();
        let field_diags = detect_unknown_entity_fields(&entity_field_info, &kind_reg, &field_reg);
        diagnostics.extend(field_diags);

        // 10a. Validate reference fields against target_kind constraints (E022)
        let node_kind_index: HashMap<String, String> = graph
            .nodes()
            .iter()
            .map(|n| (n.id.raw.to_string(), n.kind.raw.to_string()))
            .collect();
        let entity_ref_info: Vec<_> = graph
            .nodes()
            .iter()
            .map(|n| {
                let ref_fields: Vec<(String, Vec<String>)> = n
                    .fields
                    .entries()
                    .iter()
                    .filter_map(|e| {
                        if let specforge_parser::FieldValue::ReferenceList(refs) = &e.value {
                            Some((e.key.to_string(), refs.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                (n.kind.raw.to_string(), n.id.raw.to_string(), ref_fields, n.source_span.clone())
            })
            .collect();
        let ref_diags = detect_mistyped_references(&entity_ref_info, &field_reg, &kind_reg, &node_kind_index);
        diagnostics.extend(ref_diags);
    }

    // 11. Build edge label mapping (manifest label -> field name used in graph)
    let edge_label_to_field: HashMap<String, String> = manifests
        .iter()
        .flat_map(|m| m.entity_kinds.iter())
        .flat_map(|k| k.fields.iter())
        .filter_map(|f| f.edge.as_ref().map(|e| (e.clone(), f.name.clone())))
        .collect();

    // 12. Run extension validation rules (declarative patterns)
    let extension_diags = run_extension_validation(&patterns, &graph, &edge_label_to_field);
    diagnostics.extend(extension_diags);

    // 13. (Conditional field validation now handled by extension validation rules
    //     via the ConditionalFieldRequired pattern kind — no hardcoded rules.)

    // 14. Build extension info for schema generation
    let extension_info: Vec<(String, String)> = manifests
        .iter()
        .map(|m| (m.name.clone(), m.version.clone()))
        .collect();

    // 15. Register surface contributions (MCP tools, resources, CLI commands)
    let surface_inputs: Vec<(String, Option<_>)> = manifests
        .iter()
        .map(|m| (m.name.clone(), m.surfaces.clone()))
        .collect();
    let (surface_entries, surface_diags) = register_surface_contributions(&surface_inputs);
    diagnostics.extend(surface_diags);

    // Collect raw manifest surfaces for MCP descriptor generation
    let manifest_surfaces: Vec<(String, SurfaceContributions)> = manifests
        .iter()
        .filter_map(|m| {
            m.surfaces.as_ref().map(|s| (m.name.clone(), s.clone()))
        })
        .collect();

    CompilationContext {
        graph,
        kind_registry: kind_reg,
        field_registry: field_reg,
        edge_registry: edge_reg,
        diagnostics,
        resolved,
        validation_patterns: patterns,
        extension_info,
        surface_entries,
        manifest_surfaces,
        manifests,
        spec_root,
    }
}

/// Lightweight compilation: resolve + build graph + core validation only.
/// No extension manifests, no registry validation, no conditional rules.
/// Use `compile()` for the full pipeline.
pub fn compile_simple(path: &Path) -> CompilationContext {
    let config = load_project_config(path);
    let spec_root = match &config.spec_root {
        Some(sr) => path.join(sr),
        None => path.to_path_buf(),
    };
    let resolved = resolve_project(&spec_root);
    let spec_files: Vec<_> = resolved.files.iter().map(|f| f.spec_file.clone()).collect();
    let (graph, build_diagnostics) = build_graph(&spec_files);
    let validation_diagnostics = validate_with_config(&graph, &ValidatorConfig::default());

    let mut diagnostics = resolved.diagnostics.clone();
    diagnostics.extend(build_diagnostics);
    diagnostics.extend(validation_diagnostics);

    CompilationContext {
        graph,
        kind_registry: KindRegistry::new(),
        field_registry: FieldRegistry::new(),
        edge_registry: EdgeRegistry::new(),
        diagnostics,
        resolved,
        validation_patterns: Vec::new(),
        extension_info: Vec::new(),
        surface_entries: Vec::new(),
        manifest_surfaces: Vec::new(),
        manifests: Vec::new(),
        spec_root,
    }
}

/// Normalize an extension specifier to its canonical `@specforge/` name.
///
/// Config files can reference extensions as paths (`./extensions/product`,
/// `/abs/path/to/extensions/product`) or canonical names (`@specforge/product`).
/// The runtime dispatches by canonical name.
fn normalize_extension_name(ext_spec: &str) -> String {
    if ext_spec.starts_with('@') {
        return ext_spec.to_string();
    }
    let last = std::path::Path::new(ext_spec)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(ext_spec);
    format!("@specforge/{}", last)
}

/// Load extensions via the protocol path only (no manifest.json).
/// Each extension name is resolved through the runtime's __handshake/__describe exports.
fn load_extensions(
    extensions: &[String],
    runtime: &dyn WasmRuntime,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ManifestV2> {
    use specforge_wasm::protocol::{
        protocol_extension_to_manifest, ProtocolHost,
        load_protocol_extension as proto_load,
    };

    let host = ProtocolHost::new(runtime);
    let mut manifests = Vec::new();

    for ext_spec in extensions {
        let ext_name = normalize_extension_name(ext_spec);
        match proto_load(&host, &ext_name) {
            Ok(proto_ext) => {
                let manifest = protocol_extension_to_manifest(&proto_ext);
                let schema_diags = validate_manifest(&manifest);
                let consistency_diags = validate_manifest_consistency(&manifest);
                diagnostics.extend(schema_diags);
                diagnostics.extend(consistency_diags);
                manifests.push(manifest);
            }
            Err(e) => {
                diagnostics.push(Diagnostic {
                    code: "E031".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "extension '{}': protocol loading failed: {}",
                        ext_name, e
                    ),
                    span: None,
                    suggestion: None,
                });
            }
        }
    }

    manifests
}


fn run_extension_validation(
    patterns: &[ValidationRulePattern],
    graph: &Graph,
    edge_label_to_field: &HashMap<String, String>,
) -> Vec<Diagnostic> {
    if patterns.is_empty() {
        return Vec::new();
    }

    let entities: Vec<ValidationEntity> = graph
        .nodes()
        .into_iter()
        .map(|node| {
            let incoming = graph.edges_to(node.id.raw.as_str()).len();
            let outgoing = graph.edges_from(node.id.raw.as_str()).len();

            let mut fields = HashMap::new();
            for entry in node.fields.entries() {
                match &entry.value {
                    specforge_parser::FieldValue::String(s) => {
                        fields.insert(entry.key.to_string(), s.clone());
                    }
                    specforge_parser::FieldValue::Identifier(s) => {
                        fields.insert(entry.key.to_string(), s.clone());
                    }
                    specforge_parser::FieldValue::StringList(list) => {
                        fields.insert(entry.key.to_string(), list.join(", "));
                    }
                    specforge_parser::FieldValue::ReferenceList(refs) => {
                        fields.insert(entry.key.to_string(), refs.join(", "));
                    }
                    specforge_parser::FieldValue::Integer(n) => {
                        fields.insert(entry.key.to_string(), n.to_string());
                    }
                    specforge_parser::FieldValue::Boolean(b) => {
                        fields.insert(entry.key.to_string(), b.to_string());
                    }
                    specforge_parser::FieldValue::Date(d) => {
                        fields.insert(entry.key.to_string(), d.clone());
                    }
                    specforge_parser::FieldValue::VerifyList(stmts) => {
                        if !stmts.is_empty() {
                            let descriptions: Vec<&str> = stmts.iter().map(|s| s.description.as_str()).collect();
                            fields.insert(entry.key.to_string(), descriptions.join("; "));
                        }
                    }
                    specforge_parser::FieldValue::VariantList(variants) => {
                        if !variants.is_empty() {
                            fields.insert(entry.key.to_string(), variants.join(" | "));
                        }
                    }
                    _ => {}
                }
            }

            ValidationEntity {
                id: node.id.raw.to_string(),
                kind: node.kind.raw.to_string(),
                fields,
                incoming_edge_count: incoming,
                outgoing_edge_count: outgoing,
                span: node.source_span.clone(),
            }
        })
        .collect();

    let mut diagnostics = Vec::new();
    for pattern in patterns {
        if pattern.check == specforge_registry::validation_engine::ValidationPatternKind::CycleDetection {
            let diags = detect_cycles(pattern, graph, edge_label_to_field);
            diagnostics.extend(diags);
        } else {
            let diags = execute_pattern(pattern, &entities, None);
            diagnostics.extend(diags);
        }
    }
    diagnostics
}

fn detect_cycles(
    pattern: &ValidationRulePattern,
    graph: &Graph,
    edge_label_to_field: &HashMap<String, String>,
) -> Vec<Diagnostic> {
    let manifest_edge_label = match &pattern.edge_type {
        Some(label) => label.as_str(),
        None => return Vec::new(),
    };
    let edge_label = edge_label_to_field
        .get(manifest_edge_label)
        .map(|s| s.as_str())
        .unwrap_or(manifest_edge_label);
    let target_kind = match &pattern.target_kind {
        Some(kind) => kind.as_str(),
        None => return Vec::new(),
    };

    let nodes: Vec<&specforge_graph::Node> = graph.nodes().into_iter()
        .filter(|n| n.kind.raw == target_kind)
        .collect();

    if nodes.is_empty() {
        return Vec::new();
    }

    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.raw.as_str()).collect();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in graph.edges() {
        if edge.label == edge_label && node_ids.contains(edge.source.as_str()) && node_ids.contains(edge.target.as_str()) {
            adj.entry(edge.source.as_str()).or_default().push(edge.target.as_str());
        }
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Color { White, Gray, Black }

    let mut color: HashMap<&str, Color> = node_ids.iter().map(|id| (*id, Color::White)).collect();
    let mut cycle_members: HashSet<&str> = HashSet::new();

    fn dfs<'a>(
        node: &'a str,
        adj: &HashMap<&'a str, Vec<&'a str>>,
        color: &mut HashMap<&'a str, Color>,
        cycle_members: &mut HashSet<&'a str>,
    ) {
        color.insert(node, Color::Gray);
        if let Some(neighbors) = adj.get(node) {
            for &next in neighbors {
                match color.get(next) {
                    Some(Color::Gray) => {
                        cycle_members.insert(next);
                        cycle_members.insert(node);
                    }
                    Some(Color::White) => {
                        dfs(next, adj, color, cycle_members);
                        if cycle_members.contains(next) {
                            cycle_members.insert(node);
                        }
                    }
                    _ => {}
                }
            }
        }
        color.insert(node, Color::Black);
    }

    for &id in &node_ids {
        if color[id] == Color::White {
            dfs(id, &adj, &mut color, &mut cycle_members);
        }
    }

    let mut diagnostics = Vec::new();
    let mut sorted_members: Vec<&str> = cycle_members.into_iter().collect();
    sorted_members.sort();
    for id in sorted_members {
        if let Some(node) = graph.node(id) {
            let message = specforge_registry::validation_engine::interpolate_template(
                &pattern.message_template,
                id,
                target_kind,
                None,
                None,
            );
            diagnostics.push(Diagnostic {
                code: pattern.code.clone(),
                severity: pattern.severity,
                message,
                span: Some(node.source_span.clone()),
                suggestion: None,
            });
        }
    }

    diagnostics
}

// Conditional field validation (status-dependent rules like I059, W057, I060,
// I066, I069, I070) is now handled by the ConditionalFieldRequired pattern kind
// in the validation engine. The rules are declared by @specforge/product's
// validation_rules() and executed in step 12 alongside all other extension
// validation patterns. No hardcoded domain knowledge remains in the compiler.
