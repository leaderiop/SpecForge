use super::{Cardinality, ModelEntity, ModelFieldType};

/// Infer cardinality for an edge by examining source entity fields.
///
/// A relationship's cardinality is read from the source entity's field
/// carrying `edge_label`: `reference` -> ManyToOne, `reference_list` ->
/// ManyToMany, no matching field -> ManyToMany (safe default).
///
/// Returns (cardinality, source_field_name).
pub fn infer_cardinality(
    edge_label: &str,
    source_entity: &ModelEntity,
) -> (Cardinality, Option<String>) {
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
