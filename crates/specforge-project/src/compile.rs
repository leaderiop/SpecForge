//! The checks a built graph goes through, and loading the extensions they
//! come from: core validation, the registry checks, the extensions'
//! declarative rules and their Wasm `check: "custom"` rules.

use crate::snapshot::EntitySnapshot;
use specforge_common::{Diagnostic, ExtensionEntry, Severity};
use specforge_graph::{Graph, GraphConfig};
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::{
    RegistryBuild,
    compilation::{
        detect_identifier_length_violations, detect_mistyped_references,
        detect_reserved_entity_ids, detect_unknown_entity_fields, detect_unknown_entity_kinds,
    },
    validation_engine::{ValidationRulePattern, execute_pattern},
};
use specforge_validator::{ValidatorConfig, validate_with_config};
use specforge_wasm::WasmRuntime;
use std::collections::{HashMap, HashSet};
use std::path::Path;

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
        derived_references: crate::field_types::derived_references(&build.fields),
    }
}

/// What the checks on a built graph read: the project's registry build,
/// the graph's entity snapshot (ADR 0019) and the runtime its custom rules
/// call.
pub struct GraphChecks<'a> {
    /// Where core validation's file references resolve.
    pub spec_root: &'a Path,
    /// Kinds and fields (the registry checks, E061, core validation's file
    /// references) and the extensions' rules with their owners.
    pub registries: &'a RegistryBuild,
    /// The graph's entities as every check after the build reads them.
    pub entities: &'a EntitySnapshot,
    pub runtime: Option<&'a dyn WasmRuntime>,
}

/// The checks that run on a built graph: core validation, unknown kinds,
/// fields and identifiers, mistyped references (E022), field value types
/// (E061) and the extensions'
/// validation rules. `specforge check` runs them once; watch after every
/// rebuild, so both report the same diagnostics. The registry checks and
/// the rules read `checks.entities`, the snapshot of `graph`.
pub fn check_graph(graph: &Graph, checks: &GraphChecks) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let kind_reg = &checks.registries.kinds;
    let field_reg = &checks.registries.fields;

    // Core validation (with file reference fields from registries).
    // BTreeSet: the field list must be ordered, not HashSet-random (R-6 /
    // hardening-plan D2).
    let file_ref_fields: Vec<String> = field_reg
        .iter()
        .filter(|(_, _, entry)| entry.declared.file_reference)
        .map(|(_, field_name, _)| field_name.to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let validator_config = ValidatorConfig {
        spec_root: checks.spec_root.to_path_buf(),
        file_reference_fields: file_ref_fields,
    };
    let validation_diags = validate_with_config(graph, &validator_config);
    diagnostics.extend(validation_diags);

    // Unknown kinds, identifiers and fields, against the registries.
    if !kind_reg.is_empty() {
        let records = checks.entities.records();
        diagnostics.extend(detect_unknown_entity_kinds(records, kind_reg, None));

        // E013 / E014: the documented identifier contract, now enforced —
        // reserved words and the 2-60 length bound from entity-model.md.
        diagnostics.extend(detect_reserved_entity_ids(records, kind_reg));
        diagnostics.extend(detect_identifier_length_violations(records));

        diagnostics.extend(detect_unknown_entity_fields(records, kind_reg, field_reg));

        // Reference fields against their target_kind constraints (E022).
        diagnostics.extend(detect_mistyped_references(records, field_reg, kind_reg));

        // Values that can't be their field's declared type (E061).
        diagnostics.extend(crate::field_types::check_field_value_types(
            graph, kind_reg, field_reg,
        ));
    }

    // Edge label mapping (manifest label -> field name used in graph).
    let edge_label_to_field: HashMap<String, String> = field_reg
        .iter()
        .filter_map(|(_, field, entry)| {
            entry
                .declared
                .edge
                .clone()
                .map(|edge| (edge, field.to_string()))
        })
        .collect();

    // Extension validation rules (declarative + custom via wasm).
    let extension_diags = run_extension_validation(
        &checks.registries.rules,
        graph,
        checks.entities,
        checks.runtime,
        &edge_label_to_field,
    );
    diagnostics.extend(extension_diags);

    diagnostics
}

/// What one `specforge.json` `extensions` entry enables, as the runtime
/// loaded it: the entry read by [`ExtensionEntry`], the rule the runtime
/// (`specforge_component::project_runtime`) loads it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnabledExtension {
    /// The entry as `specforge.json` writes it (trimmed).
    pub entry: String,
    /// The extension's name: a named entry's; for a `.wasm` file entry the
    /// name its component declares once it loaded, else the name written
    /// before `=`, else the path.
    pub name: String,
    /// The path a `.wasm` file entry names, as written.
    pub file: Option<String>,
}

impl EnabledExtension {
    /// What `entry` enables, as `runtime` (if any) loaded it.
    pub fn of(entry: &str, runtime: Option<&dyn WasmRuntime>) -> Self {
        match ExtensionEntry::parse(entry) {
            ExtensionEntry::Named(name) => EnabledExtension {
                entry: entry.trim().to_string(),
                name: name.to_string(),
                file: None,
            },
            ExtensionEntry::File { name, path } => EnabledExtension {
                entry: entry.trim().to_string(),
                name: runtime
                    .and_then(|runtime| runtime.file_entry_extension(entry.trim()))
                    .or(name.map(str::to_string))
                    .unwrap_or_else(|| path.to_string()),
                file: Some(path.to_string()),
            },
        }
    }
}

