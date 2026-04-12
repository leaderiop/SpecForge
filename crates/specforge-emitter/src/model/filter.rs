use std::collections::{HashMap, HashSet, VecDeque};

use super::{
    FieldLevel, ModelEntity, ModelExtension, ModelFieldType, ModelIntermediate, ModelOptions,
    ModelRelationship,
};

pub fn filter_entities(model: &ModelIntermediate, options: &ModelOptions) -> ModelIntermediate {
    let has_filter = options.extension_filter.is_some()
        || options.kind_filter.is_some()
        || options.root.is_some();

    // No filter applied — return model as-is
    if !has_filter {
        return model.clone();
    }

    let mut keep: HashSet<&str> = model.entities.iter().map(|e| e.name.as_str()).collect();

    // Extension filter
    if let Some(ref ext) = options.extension_filter {
        keep.retain(|name| {
            model.entities.iter().any(|e| e.name == *name && e.extension == *ext)
        });
    }

    // Kind filter
    if let Some(ref kinds) = options.kind_filter {
        let kind_set: HashSet<&str> = kinds.iter().map(|s| s.as_str()).collect();
        keep.retain(|name| kind_set.contains(name));
    }

    // Root + depth: BFS on kind-level adjacency graph
    if let Some(ref root) = options.root {
        let depth = options.depth.unwrap_or(usize::MAX);
        let reachable = bfs_reachable(root, depth, &model.relationships);
        keep.retain(|name| reachable.contains(*name));
    }

    // Filter entities
    let entities: Vec<ModelEntity> = model
        .entities
        .iter()
        .filter(|e| keep.contains(e.name.as_str()))
        .cloned()
        .collect();

    // Prune relationships where either endpoint is filtered out
    let relationships: Vec<ModelRelationship> = model
        .relationships
        .iter()
        .filter(|r| keep.contains(r.source.as_str()) && keep.contains(r.target.as_str()))
        .cloned()
        .collect();

    // Filter edge_type_owners to only edges whose declaring extension
    // still has surviving entities
    let surviving_extensions: HashSet<&str> = entities
        .iter()
        .map(|e| e.extension.as_str())
        .collect();
    let edge_type_owners: Vec<(String, String)> = model
        .edge_type_owners
        .iter()
        .filter(|(_, ext)| surviving_extensions.contains(ext.as_str()))
        .cloned()
        .collect();

    // Recompute extension metadata using edge_type_owners for accurate attribution
    let extensions = recompute_extensions(&model.extensions, &entities, &edge_type_owners);

    ModelIntermediate {
        model_version: model.model_version.clone(),
        extensions,
        entities,
        relationships,
        edge_type_owners,
    }
}

fn bfs_reachable(root: &str, max_depth: usize, relationships: &[ModelRelationship]) -> HashSet<String> {
    // Build undirected adjacency at the kind level
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for rel in relationships {
        adj.entry(rel.source.as_str()).or_default().push(rel.target.as_str());
        adj.entry(rel.target.as_str()).or_default().push(rel.source.as_str());
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(&str, usize)> = VecDeque::new();

    visited.insert(root.to_string());
    queue.push_back((root, 0));

    while let Some((node, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        if let Some(neighbors) = adj.get(node) {
            for &neighbor in neighbors {
                if !visited.contains(neighbor) {
                    visited.insert(neighbor.to_string());
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }
    }

    visited
}

fn recompute_extensions(
    original: &[ModelExtension],
    entities: &[ModelEntity],
    edge_type_owners: &[(String, String)],
) -> Vec<ModelExtension> {
    let mut entity_counts: HashMap<&str, usize> = HashMap::new();
    for e in entities {
        *entity_counts.entry(e.extension.as_str()).or_insert(0) += 1;
    }

    // Count unique edge types per declaring extension
    let mut edge_counts: HashMap<&str, usize> = HashMap::new();
    for (_, ext) in edge_type_owners {
        *edge_counts.entry(ext.as_str()).or_insert(0) += 1;
    }

    original
        .iter()
        .map(|ext| ModelExtension {
            name: ext.name.clone(),
            version: ext.version.clone(),
            entity_count: entity_counts.get(ext.name.as_str()).copied().unwrap_or(0),
            edge_count: edge_counts.get(ext.name.as_str()).copied().unwrap_or(0),
        })
        .collect()
}

pub fn filter_fields(model: &ModelIntermediate, level: FieldLevel) -> ModelIntermediate {
    let entities = model
        .entities
        .iter()
        .map(|entity| {
            let fields = match level {
                FieldLevel::None => Vec::new(),
                FieldLevel::Keys => entity
                    .fields
                    .iter()
                    .filter(|f| {
                        f.is_primary_key
                            || f.required
                            || matches!(
                                f.field_type,
                                ModelFieldType::Reference | ModelFieldType::ReferenceList
                            )
                    })
                    .cloned()
                    .collect(),
                FieldLevel::All => entity.fields.clone(),
            };
            ModelEntity {
                name: entity.name.clone(),
                extension: entity.extension.clone(),
                description: entity.description.clone(),
                fields,
                enhanced_by: entity.enhanced_by.clone(),
            }
        })
        .collect();

    ModelIntermediate {
        model_version: model.model_version.clone(),
        extensions: model.extensions.clone(),
        entities,
        relationships: model.relationships.clone(),
        edge_type_owners: model.edge_type_owners.clone(),
    }
}
