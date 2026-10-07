//! The checks a built graph goes through, and loading the extensions they
//! come from: core validation, the registry checks and the rule set (the
//! extensions' declarative and Wasm `check: "custom"` rules, ADR 0020).

use crate::snapshot::EntitySnapshot;
use crate::verdicts::WasmVerdicts;
use specforge_common::{Diagnostic, ExtensionEntry, codes};
use specforge_graph::{Graph, GraphConfig};
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::{
    RegistryBuild,
    compilation::{
        detect_identifier_length_violations, detect_mistyped_references,
        detect_reserved_entity_ids, detect_unknown_entity_fields, detect_unknown_entity_kinds,
    },
    rules::{CustomVerdicts, NoVerdicts},
};
use specforge_validator::{ValidatorConfig, validate_with_config};
use specforge_wasm::WasmRuntime;
use std::collections::HashSet;
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

    // The rule set (declarative, cycles and custom via the extensions'
    // modules); without a runtime custom rules are skipped.
    let verdicts: Box<dyn CustomVerdicts + '_> = match checks.runtime {
        Some(runtime) => Box::new(WasmVerdicts::new(runtime, checks.entities)),
        None => Box::new(NoVerdicts),
    };
    diagnostics.extend(
        checks
            .registries
            .rules
            .check(&checks.entities.rule_input(), verdicts.as_ref()),
    );

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
/// failures in load order, then the load warnings (W153, W138) of the
/// declarations that loaded. What the declarations themselves are worth
/// (E030, W021, E027, W145) is the registry build's to say.
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
                diagnostics.push(Diagnostic::new(
                    codes::E028,
                    format!("extension '{}': protocol loading failed: {}", ext_name, e),
                ));
            }
        }
    }
    diagnostics.extend(warnings);
    declarations
}