/// Load the declarations of `extensions` (as `specforge.json` lists them)
/// through `runtime`, in that order: one [`load_declaration`] per
/// extension, each entry naming the extension [`EnabledExtension::of`]
/// says (an extension two entries enable is read once). An extension that
/// does not load is E028 (or the runtime's own reason, E028/E033, when it
/// knows one) and is left out. `diagnostics` receives those runtime
/// failures in load order, then the W138s of the declarations that loaded.
/// What the declarations themselves are worth (E030, W021, E027, W145) is
/// the registry build's to say.
///
/// [`load_declaration`]: specforge_wasm::protocol::load_declaration
pub fn load_extensions(
    extensions: &[String],
    runtime: &dyn WasmRuntime,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ExtensionDeclaration> {
    use specforge_wasm::protocol::load_declaration;

    let mut declarations = Vec::new();
    let mut warnings = Vec::new();
    let mut read = HashSet::new();
    for entry in extensions {
        // A `.wasm` file's failure is known by the entry (what it would
        // have declared is not), and is reported whatever the entry names.
        let (failure_key, file) = match ExtensionEntry::parse(entry) {
            ExtensionEntry::Named(name) => (name, false),
            ExtensionEntry::File { .. } => (entry.trim(), true),
        };
        if file && let Some(failure) = runtime.load_failure(failure_key) {
            diagnostics.push(failure);
            continue;
        }
        let ext_name = EnabledExtension::of(entry, Some(runtime)).name;
        if !read.insert(ext_name.clone()) {
            continue;
        }
        match load_declaration(runtime, &ext_name) {
            Ok(loaded) => {
                warnings.extend(loaded.warnings);
                declarations.push(loaded.declaration);
            }
            // Why the runtime could not load it (a missing or tampered
            // installed binary), when it knows.
            Err(_) if let Some(failure) = runtime.load_failure(failure_key) => {
                diagnostics.push(failure);
            }
            Err(e) => {
                diagnostics.push(Diagnostic {
                    code: "E028".to_string(),
                    severity: Severity::Error,
                    message: format!("extension '{}': protocol loading failed: {}", ext_name, e),
                    span: None,
                    suggestion: None,
                    data: None,
                });
            }
        }
    }
    diagnostics.extend(warnings);
    declarations
}

/// Wasm dispatch for extensions' `check: "custom"` rules.
///
/// The `wasm_function` names in manifests are contracts: each names an
/// export on THAT extension's module. Per call the host hands the guest the
/// entity's [`ValidatorContext`] from the snapshot
/// ([`EntitySnapshot::validator_context`]: the entity, its resolved
/// references, the declared type ids and the primitive list), and the guest
/// answers with a [`ValidatorVerdict`] — the wasm mirror of the host's
/// `CustomVerdict` (WASM-only migration, Phase 5; closes C10).
///
/// [`ValidatorContext`]: specforge_protocol_types::ValidatorContext
/// [`ValidatorVerdict`]: specforge_protocol_types::ValidatorVerdict
pub struct WasmCustomRules<'a> {
    pub runtime: &'a dyn WasmRuntime,
    /// Extension whose module owns the `wasm_function` export.
    pub extension: &'a str,
    /// The graph's entities, as the guest receives each one.
    pub entities: &'a EntitySnapshot,
}

impl<'a> specforge_registry::validation_engine::WasmValidationRuntime for WasmCustomRules<'a> {
    fn custom_verdict(
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

        let context = self
            .entities
            .validator_context(entity_id)
            .ok_or_else(|| format!("unknown entity '{entity_id}'"))?;
        call_validator(self.runtime, self.extension, wasm_function, &context)
            .map_err(|error| error.to_string())
    }
}

/// Call `extension`'s `wasm_function` on `context` and read its verdict
/// (the protocol's `ValidatorVerdict`). Err: the call failed (E028).
fn call_validator(
    runtime: &dyn WasmRuntime,
    extension: &str,
    wasm_function: &str,
    context: &specforge_protocol_types::ValidatorContext,
) -> Result<specforge_registry::validation_engine::CustomVerdict, specforge_wasm::CallError> {
    use specforge_protocol_types::ValidatorVerdict;
    use specforge_registry::validation_engine::CustomVerdict;

    let verdict =
        specforge_wasm::ExtensionCalls::new(runtime).validate(extension, wasm_function, context)?;
    Ok(match verdict {
        ValidatorVerdict::Pass => CustomVerdict::Pass,
        ValidatorVerdict::Fail { field, value } => CustomVerdict::Fail { field, value },
    })
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
        let context =
            EntitySnapshot::probe_context(pattern.target_kind.as_deref().unwrap_or_default());
        if let Err(error) = call_validator(runtime, extension, wasm_function, &context) {
            diagnostics.push(Diagnostic {
                code: "W112".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "extension '{extension}': rule '{}': wasm_function '{wasm_function}' could not be resolved ({error}) — the rule will not fire",
                    pattern.code
                ),
                span: None,
                suggestion: Some(format!(
                    "export '{wasm_function}' from '{extension}', or fix the rule's wasm_function"
                )),
                data: None,
            });
        }
    }
    diagnostics
}

fn run_extension_validation(
    patterns: &[(ValidationRulePattern, String)],
    graph: &Graph,
    entities: &EntitySnapshot,
    runtime: Option<&dyn WasmRuntime>,
    edge_label_to_field: &HashMap<String, String>,
) -> Vec<Diagnostic> {
    if patterns.is_empty() {
        return Vec::new();
    }

    let input = entities.rule_input();

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
                entities,
            });
            let diags = execute_pattern(
                pattern,
                &input,
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
                data: None,
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
