use specforge_common::{Diagnostic, Severity, load_project_config};
use specforge_graph::{Graph, GraphConfig, build_graph, build_graph_with_config};
use specforge_registry::{
    EdgeRegistry, FieldRegistry, KindRegistry, ManifestV2, SurfaceContributions,
    SurfaceRegistryEntry,
    compilation::{
        detect_identifier_length_violations, detect_mistyped_references,
        detect_reserved_entity_ids, detect_unknown_entity_fields, detect_unknown_entity_kinds,
    },
    generate_required_field_rules, populate_registries, register_surface_contributions,
    validate_manifest, validate_manifest_consistency_with_peers,
    validation_engine::{
        ValidationEntity, ValidationRulePattern, execute_pattern, parse_all_rule_patterns,
        resolve_edge_rules,
    },
};
use specforge_resolver::{ResolvedProject, resolve_project};
use specforge_validator::{ValidatorConfig, validate_with_config};
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
    /// The same rules with the extension that owns each (empty for
    /// host-generated ones), for re-running them on a rebuilt graph.
    pub extension_rules: Vec<(ValidationRulePattern, String)>,
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
/// Only extensions listed in `specforge.json` are loaded — no implicit builtins.
///
/// The caller supplies the runtime: CLI/LSP/MCP construct it through
/// `specforge_component::project_runtime` so every surface executes extensions
/// through the same Wasm engine (WASM-only migration, Phase 3).
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
    let (mut patterns, rule_diags) = parse_all_rule_patterns(&rule_inputs);
    diagnostics.extend(rule_diags);
    resolve_edge_rules(&mut patterns, &edge_reg, &kind_reg);

    // 4a. Auto-generated E006 rules for fields marked required: true.
    // Originless (host-generated, declarative — no custom-rule dispatch).
    let required_field_rules = generate_required_field_rules(&field_reg);
    patterns.extend(required_field_rules.into_iter().map(|p| (p, String::new())));

    // 5. Build keyword->extension index for I004 messages.
    //
    // KNOWN GAP: build_graph_with_config only emits I004 for keywords that
    // are NOT installed but ARE present in this map. Deriving the map from
    // the installed manifests makes the two sets identical, so I004 can
    // never fire. The intended source is the registry catalog (every
    // extension the client knows about, installed or not) - that requires
    // an offline catalog cache written by `specforge update`/`search`.
    // Until that cache exists, keep the map in sync with the manifests so
    // the structure is correct once the catalog lands.
    let known_extension_keywords: HashMap<String, String> = manifests
        .iter()
        .flat_map(|m| {
            m.entity_kinds
                .iter()
                .map(move |k| (k.keyword.clone(), m.name.clone()))
        })
        .collect();

    let bidirectional_pairs = field_reg.bidirectional_pairs();

    // 6. Build GraphConfig from registries. Body-parser E001 suppression and
    // single-reference resolution live in build_graph_with_config itself so
    // every consumer (CLI, watch pipeline, LSP) gets identical semantics.
    let body_parser_kinds: HashSet<String> = manifests
        .iter()
        .flat_map(|m| m.entity_kinds.iter())
        .filter(|k| k.has_body_parser)
        .map(|k| k.keyword.clone())
        .collect();
    // 7. Resolve project (use configured spec_root, default to project root)
    let spec_root = match &config.spec_root {
        Some(sr) => path.join(sr),
        None => path.to_path_buf(),
    };
    let resolved = resolve_project(&spec_root);
    diagnostics.extend(resolved.diagnostics.clone());
    let suppressed_parse_error_ranges: Vec<(String, usize, usize)> = resolved
        .files
        .iter()
        .flat_map(|f| f.spec_file.entities.iter())
        .filter(|e| body_parser_kinds.contains(e.kind.raw.as_str()))
        .map(|e| {
            (
                e.span.file.as_str().to_string(),
                e.span.start_line,
                e.span.end_line,
            )
        })
        .collect();
    let single_reference_fields: HashSet<(String, String)> = if kind_reg.is_empty() {
        HashSet::new()
    } else {
        field_reg
            .iter()
            .filter(|(_, _, entry)| {
                entry.field_type == specforge_registry::ManifestFieldType::Reference
            })
            .map(|(kind, field, _)| (kind.to_string(), field.to_string()))
            .collect()
    };
    let graph_config = GraphConfig {
        installed_keywords: kind_reg.keywords().cloned().collect(),
        known_provider_schemes: HashSet::new(),
        known_extension_keywords,
        bidirectional_pairs,
        suppressed_parse_error_ranges,
        single_reference_fields,
        absent_reference_targets: field_reg.absent_reference_targets(&kind_reg),
        field_coercions: crate::field_types::field_coercions(&field_reg),
    };

    // 8. Build graph
    let spec_files: Vec<_> = resolved.files.iter().map(|f| f.spec_file.clone()).collect();
    let (graph, build_diags) = build_graph_with_config(&spec_files, &graph_config);
    diagnostics.extend(build_diags);

    // 9-12. Core validation, registry checks and extension rules.
    diagnostics.extend(check_graph(
        &graph,
        &GraphChecks {
            spec_root: &spec_root,
            kind_registry: &kind_reg,
            field_registry: &field_reg,
            rules: &patterns,
            runtime,
        },
    ));

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
        .filter_map(|m| m.surfaces.as_ref().map(|s| (m.name.clone(), s.clone())))
        .collect();

    CompilationContext {
        graph,
        kind_registry: kind_reg,
        field_registry: field_reg,
        edge_registry: edge_reg,
        diagnostics,
        resolved,
        validation_patterns: patterns.iter().map(|(p, _)| p.clone()).collect(),
        extension_rules: patterns,
        extension_info,
        surface_entries,
        manifest_surfaces,
        manifests,
        spec_root,
    }
}

