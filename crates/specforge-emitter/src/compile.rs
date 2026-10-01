use specforge_common::{Diagnostic, Severity, load_project_config};
use specforge_graph::{Graph, GraphConfig, build_graph};
use specforge_registry::{
    EdgeRegistry, FieldRegistry, KindRegistry, ManifestV2, RegistryBuild, SurfaceContributions,
    SurfaceRegistryEntry,
    compilation::{
        EntityView, detect_identifier_length_violations, detect_mistyped_references,
        detect_reserved_entity_ids, detect_unknown_entity_fields, detect_unknown_entity_kinds,
    },
    validate_manifest, validate_manifest_consistency_with_peers, validate_peer_dependencies,
    validation_engine::{ValidationEntity, ValidationRulePattern, execute_pattern},
};
use specforge_resolver::{ResolvedProject, resolve_project};
use specforge_validator::{ValidatorConfig, validate_with_config};
use specforge_wasm::WasmRuntime;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// The flat view of a compiled project that older callers read
/// (`specforge_project::CompiledProject::into_context` builds it; the
/// compile itself lives in `specforge-project`).
pub struct CompilationContext {
    pub graph: Graph,
    pub kind_registry: KindRegistry,
    pub field_registry: FieldRegistry,
    pub edge_registry: EdgeRegistry,
    /// What `specforge check` reports, in its order.
    pub diagnostics: Vec<Diagnostic>,
    pub resolved: ResolvedProject,
    /// The validation rules with the extension that owns each (empty for
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

/// The graph build's inputs, from a registry build. Every surface that
/// builds a graph (`check`, watch, the LSP) takes its `GraphConfig` from
/// here, so none can drift.
pub fn graph_config(build: &RegistryBuild) -> GraphConfig {
    GraphConfig {
        known_provider_schemes: HashSet::new(),
        bidirectional_pairs: build.bidirectional_pairs.clone(),
        body_parser_kinds: build.body_parser_kinds.clone(),
        single_reference_fields: build.single_reference_fields.clone(),
        absent_reference_targets: build.absent_reference_targets.clone(),
        field_coercions: crate::field_types::field_coercions(&build.fields),
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

    // Unknown kinds, identifiers and fields, against the registries.
    if !kind_reg.is_empty() {
        let views = entity_views(graph);
        diagnostics.extend(detect_unknown_entity_kinds(&views, kind_reg, None));

        // E013 / E014: the documented identifier contract, now enforced —
        // reserved words and the 2-60 length bound from entity-model.md.
        diagnostics.extend(detect_reserved_entity_ids(&views, kind_reg));
        diagnostics.extend(detect_identifier_length_violations(&views));

        diagnostics.extend(detect_unknown_entity_fields(&views, kind_reg, field_reg));

        // Reference fields against their target_kind constraints (E022).
        diagnostics.extend(detect_mistyped_references(&views, field_reg, kind_reg));

        // Values that can't be their field's declared type (E061).
        diagnostics.extend(crate::field_types::check_field_value_types(
            graph, kind_reg, field_reg,
        ));
    }

    // Edge label mapping (manifest label -> field name used in graph).
    let edge_label_to_field: HashMap<String, String> = field_reg
        .iter()
        .filter_map(|(_, field, entry)| entry.edge.clone().map(|edge| (edge, field.to_string())))
        .collect();

    // Extension validation rules (declarative + custom via wasm).
    let extension_diags =
        run_extension_validation(patterns, graph, field_reg, runtime, &edge_label_to_field);
    diagnostics.extend(extension_diags);

    diagnostics
}

/// Every node of the graph as the registry checks see it.
fn entity_views(graph: &Graph) -> Vec<EntityView<'_>> {
    graph
        .nodes()
        .iter()
        .map(|n| EntityView {
            kind: n.kind.raw.as_str(),
            id: n.id.raw.as_str(),
            span: &n.source_span,
            fields: n.fields.entries().iter().map(|e| e.key.as_str()).collect(),
            references: n
                .fields
                .entries()
                .iter()
                .filter_map(|e| match &e.value {
                    specforge_parser::FieldValue::ReferenceList(refs) => {
                        Some((e.key.as_str(), refs.iter().map(|r| r.id.as_str()).collect()))
                    }
                    _ => None,
                })
                .collect(),
        })
        .collect()
}

/// Lightweight compilation: resolve + build graph + core validation only.
/// No extension manifests, no registry validation, no conditional rules.
/// `specforge_project::CompiledProject::compile` is the full pipeline.
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
        // `@scope/name`, or `@scope/name@version` as older `add`s wrote
        // it: the runtime loads it under its name.
        return specforge_common::extension_entry_name(ext_spec).to_string();
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
            // Why the runtime could not load it (a missing or tampered
            // installed binary), when it knows.
            Err(_) if let Some(failure) = runtime.load_failure(&ext_name) => {
                diagnostics.push(failure);
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
    // Every required peer is loaded, and every loaded peer is in range
    // (E027). The extension still registers: an error here fails the
    // check without turning each of its entities into an E024.
    diagnostics.extend(validate_peer_dependencies(&manifests));

    manifests
}

/// Convert all graph nodes into `ValidationEntity` structs for the validation engine.
/// Shared by CLI (`compile.rs`) and LSP (`backend.rs`).
///
/// `field_registry` decides which entities owe no obligations
/// ([`ValidationEntity::obligation_exempt`], see
/// [`crate::coverage::obligation_exempt`]).
pub fn build_validation_entities(
    graph: &Graph,
    field_registry: &FieldRegistry,
) -> Vec<ValidationEntity> {
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
                obligation_exempt: crate::coverage::obligation_exempt(node, field_registry),
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
        if std::env::var("SPECFORGE_DEBUG_RULES").is_ok() {
            eprintln!(
                "DETAILED fn={wasm_function} entity={entity_id} ext={} tier=wasm",
                self.extension
            );
        }

        let context = self.build_context(entity_id)?;
        call_validator(self.runtime, self.extension, wasm_function, &context)
    }
}

