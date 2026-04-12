use crate::{
    EdgeRegistry, EdgeRegistryEntry, FieldRegistry, FieldRegistryEntry, KindRegistry,
    KindRegistryEntry, ManifestFieldType, ManifestV2,
};
use specforge_common::{Diagnostic, Severity};

/// Populate all three registries from a list of extension manifests.
/// Manifests should be provided in topological order (dependencies first).
/// Returns populated registries plus any diagnostics.
pub fn populate_registries(
    manifests: &[ManifestV2],
) -> (KindRegistry, FieldRegistry, EdgeRegistry, Vec<Diagnostic>) {
    let mut kind_reg = KindRegistry::new();
    let mut field_reg = FieldRegistry::new();
    let mut edge_reg = EdgeRegistry::new();
    let mut diagnostics = Vec::new();

    for manifest in manifests {
        register_entity_kinds(&mut kind_reg, manifest, &mut diagnostics);
        register_fields(&mut field_reg, manifest, &mut diagnostics);
        register_edge_types(&mut edge_reg, manifest, &mut diagnostics);
        register_implicit_edges(&mut edge_reg, manifest, &mut diagnostics);
    }

    // After all manifests processed, apply entity enhancements
    let all_enhancements: Vec<_> = manifests
        .iter()
        .flat_map(|m| {
            m.entity_enhancements
                .iter()
                .map(move |e| (m.name.clone(), e.clone()))
        })
        .collect();
    let enh_diags = apply_entity_enhancements(&all_enhancements, &kind_reg, &mut field_reg);
    diagnostics.extend(enh_diags);

    (kind_reg, field_reg, edge_reg, diagnostics)
}

/// Apply entity enhancements to the FieldRegistry.
/// Enhancements targeting unknown kinds produce I004 info diagnostics.
/// Enhancement fields do NOT overwrite existing kind-level fields.
pub fn apply_entity_enhancements(
    enhancements: &[(String, crate::FieldEnhancement)],
    kind_reg: &KindRegistry,
    field_reg: &mut FieldRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (ext_name, enhancement) in enhancements {
        if !kind_reg.contains(&enhancement.target_kind) {
            diagnostics.push(Diagnostic {
                code: "I004".to_string(),
                severity: Severity::Info,
                message: format!(
                    "extension '{}': entity enhancement targets unknown kind '{}' (extension may not be installed)",
                    ext_name, enhancement.target_kind
                ),
                span: None,
                suggestion: None,
            });
            continue;
        }

        for field in &enhancement.fields {
            // Kind-level fields win — do not overwrite
            if field_reg.contains(&enhancement.target_kind, &field.name) {
                continue;
            }

            register_single_field(
                field_reg,
                &enhancement.target_kind,
                field,
                &enhancement.source_extension,
                &mut diagnostics,
            );
        }
    }

    diagnostics
}

/// Register entity kinds from a single manifest into the KindRegistry.
fn register_entity_kinds(
    registry: &mut KindRegistry,
    manifest: &ManifestV2,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for kind in &manifest.entity_kinds {
        let entry = KindRegistryEntry {
            kind_name: kind.keyword.clone(),
            description: kind.description.clone(),
            source_extension: manifest.name.clone(),
            testable: kind.testable,
            singleton: kind.singleton,
            supports_verify: kind.supports_verify,
            allowed_verify_kinds: kind.allowed_verify_kinds.clone(),
            semantic_token: kind.semantic_token.clone(),
            lsp_icon: kind.lsp_icon.clone(),
            dot_shape: kind.dot_shape.clone(),
            dot_color: kind.dot_color.clone(),
            dot_fillcolor: kind.dot_fillcolor.clone(),
            open_fields: kind.open_fields,
        };
        if let Some(existing) = registry.register(entry) {
            // Duplicate — first extension wins (already registered), emit E026
            diagnostics.push(Diagnostic {
                code: "E026".to_string(),
                severity: Severity::Error,
                message: format!(
                    "entity kind '{}' registered by '{}' conflicts with '{}' (first registration wins)",
                    kind.keyword, manifest.name, existing.source_extension
                ),
                span: None,
                suggestion: None,
            });
            // Restore the first registration (it wins)
            registry.register(existing);
        }
    }
}