/// What a project's registries and extension rules need to check a built
/// graph.
pub struct GraphChecks<'a> {
    pub spec_root: &'a Path,
    pub kind_registry: &'a KindRegistry,
    pub field_registry: &'a FieldRegistry,
    /// Validation rules with their owning extension.
    pub rules: &'a [(ValidationRulePattern, String)],
    pub runtime: Option<&'a dyn WasmRuntime>,
}

/// The checks that run on a built graph: core validation, unknown kinds,
/// fields and identifiers, mistyped references (E022), field value types
/// (E061) and the extensions'
/// validation rules. `specforge check` runs them once; watch after every
/// rebuild, so both report the same diagnostics.
pub fn check_graph(graph: &Graph, checks: &GraphChecks) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let spec_root = checks.spec_root.to_path_buf();
    let kind_reg = checks.kind_registry;
    let field_reg = checks.field_registry;
    let patterns = checks.rules;
    let runtime = checks.runtime;

    // Core validation (with file reference fields from registries).
    // BTreeSet: the field list must be ordered, not HashSet-random (R-6 /
    // hardening-plan D2).
    let file_ref_fields: Vec<String> = field_reg
        .iter()
        .filter(|(_, _, entry)| entry.file_reference)
        .map(|(_, field_name, _)| field_name.to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let validator_config = ValidatorConfig {
        spec_root: spec_root.clone(),
        file_reference_fields: file_ref_fields,
    };
    let validation_diags = validate_with_config(graph, &validator_config);
    diagnostics.extend(validation_diags);

    // 10. Run strict field validation against extension registries
    if !kind_reg.is_empty() {
        let entity_kind_info: Vec<_> = graph
            .nodes()
            .iter()
            .map(|n| {
                (
                    n.kind.raw.to_string(),
                    n.id.raw.to_string(),
                    n.source_span.clone(),
                )
            })
            .collect();
        let kind_diags = detect_unknown_entity_kinds(&entity_kind_info, kind_reg, None);
        diagnostics.extend(kind_diags);

        // E013 / E014: the documented identifier contract, now enforced —
        // reserved words and the 2-60 length bound from entity-model.md.
        let reserved_diags = detect_reserved_entity_ids(&entity_kind_info, kind_reg);
        diagnostics.extend(reserved_diags);
        let length_diags = detect_identifier_length_violations(&entity_kind_info);
        diagnostics.extend(length_diags);

        let entity_field_info: Vec<_> = graph
            .nodes()
            .iter()
            .map(|n| {
                let field_names: Vec<String> = n
                    .fields
                    .entries()
                    .iter()
                    .map(|e| e.key.to_string())
                    .collect();
                (
                    n.kind.raw.to_string(),
                    n.id.raw.to_string(),
                    field_names,
                    n.source_span.clone(),
                )
            })
            .collect();
        let field_diags = detect_unknown_entity_fields(&entity_field_info, kind_reg, field_reg);
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
                            Some((
                                e.key.to_string(),
                                refs.iter().map(|r| r.id.clone()).collect(),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                (
                    n.kind.raw.to_string(),
                    n.id.raw.to_string(),
                    ref_fields,
                    n.source_span.clone(),
                )
            })
            .collect();
        let ref_diags =
            detect_mistyped_references(&entity_ref_info, field_reg, kind_reg, &node_kind_index);
        diagnostics.extend(ref_diags);

        // 10b. Values that can't be their field's declared type (E061).
        diagnostics.extend(crate::field_types::check_field_value_types(
            graph, kind_reg, field_reg,
        ));
    }

    // 11. Build edge label mapping (manifest label -> field name used in graph)
    let edge_label_to_field: HashMap<String, String> = field_reg
        .iter()
        .filter_map(|(_, field, entry)| entry.edge.clone().map(|edge| (edge, field.to_string())))
        .collect();

    // 12. Run extension validation rules (declarative + custom via wasm)
    let extension_diags = run_extension_validation(patterns, graph, runtime, &edge_label_to_field);
    diagnostics.extend(extension_diags);

    diagnostics
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
        extension_rules: Vec::new(),
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
pub fn load_extensions(
    extensions: &[String],
    runtime: &dyn WasmRuntime,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ManifestV2> {
    use specforge_wasm::protocol::{
        ProtocolHost, load_protocol_extension as proto_load, protocol_extension_to_manifest,
    };

    let host = ProtocolHost::new(runtime);
    let mut manifests = Vec::new();

    for ext_spec in extensions {
        let ext_name = normalize_extension_name(ext_spec);
        match proto_load(&host, &ext_name) {
            Ok(proto_ext) => {
                let manifest = protocol_extension_to_manifest(&proto_ext);
                diagnostics.extend(validate_manifest(&manifest));
                manifests.push(manifest);
            }
            Err(e) => {
                diagnostics.push(Diagnostic {
                    code: "E028".to_string(),
                    severity: Severity::Error,
                    message: format!("extension '{}': protocol loading failed: {}", ext_name, e),
                    span: None,
                    suggestion: None,
                });
            }
        }
    }

    // Once every extension is in, so a kind is checked against what its
    // peers declare and a non-peer's kind is caught.
    for manifest in &manifests {
        diagnostics.extend(validate_manifest_consistency_with_peers(
            manifest, &manifests,
        ));
    }

    manifests
}

/// Convert all graph nodes into `ValidationEntity` structs for the validation engine.
/// Shared by CLI (`compile.rs`) and LSP (`backend.rs`).
pub fn build_validation_entities(graph: &Graph) -> Vec<ValidationEntity> {
    // Sorted by id: rule diagnostics must emit in a stable order
    // (R-6 / hardening-plan D1 class).
    let mut nodes: Vec<_> = graph.nodes();
    nodes.sort_by_key(|n| n.id.raw);
    nodes
        .into_iter()
        .map(|node| {
            let incoming_edges = graph.edges_to(node.id.raw.as_str());
            let outgoing_edges = graph.edges_from(node.id.raw.as_str());
            let (incoming, outgoing) = (incoming_edges.len(), outgoing_edges.len());
            let by_kind = |ids: &mut dyn Iterator<Item = &str>| {
                let mut counts = std::collections::BTreeMap::new();
                for kind in ids
                    .filter_map(|id| graph.node(id))
                    .map(|n| n.kind.raw.to_string())
                {
                    *counts.entry(kind).or_insert(0) += 1;
                }
                counts
            };
            let incoming_kinds = by_kind(&mut incoming_edges.iter().map(|e| e.source.as_str()));
            let outgoing_kinds = by_kind(&mut outgoing_edges.iter().map(|e| e.target.as_str()));

            let mut fields = HashMap::new();
            let mut verify_kinds: Vec<String> = Vec::new();
            let mut verify_texts: Vec<String> = Vec::new();
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
                        fields.insert(
                            entry.key.to_string(),
                            refs.iter()
                                .map(|r| r.as_str())
                                .collect::<Vec<_>>()
                                .join(", "),
                        );
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
                            let descriptions: Vec<&str> =
                                stmts.iter().map(|s| s.description.as_str()).collect();
                            fields.insert(entry.key.to_string(), descriptions.join("; "));
                            verify_kinds = stmts.iter().map(|s| s.kind.clone()).collect();
                            verify_texts = stmts.iter().map(|s| s.description.clone()).collect();
                        }
                    }
                    specforge_parser::FieldValue::VariantList(variants) if !variants.is_empty() => {
                        fields.insert(entry.key.to_string(), variants.join(" | "));
                    }
                    specforge_parser::FieldValue::Block(block) => {
                        // Contract blocks (requires/ensures/maintains): surface
                        // the clause item names so extension passes can see the
                        // block's presence and contents.
                        let items: Vec<String> =
                            block.entries().iter().map(|e| e.key.to_string()).collect();
                        if !items.is_empty() {
                            fields.insert(entry.key.to_string(), items.join(", "));
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
                verify_kinds,
                verify_texts,
                outgoing_kinds,
                incoming_kinds,
            }
        })
        .collect()
}

/// Wasm dispatch for extensions' `check: "custom"` rules.
///
/// The `wasm_function` names in manifests are contracts: each names an
/// export on THAT extension's module. Per call the host builds a
/// [`ValidatorContext`] snapshot (entity + resolved reference targets +
/// declared type ids + primitive list) and hands it to the guest, which
/// answers with a [`ValidatorVerdict`] — the wasm mirror of the host's
/// `CustomVerdict` (WASM-only migration, Phase 5; closes C10).
pub struct WasmCustomRules<'a> {
    pub runtime: &'a dyn WasmRuntime,
    /// Extension whose module owns the `wasm_function` export.
    pub extension: &'a str,
    pub graph: &'a Graph,
}

