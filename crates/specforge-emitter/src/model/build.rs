use std::collections::HashMap;

use crate::schema::{GraphProtocolSchema, SchemaEdgeType, SchemaEntityKind};
use super::{
    Cardinality, ModelEntity, ModelExtension, ModelField, ModelFieldType,
    ModelIntermediate, ModelRelationship,
};
use super::cardinality::infer_cardinality;

#[allow(non_snake_case)]
pub fn ModelIntermediate_from_schema(schema: &GraphProtocolSchema) -> ModelIntermediate {
    let entities: Vec<ModelEntity> = schema
        .entity_kinds
        .iter()
        .map(build_entity)
        .collect();

    let entity_map: HashMap<&str, &ModelEntity> = entities
        .iter()
        .map(|e| (e.name.as_str(), e))
        .collect();

    let relationships: Vec<ModelRelationship> = schema
        .edge_types
        .iter()
        .flat_map(|edge| build_relationships(edge, &entity_map))
        .collect();

    let extensions = build_extensions(schema, &entities);

    let edge_type_owners: Vec<(String, String)> = schema
        .edge_types
        .iter()
        .map(|e| (e.label.clone(), e.source_extension.clone()))
        .collect();

    ModelIntermediate {
        model_version: "1.0.0".to_string(),
        extensions,
        entities,
        relationships,
        edge_type_owners,
    }
}

fn build_entity(kind: &SchemaEntityKind) -> ModelEntity {
    let mut fields = vec![ModelField {
        name: "id".to_string(),
        field_type: ModelFieldType::String,
        required: true,
        description: None,
        default_value: None,
        enum_values: None,
        is_primary_key: true,
        references: None,
        edge_label: None,
        contributed_by: None,
        contribution: None,
    }];

    let mut enhancing_extensions: Vec<String> = Vec::new();

    for sf in &kind.fields {
        let field_type = ModelFieldType::from_schema_str(&sf.field_type);
        let references = sf.target_kind.clone();

        // Determine contribution info: "EdgeLabel -> target_kind"
        let contribution = match (&sf.edge, &sf.target_kind) {
            (Some(edge), Some(target)) => Some(format!("{} -> {}", edge, target)),
            (None, Some(target)) => Some(format!("-> {}", target)),
            _ => None,
        };

        // Track cross-extension contributions
        let contributed_by = if sf.source_extension != kind.source_extension {
            let ext = sf.source_extension.clone();
            if !enhancing_extensions.contains(&ext) {
                enhancing_extensions.push(ext.clone());
            }
            Some(ext)
        } else {
            None
        };

        fields.push(ModelField {
            name: sf.name.clone(),
            field_type,
            required: sf.required,
            description: sf.description.clone(),
            default_value: sf.default_value.clone(),
            enum_values: sf.enum_values.clone(),
            is_primary_key: false,
            references,
            edge_label: sf.edge.clone(),
            contributed_by,
            contribution,
        });
    }

    enhancing_extensions.sort();

    ModelEntity {
        name: kind.name.clone(),
        extension: kind.source_extension.clone(),
        description: None,
        fields,
        enhanced_by: enhancing_extensions,
    }
}

fn build_relationships(
    edge: &SchemaEdgeType,
    entity_map: &HashMap<&str, &ModelEntity>,
) -> Vec<ModelRelationship> {
    let source_kinds = match &edge.source_kinds {
        Some(kinds) => kinds,
        None => return Vec::new(),
    };
    let target_kinds = match &edge.target_kinds {
        Some(kinds) => kinds,
        None => return Vec::new(),
    };

    let mut relationships = Vec::new();

    for source_name in source_kinds {
        for target_name in target_kinds {
            let (cardinality, source_field) = match entity_map.get(source_name.as_str()) {
                Some(entity) => infer_cardinality(&edge.label, entity),
                None => (Cardinality::ManyToMany, None),
            };

            relationships.push(ModelRelationship {
                name: edge.label.clone(),
                source: source_name.clone(),
                target: target_name.clone(),
                cardinality,
                source_field,
                description: None,
            });
        }
    }

    relationships
}

fn build_extensions(
    schema: &GraphProtocolSchema,
    entities: &[ModelEntity],
) -> Vec<ModelExtension> {
    // Count entities per extension
    let mut entity_counts: HashMap<&str, usize> = HashMap::new();
    for entity in entities {
        *entity_counts.entry(entity.extension.as_str()).or_insert(0) += 1;
    }

    // Count edges per extension (from schema edge_types)
    let mut edge_counts: HashMap<&str, usize> = HashMap::new();
    for edge in &schema.edge_types {
        *edge_counts.entry(edge.source_extension.as_str()).or_insert(0) += 1;
    }

    schema
        .extensions
        .iter()
        .map(|ext| ModelExtension {
            name: ext.name.clone(),
            version: ext.version.clone(),
            entity_count: entity_counts.get(ext.name.as_str()).copied().unwrap_or(0),
            edge_count: edge_counts.get(ext.name.as_str()).copied().unwrap_or(0),
        })
        .collect()
}
