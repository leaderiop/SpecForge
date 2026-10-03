//! Registries the export tests share.

use specforge_registry::{FieldRegistry, FieldRegistryEntry, ManifestFieldType};

/// A field registry in which each of `kinds` declares `contract` and
/// `status` as headline fields, as `@specforge/software` declares them on
/// `behavior`.
pub fn headline_registry(kinds: &[&str]) -> FieldRegistry {
    let mut registry = FieldRegistry::new();
    for (kind, field) in kinds
        .iter()
        .flat_map(|kind| ["contract", "status"].map(|field| (*kind, field)))
    {
        registry.register(FieldRegistryEntry {
            kind_name: kind.to_string(),
            field_name: field.to_string(),
            description: None,
            field_type: ManifestFieldType::String,
            source_extension: "@test/ext".to_string(),
            edge: None,
            target_kind: None,
            file_reference: false,
            required: false,
            inverse_of: None,
            normative: field == "contract",
            exempts_obligations: false,
            headline: true,
            derived_from: None,
            proof_role: None,
        });
    }
    registry
}