/// Type names accepted by E004 without a declared `type` entity. Sent to
/// the guest as `context.primitives`; the guest may also carry its own
/// embedded copy. `number`, `integer`, `boolean` and `timestamp` are the
/// portable primitives docs/entities/type.md documents; `never` marks an
/// impossible error channel (docs/entities/port.md).
const PRIMITIVE_TYPES: &[&str] = &[
    "string",
    "void",
    "bool",
    "i8",
    "i16",
    "i32",
    "i64",
    "u8",
    "u16",
    "u32",
    "u64",
    "f32",
    "f64",
    "usize",
    "isize",
    "any",
    "number",
    "integer",
    "boolean",
    "timestamp",
    "never",
    // stdlib containers: their type arguments are checked recursively
    "Result",
    "Option",
    "Vec",
    "Box",
    "Arc",
    "Rc",
    "HashMap",
    "HashSet",
    "BTreeMap",
    "BTreeSet",
    "String",
];

/// Stringify a field value the way [`build_validation_entities`] does:
/// scalars as strings, reference lists as the declared IDs (comma-joined).
fn stringify_field_value(value: &specforge_parser::FieldValue) -> serde_json::Value {
    use specforge_parser::FieldValue;
    match value {
        FieldValue::String(s) => serde_json::Value::String(s.clone()),
        FieldValue::Identifier(s) => serde_json::Value::String(s.clone()),
        FieldValue::StringList(list) => serde_json::Value::String(list.join(", ")),
        FieldValue::ReferenceList(refs) => serde_json::Value::String(
            refs.iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        ),
        FieldValue::Integer(n) => serde_json::Value::String(n.to_string()),
        FieldValue::Boolean(b) => serde_json::Value::String(b.to_string()),
        FieldValue::Date(d) => serde_json::Value::String(d.clone()),
        FieldValue::VerifyList(stmts) => {
            let descriptions: Vec<&str> = stmts.iter().map(|s| s.description.as_str()).collect();
            serde_json::Value::String(descriptions.join("; "))
        }
        FieldValue::Block(block) => {
            // Contract blocks: surface the clause item names (same as
            // build_validation_entities).
            let items: Vec<String> = block.entries().iter().map(|e| e.key.to_string()).collect();
            serde_json::Value::String(items.join(", "))
        }
        _ => serde_json::Value::Null,
    }
}

