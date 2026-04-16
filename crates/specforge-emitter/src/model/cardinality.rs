use super::{Cardinality, ModelEntity, ModelFieldType};

/// Infer cardinality for an edge by examining source entity fields.
///
/// Looks for a field on the source entity where `field.edge == Some(edge_label)`.
/// - `reference` field type -> ManyToOne
/// - `reference_list` field type -> ManyToMany
/// - No matching field -> ManyToMany (safe default)
///
/// Returns (cardinality, source_field_name).
pub fn infer_cardinality(
    edge_label: &str,
    source_entity: &ModelEntity,
) -> (Cardinality, Option<String>) {
    // The source entity's fields carry edge metadata from SchemaField enrichment.
    // We need to match by the field's references (target_kind) and the field name
    // pattern. The edge label was stored on SchemaField.edge and used during build
    // to set references. But at ModelField level, we don't have edge directly — we
    // need to check the field that was created from a SchemaField with edge == edge_label.
    //
    // Since build.rs preserves SchemaField order and we have the references field,
    // we look for fields whose reference/reference_list type + edge label match.
    //
    // Actually, we need to pass the edge info through. Let's check for fields that
    // have references set and match the pattern. The cleanest approach is to check
    // the original SchemaField.edge during build and pass it here. But since we
    // already built ModelEntity, let's work with what we have.
    //
    // The edge metadata is not on ModelField directly. We need to refine the approach:
    // use the SchemaEntityKind during relationship building instead.

    // For now, scan fields by name or by reference target.
    // The field name won't necessarily match the edge label, but the SchemaField.edge
    // field was set. We need to carry that through.

    // Let's add an internal edge_label field to help with lookup.
    for field in &source_entity.fields {
        if field.is_primary_key {
            continue;
        }
        // Check if this field's internal edge label matches
        if field.edge_label.as_deref() == Some(edge_label) {
            let cardinality = match field.field_type {
                ModelFieldType::Reference => Cardinality::ManyToOne,
                ModelFieldType::ReferenceList => Cardinality::ManyToMany,
                _ => Cardinality::ManyToMany,
            };
            return (cardinality, Some(field.name.clone()));
        }
    }

    (Cardinality::ManyToMany, None)
}