/// Call `extension`'s `wasm_function` export with `context` and read its
/// verdict.
fn call_validator(
    runtime: &dyn WasmRuntime,
    extension: &str,
    wasm_function: &str,
    context: &specforge_protocol_types::ValidatorContext,
) -> Result<specforge_registry::validation_engine::CustomVerdict, String> {
    use specforge_registry::validation_engine::CustomVerdict;
    use specforge_wasm::runtime::WasmCallResult;

    let input = serde_json::to_vec(context)
        .map_err(|e| format!("cannot serialize validator context: {e}"))?;
    match runtime.call_export(extension, wasm_function, &input) {
        WasmCallResult::Ok(output) => {
            let verdict: specforge_protocol_types::ValidatorVerdict =
                serde_json::from_slice(&output).map_err(|e| {
                    format!("custom validator '{wasm_function}' returned malformed verdict: {e}")
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

/// Resolve each `check: "custom"` rule's `wasm_function` against the
/// extension that declared it, when the rules are registered: one call with
/// an entity of the rule's target kind that declares nothing. A name the
/// extension does not export, or an export that does not answer with a
/// verdict, is W112 here, once, instead of a rule that silently never fires
/// (dispatch skips an entity whose call fails). The rule stays registered.
pub fn probe_custom_rules(
    rules: &[(ValidationRulePattern, String)],
    runtime: &dyn WasmRuntime,
) -> Vec<Diagnostic> {
    use specforge_registry::validation_engine::ValidationPatternKind;

    let mut diagnostics = Vec::new();
    for (pattern, extension) in rules {
        if pattern.check != ValidationPatternKind::Custom {
            continue;
        }
        // parse_rule_pattern rejects a custom rule without one.
        let Some(wasm_function) = pattern.wasm_function.as_deref() else {
            continue;
        };
        let context = specforge_protocol_types::ValidatorContext {
            entity: specforge_protocol_types::ValidatorEntity {
                id: "__probe__".to_string(),
                kind: pattern.target_kind.clone().unwrap_or_default(),
                fields: Vec::new(),
                methods: Vec::new(),
            },
            referenced: Vec::new(),
            declared_types: Vec::new(),
            primitives: PRIMITIVE_TYPES.iter().map(|s| s.to_string()).collect(),
        };
        if let Err(reason) = call_validator(runtime, extension, wasm_function, &context) {
            diagnostics.push(Diagnostic {
                code: "W112".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "extension '{extension}': rule '{}': wasm_function '{wasm_function}' could not be resolved ({reason}) — the rule will not fire",
                    pattern.code
                ),
                span: None,
                suggestion: Some(format!(
                    "export '{wasm_function}' from '{extension}', or fix the rule's wasm_function"
                )),
            });
        }
    }
    diagnostics
}

fn run_extension_validation(
    patterns: &[(ValidationRulePattern, String)],
    graph: &Graph,
    fields: &FieldRegistry,
    runtime: Option<&dyn WasmRuntime>,
    edge_label_to_field: &HashMap<String, String>,
) -> Vec<Diagnostic> {
    if patterns.is_empty() {
        return Vec::new();
    }

    let entities = build_validation_entities(graph, fields);

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
// validation_rules() and executed by check_graph alongside all other
// extension validation patterns. No hardcoded domain knowledge remains in the compiler.