impl<'a> WasmCustomRules<'a> {
    /// Build the per-call context snapshot for one entity.
    fn build_context(
        &self,
        entity_id: &str,
    ) -> Result<specforge_protocol_types::ValidatorContext, String> {
        use specforge_protocol_types::{
            ValidatorContext, ValidatorEntity, ValidatorField, ValidatorMethod, ValidatorRef,
        };

        let node = self
            .graph
            .node(entity_id)
            .ok_or_else(|| format!("unknown entity '{entity_id}'"))?;

        let mut referenced: Vec<ValidatorRef> = Vec::new();
        let mut seen_refs: std::collections::HashSet<String> = std::collections::HashSet::new();
        for entry in node.fields.entries() {
            if let specforge_parser::FieldValue::ReferenceList(refs) = &entry.value {
                for r in refs {
                    if seen_refs.insert(r.id.clone()) {
                        referenced.push(ValidatorRef {
                            id: r.id.clone(),
                            kind: self
                                .graph
                                .node(&r.id)
                                .map(|target| target.kind.raw.to_string()),
                        });
                    }
                }
            }
        }

        let declared_types: Vec<String> = self
            .graph
            .nodes()
            .iter()
            .filter(|n| n.kind.raw.as_str() == "type")
            .map(|n| n.id.raw.to_string())
            .collect();

        Ok(ValidatorContext {
            entity: ValidatorEntity {
                id: node.id.raw.to_string(),
                kind: node.kind.raw.to_string(),
                fields: node
                    .fields
                    .entries()
                    .iter()
                    .map(|entry| ValidatorField {
                        key: entry.key.to_string(),
                        value: stringify_field_value(&entry.value),
                        annotations: entry
                            .annotations
                            .iter()
                            .map(|a| a.name.to_string())
                            .collect(),
                    })
                    .collect(),
                methods: node
                    .methods
                    .iter()
                    .map(|m| ValidatorMethod {
                        name: m.name.clone(),
                        params: m
                            .params
                            .iter()
                            .map(|p| specforge_protocol_types::ValidatorParam {
                                name: p.name.clone(),
                                ty: p.ty.clone(),
                            })
                            .collect(),
                        returns: m.returns.clone(),
                    })
                    .collect(),
            },
            referenced,
            declared_types,
            primitives: PRIMITIVE_TYPES.iter().map(|s| s.to_string()).collect(),
        })
    }
}