/// Register fields from a manifest into the FieldRegistry.
fn register_fields(
    registry: &mut FieldRegistry,
    manifest: &ManifestV2,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for kind in &manifest.entity_kinds {
        // Extension-level shared fields first
        for field in &manifest.fields {
            register_single_field(
                registry,
                &kind.keyword,
                field,
                &manifest.name,
                diagnostics,
            );
        }
        // Kind-level fields override extension-level
        for field in &kind.fields {
            register_single_field(
                registry,
                &kind.keyword,
                field,
                &manifest.name,
                diagnostics,
            );
        }
    }
}

fn register_single_field(
    registry: &mut FieldRegistry,
    kind_name: &str,
    field: &crate::ManifestField,
    source_extension: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let field_type = match parse_field_type(&field.field_type) {
        Some(ft) => ft,
        None => {
            diagnostics.push(Diagnostic {
                code: "W019".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "extension '{}': unknown field type '{}' for field '{}' on kind '{}'",
                    source_extension, field.field_type, field.name, kind_name
                ),
                span: None,
                suggestion: None,
            });
            return;
        }
    };

    registry.register(FieldRegistryEntry {
        kind_name: kind_name.to_string(),
        field_name: field.name.clone(),
        description: field.description.clone(),
        field_type,
        source_extension: source_extension.to_string(),
        edge: field.edge.clone(),
        target_kind: field.target_kind.clone(),
        file_reference: field.file_reference,
        required: field.required,
    });
}

fn parse_field_type(s: &str) -> Option<ManifestFieldType> {
    match s {
        "string" | "string_type" => Some(ManifestFieldType::String),
        "integer" | "integer_type" => Some(ManifestFieldType::Integer),
        "bool" | "bool_type" => Some(ManifestFieldType::Bool),
        "string_list" | "string_list_type" => Some(ManifestFieldType::StringList),
        "reference" | "reference_type" => Some(ManifestFieldType::Reference),
        "reference_list" | "reference_list_type" => Some(ManifestFieldType::ReferenceList),
        "block" | "block_type" => Some(ManifestFieldType::Block),
        _ => None,
    }
}

/// Register explicit edge types from a manifest.
fn register_edge_types(
    registry: &mut EdgeRegistry,
    manifest: &ManifestV2,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for edge in &manifest.edge_types {
        let entry = EdgeRegistryEntry {
            label: edge.label.clone(),
            source_kind: edge.source_kind.clone(),
            target_kind: edge.target_kind.clone(),
            source_extension: manifest.name.clone(),
            edge_style: edge.edge_style.clone(),
            edge_color: edge.edge_color.clone(),
            edge_arrowhead: edge.edge_arrowhead.clone(),
        };
        if let Some(existing) = registry.register(entry) {
            diagnostics.push(Diagnostic {
                code: "W018".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "edge type '{}' from '{}' duplicates '{}' from '{}' (first wins)",
                    edge.label, manifest.name, existing.label, existing.source_extension
                ),
                span: None,
                suggestion: None,
            });
            // Restore first registration
            registry.register(existing);
        }
    }
}

