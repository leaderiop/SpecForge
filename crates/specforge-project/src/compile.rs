//! The checks a built graph goes through, and loading the extensions they
//! come from: core validation, the registry checks and the rule set (the
//! extensions' declarative and Wasm `check: "custom"` rules, ADR 0020).

use crate::snapshot::EntitySnapshot;
use crate::verdicts::WasmVerdicts;
use specforge_common::Diagnostic;
use specforge_graph::{Graph, GraphConfig};
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

    // Core validation (with file reference fields from registries), in
    // field-name order.
    let file_ref_fields: Vec<String> = field_reg
        .file_reference_fields()
        .into_iter()
        .map(str::to_string)
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