impl<'a> specforge_registry::validation_engine::WasmValidationRuntime for WasmCustomRules<'a> {
    fn call_custom_validator(
        &self,
        wasm_function: &str,
        entity_id: &str,
        _entity_kind: &str,
    ) -> Result<bool, String> {
        // Detailed verdicts carry the information; the bool form is unused.
        let _ = wasm_function;
        let _ = entity_id;
        Err("use call_custom_validator_detailed".to_string())
    }

    fn call_custom_validator_detailed(
        &self,
        wasm_function: &str,
        entity_id: &str,
        _entity_kind: &str,
    ) -> Result<specforge_registry::validation_engine::CustomVerdict, String> {
        use specforge_registry::validation_engine::CustomVerdict;
        use specforge_wasm::runtime::WasmCallResult;

        if std::env::var("SPECFORGE_DEBUG_RULES").is_ok() {
            eprintln!(
                "DETAILED fn={wasm_function} entity={entity_id} ext={} tier=wasm",
                self.extension
            );
        }

        let context = self.build_context(entity_id)?;
        let input = serde_json::to_vec(&context)
            .map_err(|e| format!("cannot serialize validator context: {e}"))?;

        match self
            .runtime
            .call_export(self.extension, wasm_function, &input)
        {
            WasmCallResult::Ok(output) => {
                let verdict: specforge_protocol_types::ValidatorVerdict =
                    serde_json::from_slice(&output).map_err(|e| {
                        format!(
                            "custom validator '{wasm_function}' returned malformed verdict: {e}"
                        )
                    })?;
                Ok(match verdict {
                    specforge_protocol_types::ValidatorVerdict::Pass => CustomVerdict::Pass,
                    specforge_protocol_types::ValidatorVerdict::Fail { field, value } => {
                        CustomVerdict::Fail { field, value }
                    }
                })
            }
            WasmCallResult::Trap(trap) => Err(format!(
                "custom validator '{}' did not execute: {} — {}",
                wasm_function, trap.kind, trap.message
            )),
        }
    }
}