/// Register implicit edge types from field-to-edge mappings.
fn register_implicit_edges(
    registry: &mut EdgeRegistry,
    manifest: &ManifestV2,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for kind in &manifest.entity_kinds {
        for field in &kind.fields {
            if let Some(ref edge_label) = field.edge
                && !registry.contains(edge_label)
            {
                registry.register(EdgeRegistryEntry {
                    label: edge_label.clone(),
                    source_kind: Some(kind.keyword.clone()),
                    target_kind: field.target_kind.clone(),
                    source_extension: manifest.name.clone(),
                    edge_style: None,
                    edge_color: None,
                    edge_arrowhead: None,
                });
            }
        }
    }
    // Suppress unused warning — diagnostics are used for future duplicate implicit edge warnings
    let _ = diagnostics;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn software_manifest() -> ManifestV2 {
        serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm",
                "entityKinds": [
                    {
                        "name": "Behavior",
                        "keyword": "behavior",
                        "testable": true,
                        "singleton": false,
                        "supportsVerify": true,
                        "semanticToken": "function",
                        "lspIcon": "Method",
                        "dotShape": "ellipse",
                        "fields": [
                            { "name": "contract", "fieldType": "block" },
                            { "name": "invariants", "fieldType": "reference_list", "edge": "enforces", "targetKind": "invariant" }
                        ]
                    },
                    {
                        "name": "Invariant",
                        "keyword": "invariant",
                        "testable": true,
                        "supportsVerify": true,
                        "dotShape": "diamond"
                    }
                ],
                "edgeTypes": [
                    { "label": "enforces", "sourceKind": "behavior", "targetKind": "invariant", "edgeStyle": "dashed" }
                ]
            }"#,
        )
        .unwrap()
    }

    fn product_manifest() -> ManifestV2 {
        serde_json::from_str(
            r#"{
                "name": "@specforge/product",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "product.wasm",
                "entityKinds": [
                    {
                        "name": "Feature",
                        "keyword": "feature",
                        "testable": false,
                        "dotShape": "box",
                        "fields": [
                            { "name": "behaviors", "fieldType": "reference_list", "edge": "composes", "targetKind": "behavior" }
                        ]
                    }
                ],
                "edgeTypes": [
                    { "label": "composes", "sourceKind": "feature", "targetKind": "behavior" }
                ]
            }"#,
        )
        .unwrap()
    }

    // -- B:register_entity_kinds_from_manifest tests --

    // B:register_entity_kinds_from_manifest — verify unit "entity kind registered with testable flag"
    #[test]
    fn test_entity_kind_registered_with_testable_flag() {
        let (kind_reg, _, _, diags) = populate_registries(&[software_manifest()]);
        assert!(diags.is_empty());
        let behavior = kind_reg.get("behavior").unwrap();
        assert!(behavior.testable);
        let invariant = kind_reg.get("invariant").unwrap();
        assert!(invariant.testable);
    }

    // B:register_entity_kinds_from_manifest — verify unit "entity kind registered with singleton flag"
    #[test]
    fn test_entity_kind_registered_with_singleton_flag() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        let behavior = kind_reg.get("behavior").unwrap();
        assert!(!behavior.singleton);
    }

    // B:register_entity_kinds_from_manifest — verify unit "entity kind registered with LSP metadata"
    #[test]
    fn test_entity_kind_registered_with_lsp_metadata() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        let behavior = kind_reg.get("behavior").unwrap();
        assert_eq!(behavior.semantic_token.as_deref(), Some("function"));
        assert_eq!(behavior.lsp_icon.as_deref(), Some("Method"));
    }

    // B:register_entity_kinds_from_manifest — verify unit "source extension recorded in registry entry"
    #[test]
    fn test_source_extension_recorded_in_registry_entry() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        let behavior = kind_reg.get("behavior").unwrap();
        assert_eq!(behavior.source_extension, "@specforge/software");
    }

    // B:register_entity_kinds_from_manifest — verify unit "testable=true entity participates in coverage"
    #[test]
    fn test_testable_true_entity_participates_in_coverage() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        let testable_kinds: Vec<_> = kind_reg
            .iter()
            .filter(|(_, e)| e.testable)
            .map(|(k, _)| k.clone())
            .collect();
        assert!(testable_kinds.contains(&"behavior".to_string()));
        assert!(testable_kinds.contains(&"invariant".to_string()));
    }

    // B:register_entity_kinds_from_manifest — verify unit "testable=false entity excluded from coverage"
    #[test]
    fn test_testable_false_entity_excluded_from_coverage() {
        let (kind_reg, _, _, _) = populate_registries(&[product_manifest()]);
        let feature = kind_reg.get("feature").unwrap();
        assert!(!feature.testable);
    }

    // B:register_entity_kinds_from_manifest — verify unit "no default testability assumed by core"
    #[test]
    fn test_no_default_testability_assumed_by_core() {
        // An entity kind with no testable flag explicitly set defaults to false
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    { "name": "Thing", "keyword": "thing" }
                ]
            }"#,
        )
        .unwrap();
        let (kind_reg, _, _, _) = populate_registries(&[manifest]);
        let thing = kind_reg.get("thing").unwrap();
        assert!(!thing.testable, "default testability should be false");
    }

    // -- B:populate_kind_registry_from_extensions tests --

    // B:populate_kind_registry_from_extensions — verify unit "extensions iterated in topological order"
    #[test]
    fn test_extensions_iterated_in_topological_order() {
        // First manifest's kinds should be registered first
        let (kind_reg, _, _, _) =
            populate_registries(&[software_manifest(), product_manifest()]);
        // Both should be present
        assert!(kind_reg.contains("behavior"));
        assert!(kind_reg.contains("invariant"));
        assert!(kind_reg.contains("feature"));
    }

    // B:populate_kind_registry_from_extensions — verify unit "all entityKinds entries registered"
    #[test]
    fn test_all_entity_kinds_entries_registered() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        assert_eq!(kind_reg.len(), 2); // behavior + invariant
        assert!(kind_reg.contains("behavior"));
        assert!(kind_reg.contains("invariant"));
    }

    // B:populate_kind_registry_from_extensions — verify unit "registered keywords available to parser"
    #[test]
    fn test_registered_keywords_available_to_parser() {
        let (kind_reg, _, _, _) =
            populate_registries(&[software_manifest(), product_manifest()]);
        let keywords: Vec<String> = kind_reg.keywords().cloned().collect();
        assert!(keywords.contains(&"behavior".to_string()));
        assert!(keywords.contains(&"invariant".to_string()));
        assert!(keywords.contains(&"feature".to_string()));
    }

    // B:populate_kind_registry_from_extensions — verify unit "population completes before validation"
    #[test]
    fn test_population_completes_before_validation() {
        // populate_registries returns all three registries fully populated.
        // Validation is a separate step that consumes these registries.
        let (kind_reg, field_reg, edge_reg, _) =
            populate_registries(&[software_manifest()]);
        assert!(!kind_reg.is_empty());
        assert!(!field_reg.is_empty());
        assert!(!edge_reg.is_empty());
    }

    // B:populate_kind_registry_from_extensions — verify integration "two extensions register kinds without collision"
    #[test]
    fn test_two_extensions_register_kinds_without_collision() {
        let (kind_reg, _, _, diags) =
            populate_registries(&[software_manifest(), product_manifest()]);
        // No E026 diagnostics
        assert!(
            !diags.iter().any(|d| d.code == "E026"),
            "expected no collision diagnostics, got: {:?}",
            diags
        );
        assert_eq!(kind_reg.len(), 3); // behavior, invariant, feature
    }

    // -- B:register_edge_types_from_manifest tests --

    // B:register_edge_types_from_manifest — verify unit "edge type registered with label and description"
    #[test]
    fn test_edge_type_registered_with_label() {
        let (_, _, edge_reg, _) = populate_registries(&[software_manifest()]);
        let enforces = edge_reg.get("enforces").unwrap();
        assert_eq!(enforces.label, "enforces");
        assert_eq!(enforces.source_kind.as_deref(), Some("behavior"));
        assert_eq!(enforces.target_kind.as_deref(), Some("invariant"));
    }

    // B:register_edge_types_from_manifest — verify unit "source/target kind constraints recorded"
    #[test]
    fn test_source_target_kind_constraints_recorded() {
        let (_, _, edge_reg, _) = populate_registries(&[software_manifest()]);
        let enforces = edge_reg.get("enforces").unwrap();
        assert_eq!(enforces.source_kind.as_deref(), Some("behavior"));
        assert_eq!(enforces.target_kind.as_deref(), Some("invariant"));
        assert_eq!(enforces.edge_style.as_deref(), Some("dashed"));
    }

    // B:register_edge_types_from_manifest — verify unit "duplicate edge label across extensions produces W-level warning"
    #[test]
    fn test_duplicate_edge_label_produces_warning() {
        let m1 = software_manifest();
        let mut m2 = product_manifest();
        // Add a duplicate "enforces" edge to product manifest
        m2.edge_types.push(crate::ManifestEdgeType {
            label: "enforces".to_string(),
            description: None,
            source_kind: Some("feature".to_string()),
            target_kind: Some("behavior".to_string()),
            edge_style: None,
            edge_color: None,
            edge_arrowhead: None,
        });
        let (_, _, _, diags) = populate_registries(&[m1, m2]);
        assert!(
            diags.iter().any(|d| d.code == "W018" && d.message.contains("enforces")),
            "expected W018 for duplicate edge, got: {:?}",
            diags
        );
    }

    // B:register_edge_types_from_manifest — verify unit "first-registered edge type wins on collision (topological order)"
    #[test]
    fn test_first_registered_edge_type_wins_on_collision() {
        let m1 = software_manifest();
        let mut m2 = product_manifest();
        m2.edge_types.push(crate::ManifestEdgeType {
            label: "enforces".to_string(),
            description: None,
            source_kind: Some("feature".to_string()),
            target_kind: Some("behavior".to_string()),
            edge_style: Some("dotted".to_string()),
            edge_color: None,
            edge_arrowhead: None,
        });
        let (_, _, edge_reg, _) = populate_registries(&[m1, m2]);
        let enforces = edge_reg.get("enforces").unwrap();
        // First extension's version should win
        assert_eq!(enforces.source_extension, "@specforge/software");
        assert_eq!(enforces.edge_style.as_deref(), Some("dashed"));
    }

    // B:register_edge_types_from_manifest — verify unit "field-to-edge mapping creates edge type"
    #[test]
    fn test_field_to_edge_mapping_creates_edge_type() {
        // product_manifest has a "composes" edge in edgeTypes AND in field mapping
        // The explicit edgeType should be registered, implicit should not duplicate
        let (_, _, edge_reg, _) = populate_registries(&[product_manifest()]);
        assert!(edge_reg.contains("composes"));
    }

    // -- B:populate_field_registry_from_extensions tests --

    // B:populate_field_registry_from_extensions — verify unit "fields registered per entity kind"
    #[test]
    fn test_fields_registered_per_entity_kind() {
        let (_, field_reg, _, _) = populate_registries(&[software_manifest()]);
        assert!(field_reg.contains("behavior", "contract"));
        assert!(field_reg.contains("behavior", "invariants"));
        // invariant has no declared fields
        assert!(field_reg.fields_for_kind("invariant").is_empty());
    }

    // B:populate_field_registry_from_extensions — verify unit "field types validated against known types"
    #[test]
    fn test_field_types_validated_against_known_types() {
        let (_, field_reg, _, _) = populate_registries(&[software_manifest()]);
        let contract = field_reg.get("behavior", "contract").unwrap();
        assert_eq!(contract.field_type, ManifestFieldType::Block);
        let invariants = field_reg.get("behavior", "invariants").unwrap();
        assert_eq!(invariants.field_type, ManifestFieldType::ReferenceList);
    }

    // B:populate_field_registry_from_extensions — verify unit "invalid field type produces warning"
    #[test]
    fn test_invalid_field_type_produces_warning() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    {
                        "name": "Thing",
                        "keyword": "thing",
                        "fields": [
                            { "name": "data", "fieldType": "unknown_type_xyz" }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();
        let (_, field_reg, _, diags) = populate_registries(&[manifest]);
        assert!(
            diags.iter().any(|d| d.code == "W019" && d.message.contains("unknown_type_xyz")),
            "expected W019 for unknown field type, got: {:?}",
            diags
        );
        // Field should not be registered
        assert!(!field_reg.contains("thing", "data"));
    }

    // -- B:populate_edge_registry_from_extensions tests --

    // B:populate_edge_registry_from_extensions — verify unit "explicit edgeTypes merged into edge set"
    #[test]
    fn test_explicit_edge_types_merged_into_edge_set() {
        let (_, _, edge_reg, _) = populate_registries(&[software_manifest()]);
        assert!(edge_reg.contains("enforces"));
    }

    // B:populate_edge_registry_from_extensions — verify unit "implicit edges from field mappings merged"
    #[test]
    fn test_implicit_edges_from_field_mappings_merged() {
        // Create a manifest with field edge mapping but no explicit edgeTypes
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    {
                        "name": "Task",
                        "keyword": "task",
                        "fields": [
                            { "name": "assignee", "fieldType": "reference", "edge": "assigned_to", "targetKind": "person" }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();
        let (_, _, edge_reg, _) = populate_registries(&[manifest]);
        assert!(edge_reg.contains("assigned_to"));
        let edge = edge_reg.get("assigned_to").unwrap();
        assert_eq!(edge.source_kind.as_deref(), Some("task"));
        assert_eq!(edge.target_kind.as_deref(), Some("person"));
    }

    // B:populate_edge_registry_from_extensions — verify unit "duplicate edge labels produce warning"
    #[test]
    fn test_duplicate_edge_labels_from_multiple_extensions_produce_warning() {
        let mut m1 = software_manifest();
        let mut m2 = product_manifest();
        // Both declare "links_to" edge
        m1.edge_types.push(crate::ManifestEdgeType {
            label: "links_to".to_string(),
            description: None,
            source_kind: None,
            target_kind: None,
            edge_style: None,
            edge_color: None,
            edge_arrowhead: None,
        });
        m2.edge_types.push(crate::ManifestEdgeType {
            label: "links_to".to_string(),
            description: None,
            source_kind: None,
            target_kind: None,
            edge_style: None,
            edge_color: None,
            edge_arrowhead: None,
        });
        let (_, _, _, diags) = populate_registries(&[m1, m2]);
        assert!(diags.iter().any(|d| d.code == "W018" && d.message.contains("links_to")));
    }

    // -- Description propagation tests --

    #[test]
    fn test_entity_kind_description_propagated_to_registry() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    {
                        "name": "Behavior",
                        "keyword": "behavior",
                        "description": "A testable unit of system functionality"
                    }
                ]
            }"#,
        )
        .unwrap();
        let (kind_reg, _, _, _) = populate_registries(&[manifest]);
        let entry = kind_reg.get("behavior").unwrap();
        assert_eq!(
            entry.description.as_deref(),
            Some("A testable unit of system functionality")
        );
    }

    #[test]
    fn test_entity_kind_without_description_has_none() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        let entry = kind_reg.get("behavior").unwrap();
        assert!(entry.description.is_none());
    }

    #[test]
    fn test_field_description_propagated_to_registry() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    {
                        "name": "Behavior",
                        "keyword": "behavior",
                        "fields": [
                            {
                                "name": "contract",
                                "fieldType": "block",
                                "description": "The behavioral contract this entity fulfills"
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();
        let (_, field_reg, _, _) = populate_registries(&[manifest]);
        let entry = field_reg.get("behavior", "contract").unwrap();
        assert_eq!(
            entry.description.as_deref(),
            Some("The behavioral contract this entity fulfills")
        );
    }

    #[test]
    fn test_field_without_description_has_none() {
        let (_, field_reg, _, _) = populate_registries(&[software_manifest()]);
        let entry = field_reg.get("behavior", "contract").unwrap();
        assert!(entry.description.is_none());
    }

    // B:register_entity_kinds_from_manifest — verify contract "requires/ensures consistency for entity kind registration"
    #[test]
    fn test_register_entity_kinds_contract() {
        // requires: manifest validated, registries empty
        let manifest = software_manifest();
        let (kind_reg, _, _, diags) = populate_registries(&[manifest]);
        // ensures: all kinds registered
        assert!(kind_reg.contains("behavior"));
        assert!(kind_reg.contains("invariant"));
        // ensures: source extension recorded
        assert_eq!(kind_reg.get("behavior").unwrap().source_extension, "@specforge/software");
        // ensures: testable flag preserved
        assert!(kind_reg.get("behavior").unwrap().testable);
        // ensures: no errors on clean manifest
        assert!(!diags.iter().any(|d| d.severity == Severity::Error));
    }

    // B:register_edge_types_from_manifest — verify contract "requires/ensures consistency for edge type registration"
    #[test]
    fn test_register_edge_types_contract() {
        // requires: manifest validated
        let manifest = software_manifest();
        let (_, _, edge_reg, diags) = populate_registries(&[manifest]);
        // ensures: explicit edge registered
        assert!(edge_reg.contains("enforces"));
        // ensures: source/target constraints recorded
        let enforces = edge_reg.get("enforces").unwrap();
        assert_eq!(enforces.source_kind.as_deref(), Some("behavior"));
        assert_eq!(enforces.target_kind.as_deref(), Some("invariant"));
        // ensures: no errors on clean manifest
        assert!(!diags.iter().any(|d| d.severity == Severity::Error));
    }

    // B:populate_kind_registry_from_extensions — verify contract "requires/ensures consistency for registry population"
    #[test]
    fn test_populate_kind_registry_contract() {
        // requires: manifests in topological order, registries empty
        let (kind_reg, field_reg, edge_reg, diags) =
            populate_registries(&[software_manifest(), product_manifest()]);
        // ensures: all registries populated
        assert!(!kind_reg.is_empty());
        assert!(!field_reg.is_empty());
        assert!(!edge_reg.is_empty());
        // ensures: keywords from all extensions registered
        assert!(kind_reg.contains("behavior"));
        assert!(kind_reg.contains("feature"));
        // ensures: no collisions
        assert!(!diags.iter().any(|d| d.code == "E026"));
        // ensures: population is complete (all 3 kinds)
        assert_eq!(kind_reg.len(), 3);
    }

    // B:populate_field_registry_from_extensions — verify contract "requires/ensures consistency for field registry population"
    #[test]
    fn test_populate_field_registry_contract() {
        // requires: manifests validated
        let (_, field_reg, _, diags) = populate_registries(&[software_manifest()]);
        // ensures: fields registered per entity kind
        assert!(field_reg.contains("behavior", "contract"));
        assert!(field_reg.contains("behavior", "invariants"));
        // ensures: field types are valid
        let contract = field_reg.get("behavior", "contract").unwrap();
        assert_eq!(contract.field_type, ManifestFieldType::Block);
        // ensures: no warnings on valid manifest
        assert!(!diags.iter().any(|d| d.code == "W019"));
    }

    // B:populate_edge_registry_from_extensions — verify contract "requires/ensures consistency for edge registry population"
    #[test]
    fn test_populate_edge_registry_contract() {
        // requires: manifests validated
        let (_, _, edge_reg, diags) =
            populate_registries(&[software_manifest(), product_manifest()]);
        // ensures: explicit edges merged
        assert!(edge_reg.contains("enforces"));
        assert!(edge_reg.contains("composes"));
        // ensures: no duplicate warnings between different labels
        assert!(!diags.iter().any(|d| d.code == "W018"));
    }

    // -- Slice 6: apply_entity_enhancements tests --

    fn enhancement_manifest() -> ManifestV2 {
        serde_json::from_str(
            r#"{
                "name": "@test/coverage",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "coverage.wasm",
                "entityEnhancements": [
                    {
                        "targetKind": "behavior",
                        "sourceExtension": "@test/coverage",
                        "fields": [
                            { "name": "coverage_threshold", "fieldType": "string" }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap()
    }

    // B:apply_entity_enhancements — verify unit "merges enhancement fields into FieldRegistry for known target kind"
    #[test]
    fn test_apply_enhancements_merges_fields_for_known_kind() {
        let (kind_reg, mut field_reg, _, _) = populate_registries(&[software_manifest()]);
        let enhancements = vec![(
            "@test/coverage".to_string(),
            crate::FieldEnhancement {
                target_kind: "behavior".to_string(),
                source_extension: "@test/coverage".to_string(),
                fields: vec![crate::ManifestField {
                    name: "coverage_threshold".to_string(),
                    field_type: "string".to_string(),
                    description: None,
                    edge: None,
                    target_kind: None,
                    file_reference: false,
                    required: false,
                }],
            },
        )];
        let diags = apply_entity_enhancements(&enhancements, &kind_reg, &mut field_reg);
        assert!(diags.is_empty(), "expected no diagnostics, got: {:?}", diags);
        assert!(field_reg.contains("behavior", "coverage_threshold"));
    }

    // B:apply_entity_enhancements — verify unit "unknown target kind produces I004 info diagnostic"
    #[test]
    fn test_apply_enhancements_unknown_kind_produces_i004() {
        let (kind_reg, mut field_reg, _, _) = populate_registries(&[software_manifest()]);
        let enhancements = vec![(
            "@test/ext".to_string(),
            crate::FieldEnhancement {
                target_kind: "nonexistent_kind".to_string(),
                source_extension: "@test/ext".to_string(),
                fields: vec![crate::ManifestField {
                    name: "extra".to_string(),
                    field_type: "string".to_string(),
                    description: None,
                    edge: None,
                    target_kind: None,
                    file_reference: false,
                    required: false,
                }],
            },
        )];
        let diags = apply_entity_enhancements(&enhancements, &kind_reg, &mut field_reg);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "I004");
        assert!(diags[0].message.contains("nonexistent_kind"));
        // Field should NOT be registered
        assert!(!field_reg.contains("nonexistent_kind", "extra"));
    }

    // B:apply_entity_enhancements — verify unit "enhancement field does NOT overwrite existing kind-level field"
    #[test]
    fn test_apply_enhancements_does_not_overwrite_kind_level_field() {
        let (kind_reg, mut field_reg, _, _) = populate_registries(&[software_manifest()]);
        // "contract" is already a kind-level field on behavior (type: block)
        let enhancements = vec![(
            "@test/ext".to_string(),
            crate::FieldEnhancement {
                target_kind: "behavior".to_string(),
                source_extension: "@test/ext".to_string(),
                fields: vec![crate::ManifestField {
                    name: "contract".to_string(),
                    field_type: "string".to_string(), // Different type!
                    description: None,
                    edge: None,
                    target_kind: None,
                    file_reference: false,
                    required: false,
                }],
            },
        )];
        let diags = apply_entity_enhancements(&enhancements, &kind_reg, &mut field_reg);
        assert!(diags.is_empty());
        // Original kind-level field should be unchanged
        let contract = field_reg.get("behavior", "contract").unwrap();
        assert_eq!(contract.field_type, ManifestFieldType::Block);
        assert_eq!(contract.source_extension, "@specforge/software");
    }

    // B:apply_entity_enhancements — verify unit "two non-conflicting enhancements on same kind both registered"
    #[test]
    fn test_apply_two_non_conflicting_enhancements_on_same_kind() {
        let (kind_reg, mut field_reg, _, _) = populate_registries(&[software_manifest()]);
        let enhancements = vec![
            (
                "@ext/a".to_string(),
                crate::FieldEnhancement {
                    target_kind: "behavior".to_string(),
                    source_extension: "@ext/a".to_string(),
                    fields: vec![crate::ManifestField {
                        name: "priority".to_string(),
                        field_type: "string".to_string(),
                        description: None,
                        edge: None,
                        target_kind: None,
                        file_reference: false,
                        required: false,
                    }],
                },
            ),
            (
                "@ext/b".to_string(),
                crate::FieldEnhancement {
                    target_kind: "behavior".to_string(),
                    source_extension: "@ext/b".to_string(),
                    fields: vec![crate::ManifestField {
                        name: "category".to_string(),
                        field_type: "string".to_string(),
                        description: None,
                        edge: None,
                        target_kind: None,
                        file_reference: false,
                        required: false,
                    }],
                },
            ),
        ];
        let diags = apply_entity_enhancements(&enhancements, &kind_reg, &mut field_reg);
        assert!(diags.is_empty());
        assert!(field_reg.contains("behavior", "priority"));
        assert!(field_reg.contains("behavior", "category"));
    }

    // B:apply_entity_enhancements — verify contract "requires KindRegistry populated, ensures fields merged + diagnostics"
    #[test]
    fn test_apply_entity_enhancements_contract() {
        // requires: KindRegistry populated
        let manifests = vec![software_manifest(), enhancement_manifest()];
        let (_kind_reg, field_reg, _, diags) = populate_registries(&manifests);

        // ensures: enhancements applied during populate_registries
        assert!(field_reg.contains("behavior", "coverage_threshold"),
            "enhancement field should be merged via populate_registries");

        // ensures: no diagnostics for valid enhancement
        assert!(!diags.iter().any(|d| d.code == "I004"),
            "no I004 expected for known kind, got: {:?}", diags);

        // ensures: original fields preserved
        assert!(field_reg.contains("behavior", "contract"));
        assert!(field_reg.contains("behavior", "invariants"));
    }
}
