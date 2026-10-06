use crate::{
    EdgeRegistry, EdgeRegistryEntry, FieldRegistry, FieldRegistryEntry, KindRegistry,
    KindRegistryEntry, ManifestFieldType, ProofRole,
};
use specforge_common::{Diagnostic, DiagnosticData, Severity};
use specforge_protocol_types::{
    EntityEnhancementDescriptor, EntityKindDescriptor, ExtensionDeclaration, FieldDescriptor,
};

/// The keyword a kind's entities are written with: its declared keyword,
/// else its name.
pub(crate) fn keyword(kind: &EntityKindDescriptor) -> &str {
    kind.keyword.as_deref().unwrap_or(&kind.name)
}

/// Populate all three registries from the declarations, in load order
/// (dependencies first), plus the diagnostics of doing so.
pub(crate) fn populate(
    declarations: &[ExtensionDeclaration],
) -> (KindRegistry, FieldRegistry, EdgeRegistry, Vec<Diagnostic>) {
    let mut kind_reg = KindRegistry::new();
    let mut field_reg = FieldRegistry::new();
    let mut edge_reg = EdgeRegistry::new();
    let mut diagnostics = Vec::new();

    for declaration in declarations {
        register_entity_kinds(&mut kind_reg, declaration, &mut diagnostics);
        register_fields(&mut field_reg, declaration, &mut diagnostics);
        register_edge_types(&mut edge_reg, declaration, &mut diagnostics);
        register_implicit_edges(&mut edge_reg, declaration);
    }

    // After every extension, apply the entity enhancements.
    let all_enhancements: Vec<(String, EntityEnhancementDescriptor)> = declarations
        .iter()
        .flat_map(|d| {
            d.enhancements
                .iter()
                .map(move |e| (d.name().to_string(), e.clone()))
        })
        .collect();
    let loaded: Vec<String> = declarations.iter().map(|d| d.name().to_string()).collect();
    let enh_diags =
        apply_entity_enhancements(&all_enhancements, &loaded, &mut kind_reg, &mut field_reg);
    diagnostics.extend(enh_diags);

    (kind_reg, field_reg, edge_reg, diagnostics)
}