fn run_extension_validation(
    patterns: &[(ValidationRulePattern, String)],
    graph: &Graph,
    runtime: Option<&dyn WasmRuntime>,
    edge_label_to_field: &HashMap<String, String>,
) -> Vec<Diagnostic> {
    if patterns.is_empty() {
        return Vec::new();
    }

    let entities = build_validation_entities(graph);

    if std::env::var("SPECFORGE_DEBUG_RULES").is_ok() {
        for (p, ext) in patterns {
            eprintln!(
                "RULE {} ext={} check={:?} target={:?} values={:?}",
                p.code,
                ext,
                p.check,
                p.target_kind,
                p.constraint.as_ref().map(|c| c.values.clone())
            );
        }
    }
    let mut diagnostics: Vec<specforge_common::Diagnostic> = Vec::new();
    for (pattern, extension) in patterns {
        if pattern.check
            == specforge_registry::validation_engine::ValidationPatternKind::CycleDetection
        {
            let diags = detect_cycles(pattern, graph, edge_label_to_field);
            diagnostics.extend(diags);
        } else {
            // Declarative rules evaluate host-side; custom ones need the
            // extension's module, so they're skipped without a runtime.
            let verdicts = runtime.map(|runtime| WasmCustomRules {
                runtime,
                extension,
                graph,
            });
            let diags = execute_pattern(
                pattern,
                &entities,
                verdicts.as_ref().map(|v| {
                    v as &dyn specforge_registry::validation_engine::WasmValidationRuntime
                }),
            );
            diagnostics.extend(diags);
        }
    }
    diagnostics
}

/// A `cycle_detection` rule over the graph: each cycle among `target_kind`
/// entities along the rule's edge type, reported with the rule's code and
/// severity. `edge_label_to_field` maps a manifest edge type to the field
/// whose references form those edges.
pub fn detect_cycles(
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

    // Deterministic node order: seed and traversal order must not depend on
    // HashMap iteration (per-process RandomState) — R-6.
    let mut nodes: Vec<&specforge_graph::Node> = graph
        .nodes()
        .into_iter()
        .filter(|n| n.kind.raw == target_kind)
        .collect();
    nodes.sort_by_key(|n| n.id.raw);

    if nodes.is_empty() {
        return Vec::new();
    }

    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.raw.as_str()).collect();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in graph.edges() {
        if edge.label == edge_label
            && node_ids.contains(edge.source.as_str())
            && node_ids.contains(edge.target.as_str())
        {
            adj.entry(edge.source.as_str())
                .or_default()
                .push(edge.target.as_str());
        }
    }
    for neighbors in adj.values_mut() {
        neighbors.sort_unstable();
    }

    // Cycle membership via the shared exact-membership walker (C5-00):
    // one 3-color DFS with path-stack semantics for every entity-level
    // detector (graph.rs, this pass).
    let btree_adj: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> = adj
        .iter()
        .map(|(k, v)| {
            (
                (*k).to_string(),
                v.iter().map(|s| (*s).to_string()).collect(),
            )
        })
        .collect();
    let seeds: Vec<String> = nodes.iter().map(|n| n.id.raw.to_string()).collect();
    let (cycle_members_set, _) =
        specforge_graph::find_cycles(&seeds, &btree_adj, specforge_graph::CycleOptions::default());
    let cycle_members: HashSet<&str> = cycle_members_set.iter().map(|s| s.as_str()).collect();

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
