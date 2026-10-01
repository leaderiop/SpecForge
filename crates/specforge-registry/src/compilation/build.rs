//! One registry build: loaded manifests in, everything the compiler derives
//! from them out (architecture plan 05, step R1).
//!
//! Callers used to run the steps themselves (populate, parse the rules,
//! scope the edge rules, generate the E006 rules, register the surfaces)
//! and derive the graph's inputs from the result, each in its own copy.
//! [`build_registries`] owns that order; callers read [`RegistryBuild`].

use std::collections::{HashMap, HashSet};

use specforge_common::Diagnostic;

use super::detection::generate_required_field_rules;
use super::populate::populate_registries;
use super::validate::register_validation_rules;
use super::validation_engine::{
    ValidationRulePattern, parse_all_rule_patterns, resolve_edge_rules,
};
use crate::{
    EdgeRegistry, FieldRegistry, KindRegistry, ManifestFieldType, ManifestV2, SurfaceContributions,
    SurfaceRegistryEntry, register_surface_contributions,
};

/// Everything the compiler derives from the loaded manifests, before any
/// `.spec` file is read.
#[derive(Debug, Default)]
pub struct RegistryBuild {
    /// The manifests the build was made from, in load order.
    pub manifests: Vec<ManifestV2>,
    pub kinds: KindRegistry,
    pub fields: FieldRegistry,
    pub edges: EdgeRegistry,
    /// The extensions' rules (parsed and scoped to their edge types) plus
    /// the host-generated E006 rules for required fields, each with the
    /// extension that owns it (empty for host-generated ones) for custom
    /// rule dispatch.
    pub rules: Vec<(ValidationRulePattern, String)>,
    /// Kinds whose bodies an extension parses: the core grammar's parse
    /// errors inside them are not reported.
    pub body_parser_kinds: HashSet<String>,
    /// (kind, field) pairs registered as single references (empty when no
    /// kind is registered).
    pub single_reference_fields: HashSet<(String, String)>,
    /// Inverse field pairs that are not reference cycles.
    pub bidirectional_pairs: Vec<(String, String)>,
    /// (kind, field) reference fields whose target kind no loaded extension
    /// declares, mapped to that kind.
    pub absent_reference_targets: HashMap<(String, String), String>,
    /// Registered surface contributions (first registration wins).
    pub surfaces: Vec<SurfaceRegistryEntry>,
    /// Each manifest's raw surface contributions, for MCP descriptors.
    pub manifest_surfaces: Vec<(String, SurfaceContributions)>,
    /// (name, version) of each loaded extension.
    pub extension_info: Vec<(String, String)>,
    /// Populate (E026, W018, W019, I004), rule-parse (W112), then
    /// duplicate rule codes (W023)
    /// diagnostics, in that order. `specforge_project::Environment::load`
    /// appends the custom rules' probes (W112) when the extensions ran in a
    /// runtime.
    pub registry_diagnostics: Vec<Diagnostic>,
    /// Surface registration conflicts (E039). `specforge check` reports
    /// them after the graph's own diagnostics.
    pub surface_diagnostics: Vec<Diagnostic>,
}

/// Build every registry and derived input from the loaded manifests, which
/// come in load order (dependencies first).
pub fn build_registries(manifests: Vec<ManifestV2>) -> RegistryBuild {
    let (kinds, fields, edges, mut registry_diagnostics) = populate_registries(&manifests);

    let rule_inputs: Vec<(String, Vec<_>)> = manifests
        .iter()
        .map(|m| (m.name.clone(), m.validation_rules.clone()))
        .collect();
    let (mut rules, rule_diagnostics) = parse_all_rule_patterns(&rule_inputs);
    registry_diagnostics.extend(rule_diagnostics);
    // W023: two extensions declaring the same rule code.
    registry_diagnostics.extend(register_validation_rules(&manifests).1);
    resolve_edge_rules(&mut rules, &edges, &kinds);
    // Required fields (`required: true`) get host-generated, declarative
    // E006 rules: originless, so never dispatched to an extension.
    rules.extend(
        generate_required_field_rules(&fields)
            .into_iter()
            .map(|p| (p, String::new())),
    );

    let body_parser_kinds: HashSet<String> = manifests
        .iter()
        .flat_map(|m| m.entity_kinds.iter())
        .filter(|k| k.has_body_parser)
        .map(|k| k.keyword.clone())
        .collect();
    let single_reference_fields: HashSet<(String, String)> = if kinds.is_empty() {
        HashSet::new()
    } else {
        fields
            .iter()
            .filter(|(_, _, entry)| entry.field_type == ManifestFieldType::Reference)
            .map(|(kind, field, _)| (kind.to_string(), field.to_string()))
            .collect()
    };
    let bidirectional_pairs = fields.bidirectional_pairs();
    let absent_reference_targets = fields.absent_reference_targets(&kinds);

    let extension_info: Vec<(String, String)> = manifests
        .iter()
        .map(|m| (m.name.clone(), m.version.clone()))
        .collect();
    let surface_inputs: Vec<(String, Option<SurfaceContributions>)> = manifests
        .iter()
        .map(|m| (m.name.clone(), m.surfaces.clone()))
        .collect();
    let (surfaces, surface_diagnostics) = register_surface_contributions(&surface_inputs);
    let manifest_surfaces: Vec<(String, SurfaceContributions)> = manifests
        .iter()
        .filter_map(|m| m.surfaces.as_ref().map(|s| (m.name.clone(), s.clone())))
        .collect();

    RegistryBuild {
        manifests,
        kinds,
        fields,
        edges,
        rules,
        body_parser_kinds,
        single_reference_fields,
        bidirectional_pairs,
        absent_reference_targets,
        surfaces,
        manifest_surfaces,
        extension_info,
        registry_diagnostics,
        surface_diagnostics,
    }
}