/// Apply entity enhancements to the FieldRegistry.
/// Enhancement fields do NOT overwrite existing kind-level fields.
///
/// An enhancement whose target kind is unknown is skipped. It is *conditional*
/// — skipped silently — when it names another extension as the kind's owner
/// (`source_extension`) and that extension is not in `loaded_extensions`: the
/// project simply doesn't use it. Any other unknown target is reported as I004.
///
/// An enhancement carrying `verify_kinds` makes its target kind testable
/// with exactly those kinds (ADR 0002).
pub fn apply_entity_enhancements(
    enhancements: &[(String, EntityEnhancementDescriptor)],
    loaded_extensions: &[String],
    kind_reg: &mut KindRegistry,
    field_reg: &mut FieldRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (ext_name, enhancement) in enhancements {
        if !kind_reg.contains(&enhancement.target_kind) {
            let owner = &enhancement.source_extension;
            let owner_absent = !owner.is_empty()
                && owner != ext_name
                && !loaded_extensions.iter().any(|loaded| loaded == owner);
            if owner_absent {
                continue;
            }
            diagnostics.push(Diagnostic {
                code: "I004".to_string(),
                severity: Severity::Info,
                message: format!(
                    "extension '{}': entity enhancement targets unknown kind '{}' (extension may not be installed)",
                    ext_name, enhancement.target_kind
                ),
                span: None,
                suggestion: None,
                data: None,
            });
            continue;
        }

        if let Some(verify_kinds) = &enhancement.verify_kinds
            && let Some(kind) = kind_reg.get_mut(&enhancement.target_kind)
        {
            kind.testable = true;
            kind.supports_verify = true;
            kind.allowed_verify_kinds = verify_kinds.clone();
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

/// Register a declaration's entity kinds into the KindRegistry.
fn register_entity_kinds(
    registry: &mut KindRegistry,
    declaration: &ExtensionDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for kind in &declaration.entities {
        let keyword = keyword(kind);
        let entry = KindRegistryEntry {
            kind_name: keyword.to_string(),
            source_extension: declaration.name().to_string(),
            testable: kind.testable,
            supports_verify: kind.supports_verify,
            allowed_verify_kinds: kind.verify_kinds.clone(),
            lifecycle_field: lifecycle_field(kind, declaration, diagnostics),
            declared: kind.clone(),
        };
        if let Some(existing) = registry.register(entry) {
            // Duplicate — first extension wins (already registered), emit E026
            diagnostics.push(Diagnostic {
                code: "E026".to_string(),
                severity: Severity::Error,
                message: format!(
                    "entity kind '{}' registered by '{}' conflicts with '{}' (first registration wins)",
                    keyword,
                    declaration.name(),
                    existing.source_extension
                ),
                span: None,
                suggestion: None,
                data: Some(Box::new(DiagnosticData::ShadowedKeyword {
                    keyword: keyword.to_string(),
                })),
            });
            // Restore the first registration (it wins)
            registry.register(existing);
        }
    }
}

/// Register a declaration's fields into the FieldRegistry: its shared
/// fields on every kind it declares, then each kind's own.
fn register_fields(
    registry: &mut FieldRegistry,
    declaration: &ExtensionDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for kind in &declaration.entities {
        // Extension-level shared fields first
        for field in &declaration.shared_fields {
            register_single_field(
                registry,
                keyword(kind),
                field,
                declaration.name(),
                diagnostics,
            );
        }
        // Kind-level fields override extension-level
        for field in &kind.fields {
            register_single_field(
                registry,
                keyword(kind),
                field,
                declaration.name(),
                diagnostics,
            );
        }
    }
}

fn register_single_field(
    registry: &mut FieldRegistry,
    kind_name: &str,
    field: &FieldDescriptor,
    source_extension: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let field_type = match parse_field_type(&field.field_type) {
        // An enum's values come with the field, not its type name.
        Some(ManifestFieldType::Enum(_)) => ManifestFieldType::Enum(field.enum_values.clone()),
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
                data: None,
            });
            return;
        }
    };

    registry.register(FieldRegistryEntry {
        kind_name: kind_name.to_string(),
        source_extension: source_extension.to_string(),
        field_type,
        proof_role: proof_role(kind_name, field, source_extension, diagnostics),
        declared: field.clone(),
    });
}

/// The field `kind` declares as its lifecycle field, when it declares one
/// among its own or the extension's shared fields; a name it does not
/// declare is refused (W021).
fn lifecycle_field(
    kind: &EntityKindDescriptor,
    declaration: &ExtensionDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    let name = kind.lifecycle_field.as_ref()?;
    let declared = kind
        .fields
        .iter()
        .chain(&declaration.shared_fields)
        .any(|f| &f.name == name);
    if declared {
        return Some(name.clone());
    }
    diagnostics.push(Diagnostic {
        code: "W021".to_string(),
        severity: Severity::Warning,
        message: format!(
            "extension '{}': kind '{}' declares lifecycle_field '{}', which is not one of its fields",
            declaration.name(),
            keyword(kind),
            name
        ),
        span: None,
        suggestion: None,
        data: None,
    });
    None
}

/// The prove-pass role `field` declares, when it names one; any value but
/// `bound` or `claim` is refused (W021).
fn proof_role(
    kind_name: &str,
    field: &FieldDescriptor,
    source_extension: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProofRole> {
    let name = field.proof_role.as_deref()?;
    let role = ProofRole::parse(name);
    if role.is_none() {
        diagnostics.push(Diagnostic {
            code: "W021".to_string(),
            severity: Severity::Warning,
            message: format!(
                "extension '{}': field '{}' on kind '{}' declares proof_role '{}': expected 'bound' or 'claim'",
                source_extension, field.name, kind_name, name
            ),
            span: None,
            suggestion: None,
            data: None,
        });
    }
    role
}

fn parse_field_type(s: &str) -> Option<ManifestFieldType> {
    specforge_protocol_types::FieldType::parse(s).map(ManifestFieldType::from)
}

/// Register a declaration's explicit edge types.
fn register_edge_types(
    registry: &mut EdgeRegistry,
    declaration: &ExtensionDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for edge in &declaration.edges {
        let entry = EdgeRegistryEntry {
            source_extension: declaration.name().to_string(),
            declared: edge.clone(),
        };
        if let Some(existing) = registry.register(entry) {
            diagnostics.push(Diagnostic {
                code: "W018".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "edge type '{}' from '{}' duplicates '{}' from '{}' (first wins)",
                    edge.label,
                    declaration.name(),
                    existing.declared.label,
                    existing.source_extension
                ),
                span: None,
                suggestion: None,
                data: None,
            });
            // Restore first registration
            registry.register(existing);
        }
    }
}

/// Register the edge types a declaration's fields map to without declaring
/// them.
fn register_implicit_edges(registry: &mut EdgeRegistry, declaration: &ExtensionDeclaration) {
    for kind in &declaration.entities {
        for field in &kind.fields {
            if let Some(ref edge_label) = field.edge
                && !registry.contains(edge_label)
            {
                registry.register(EdgeRegistryEntry {
                    source_extension: declaration.name().to_string(),
                    declared: specforge_protocol_types::EdgeTypeDescriptor {
                        label: edge_label.clone(),
                        source_kind: Some(keyword(kind).to_string()),
                        target_kind: field.target_kind.clone(),
                        ..Default::default()
                    },
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation::tests::support::{declare, software};
    use specforge_extension_sdk::prelude::*;

    fn software_manifest() -> ExtensionDeclaration {
        software()
    }

    /// A string field `name`, as an enhancement declares it.
    fn string_field(name: &str) -> FieldDescriptor {
        FieldDescriptor {
            name: name.to_string(),
            field_type: FieldType::String.as_str().to_string(),
            ..Default::default()
        }
    }

    /// An enhancement of `target_kind` by `owner` adding string `fields`.
    fn enhancement(target_kind: &str, owner: &str, fields: &[&str]) -> EntityEnhancementDescriptor {
        EntityEnhancementDescriptor {
            target_kind: target_kind.to_string(),
            source_extension: owner.to_string(),
            fields: fields.iter().map(|f| string_field(f)).collect(),
            edge_types: vec![],
            verify_kinds: None,
        }
    }

    // -- B:populate_kind_registry_from_extensions tests --

    // -- B:register_edge_types_from_manifest tests --

    // -- B:populate_field_registry_from_extensions tests --

    // -- B:populate_edge_registry_from_extensions tests --

    // -- Description propagation tests --

    // -- parse_field_type tests --

    // -- Slice 6: apply_entity_enhancements tests --

    fn enhancement_manifest() -> ExtensionDeclaration {
        declare("@test/coverage", |c| {
            c.enhance("behavior", "@test/coverage", |e| {
                e.field("coverage_threshold", |f| {
                    f.field_type(FieldType::String);
                });
            });
        })
    }
    // B:apply_entity_enhancements — verify unit "merges enhancement fields into FieldRegistry for known target kind"
    #[test]
    fn test_apply_enhancements_merges_fields_for_known_kind() {
        let (mut kind_reg, mut field_reg, _, _) = populate(&[software_manifest()]);
        let enhancements = vec![(
            "@test/coverage".to_string(),
            enhancement("behavior", "@test/coverage", &["coverage_threshold"]),
        )];
        let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
        assert!(
            diags.is_empty(),
            "expected no diagnostics, got: {:?}",
            diags
        );
        assert!(field_reg.contains("behavior", "coverage_threshold"));
    }

    // B:apply_entity_enhancements — verify unit "unknown target kind produces I004 info diagnostic"
    #[test]
    fn test_apply_enhancements_unknown_kind_produces_i004() {
        let (mut kind_reg, mut field_reg, _, _) = populate(&[software_manifest()]);
        let enhancements = vec![(
            "@test/ext".to_string(),
            enhancement("nonexistent_kind", "@test/ext", &["extra"]),
        )];
        let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "I004");
        assert!(diags[0].message.contains("nonexistent_kind"));
        // Field should NOT be registered
        assert!(!field_reg.contains("nonexistent_kind", "extra"));
    }

    fn enhancement_of(target_kind: &str, owner: &str) -> EntityEnhancementDescriptor {
        enhancement(target_kind, owner, &[])
    }

    // B:apply_entity_enhancements — verify unit "enhancement of a kind owned by an extension that is not loaded is skipped silently"
    #[test]
    fn test_apply_enhancements_for_absent_owner_is_silent() {
        let (mut kind_reg, mut field_reg, _, _) = populate(&[software_manifest()]);
        let enhancements = vec![(
            "@specforge/software".to_string(),
            enhancement_of("module", "@specforge/product"),
        )];
        let loaded = vec!["@specforge/software".to_string()];
        let diags =
            apply_entity_enhancements(&enhancements, &loaded, &mut kind_reg, &mut field_reg);
        assert!(diags.is_empty(), "product not loaded: nothing to report");

        // Owner loaded yet the kind is missing: a real mismatch, still I004.
        let loaded = vec![
            "@specforge/software".to_string(),
            "@specforge/product".to_string(),
        ];
        let diags =
            apply_entity_enhancements(&enhancements, &loaded, &mut kind_reg, &mut field_reg);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "I004");
    }

    // B:register_entity_enhancements — verify unit "an enhancement with verify kinds makes its target kind testable"
    #[test]
    fn test_apply_enhancements_with_verify_kinds_makes_kind_testable() {
        let (mut kind_reg, mut field_reg, _, _) = populate(&[software_manifest()]);
        kind_reg.get_mut("behavior").unwrap().supports_verify = false;
        let mut enhancement = enhancement_of("behavior", "@specforge/software");
        enhancement.verify_kinds = Some(vec!["unit".to_string(), "contract".to_string()]);
        let enhancements = vec![("@specforge/testing".to_string(), enhancement)];

        let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
        assert!(diags.is_empty());
        let behavior = kind_reg.get("behavior").unwrap();
        assert!(behavior.supports_verify && behavior.testable);
        assert_eq!(behavior.allowed_verify_kinds, ["unit", "contract"]);
    }

    // B:apply_entity_enhancements — verify unit "enhancement field does NOT overwrite existing kind-level field"
    #[test]
    fn test_apply_enhancements_does_not_overwrite_kind_level_field() {
        let (mut kind_reg, mut field_reg, _, _) = populate(&[software_manifest()]);
        // "contract" is already a kind-level field on behavior (type: block)
        let enhancements = vec![(
            "@test/ext".to_string(),
            // A string field: a different type!
            enhancement("behavior", "@test/ext", &["contract"]),
        )];
        let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
        assert!(diags.is_empty());
        // Original kind-level field should be unchanged
        let contract = field_reg.get("behavior", "contract").unwrap();
        assert_eq!(contract.field_type, ManifestFieldType::Block);
        assert_eq!(contract.source_extension, "@specforge/software");
    }

    // B:apply_entity_enhancements — verify unit "two non-conflicting enhancements on same kind both registered"
    #[test]
    fn test_apply_two_non_conflicting_enhancements_on_same_kind() {
        let (mut kind_reg, mut field_reg, _, _) = populate(&[software_manifest()]);
        let enhancements = vec![
            (
                "@ext/a".to_string(),
                enhancement("behavior", "@ext/a", &["priority"]),
            ),
            (
                "@ext/b".to_string(),
                enhancement("behavior", "@ext/b", &["category"]),
            ),
        ];
        let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
        assert!(diags.is_empty());
        assert!(field_reg.contains("behavior", "priority"));
        assert!(field_reg.contains("behavior", "category"));
    }

    // B:apply_entity_enhancements — verify contract "requires KindRegistry populated, ensures fields merged + diagnostics"
    #[test]
    fn test_apply_entity_enhancements_contract() {
        // requires: KindRegistry populated
        let manifests = vec![software_manifest(), enhancement_manifest()];
        let (_kind_reg, field_reg, _, diags) = populate(&manifests);

        // ensures: enhancements applied during populate_registries
        assert!(
            field_reg.contains("behavior", "coverage_threshold"),
            "enhancement field should be merged via populate_registries"
        );

        // ensures: no diagnostics for valid enhancement
        assert!(
            !diags.iter().any(|d| d.code == "I004"),
            "no I004 expected for known kind, got: {:?}",
            diags
        );

        // ensures: original fields preserved
        assert!(field_reg.contains("behavior", "contract"));
        assert!(field_reg.contains("behavior", "invariants"));
    }
}
