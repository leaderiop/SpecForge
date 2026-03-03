use crate::file_index::FileIndex;
use crate::spec_graph::{GraphEdge, GraphNode, SpecGraph};
use specforge_common::{EdgeType, FieldLookup, FieldRegistry, FieldValue};
use specforge_parser::{AstEntity, SpecFile};

/// Result of building the spec graph from parsed files.
pub struct GraphBuildResult {
    pub graph: SpecGraph,
    pub file_index: FileIndex,
}

/// Build a `SpecGraph` and `FileIndex` from resolved spec files.
///
/// Steps:
/// 1. Create a node for each entity
/// 2. Register entity-to-file mapping
/// 3. Walk fields, mapping reference fields to typed edges via `FieldRegistry::lookup`
pub fn build_graph(files: &[SpecFile], registry: &FieldRegistry) -> GraphBuildResult {
    let mut graph = SpecGraph::new();
    let mut file_index = FileIndex::new();

    // Step 1 & 2: Add all nodes first
    for file in files {
        for entity in &file.entities {
            let node = entity_to_node(entity, &file.path);
            graph.add_node(node);
            file_index.register(entity.id.raw(), &file.path);
        }
    }

    // Step 3: Add edges from reference fields
    for file in files {
        for entity in &file.entities {
            add_edges_for_entity(entity, &mut graph, registry);
        }
    }

    GraphBuildResult { graph, file_index }
}

fn entity_to_node(entity: &AstEntity, file: &str) -> GraphNode {
    GraphNode {
        id: entity.id.clone(),
        kind: entity.kind.clone(),
        title: entity.title.clone(),
        file: file.to_string(),
        span: entity.span.clone(),
    }
}

fn add_edges_for_entity(entity: &AstEntity, graph: &mut SpecGraph, registry: &FieldRegistry) {
    let from_id = entity.id.raw();
    let entity_kind = entity.kind.keyword();

    for (field_name, value) in entity.fields.iter() {
        match registry.lookup(entity_kind, field_name) {
            Some(FieldLookup::Builtin(edge_type)) => {
                add_ref_edges(from_id, value, edge_type, field_name, None, graph);
            }
            Some(FieldLookup::Enhanced(enh)) if enh.enhancement.field_type.is_reference() => {
                let label = enh.enhancement.field_type.edge_label().map(|s| s.to_string());
                add_ref_edges(from_id, value, EdgeType::Enhanced, field_name, label, graph);
            }
            None => {
                // Not a reference field — check nested blocks for references too
                if let FieldValue::Block(block) = value {
                    for (sub_field, sub_value) in block.iter() {
                        match registry.lookup(entity_kind, sub_field) {
                            Some(FieldLookup::Builtin(sub_edge_type)) => {
                                add_ref_edges(from_id, sub_value, sub_edge_type, sub_field, None, graph);
                            }
                            Some(FieldLookup::Enhanced(sub_enh))
                                if sub_enh.enhancement.field_type.is_reference() =>
                            {
                                let label = sub_enh.enhancement.field_type.edge_label().map(|s| s.to_string());
                                add_ref_edges(from_id, sub_value, EdgeType::Enhanced, sub_field, label, graph);
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {} // Enhanced non-reference field — no edge
        }
    }
}

fn add_ref_edges(
    from_id: &str,
    value: &FieldValue,
    edge_type: EdgeType,
    field_name: &str,
    enhanced_label: Option<String>,
    graph: &mut SpecGraph,
) {
    match value {
        FieldValue::Reference(ref_id) => {
            graph.add_edge(
                from_id,
                ref_id.raw(),
                GraphEdge {
                    edge_type,
                    field_name: field_name.to_string(),
                    enhanced_label: enhanced_label.clone(),
                },
            );
        }
        FieldValue::ReferenceList(refs) => {
            for ref_id in refs {
                graph.add_edge(
                    from_id,
                    ref_id.raw(),
                    GraphEdge {
                        edge_type,
                        field_name: field_name.to_string(),
                        enhanced_label: enhanced_label.clone(),
                    },
                );
            }
        }
        FieldValue::Enum(name) => {
            graph.add_edge(
                from_id,
                name,
                GraphEdge {
                    edge_type,
                    field_name: field_name.to_string(),
                    enhanced_label: enhanced_label.clone(),
                },
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::{
        EnhancedFieldType, EnhancementPolicy, EntityId, EntityKind, FieldEnhancement, FieldMap,
        FieldRegistry, SourceSpan,
    };
    use std::collections::HashMap;

    fn make_file(path: &str, entities: Vec<AstEntity>) -> SpecFile {
        SpecFile {
            path: path.to_string(),
            imports: Vec::new(),
            entities,
            custom_defs: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn make_entity(id: &str, kind: EntityKind, fields: FieldMap) -> AstEntity {
        AstEntity {
            kind,
            id: EntityId::parse(id),
            title: Some(format!("Test {id}")),
            fields,
            span: SourceSpan::new("test.spec", 1, 1, 1, 1),
        }
    }

    #[test]
    fn build_simple_graph() {
        let mut fields = FieldMap::new();
        fields.insert(
            "invariants",
            FieldValue::ReferenceList(vec![EntityId::parse("INV-SF-1")]),
        );
        fields.insert("contract", FieldValue::String("must work".to_string()));

        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("INV-SF-1", EntityKind::Invariant, FieldMap::new()),
                make_entity("BEH-SF-1", EntityKind::Behavior, fields),
            ],
        )];

        let registry = FieldRegistry::with_builtins();
        let result = build_graph(&files, &registry);
        assert_eq!(result.graph.node_count(), 2);
        assert_eq!(result.graph.edge_count(), 1);

        let edges: Vec<_> = result.graph.edges().collect();
        assert_eq!(edges[0].2.edge_type, EdgeType::References);
    }

    #[test]
    fn file_index_populated() {
        let files = vec![
            make_file(
                "invariants.spec",
                vec![make_entity(
                    "INV-SF-1",
                    EntityKind::Invariant,
                    FieldMap::new(),
                )],
            ),
            make_file(
                "behaviors.spec",
                vec![make_entity(
                    "BEH-SF-1",
                    EntityKind::Behavior,
                    FieldMap::new(),
                )],
            ),
        ];

        let registry = FieldRegistry::with_builtins();
        let result = build_graph(&files, &registry);
        assert_eq!(result.file_index.entity_count(), 2);
        assert_eq!(result.file_index.file_of("INV-SF-1"), Some("invariants.spec"));
        assert_eq!(result.file_index.file_of("BEH-SF-1"), Some("behaviors.spec"));
    }

    #[test]
    fn multiple_edge_types() {
        let mut beh_fields = FieldMap::new();
        beh_fields.insert(
            "invariants",
            FieldValue::ReferenceList(vec![EntityId::parse("INV-SF-1")]),
        );

        let mut feat_fields = FieldMap::new();
        feat_fields.insert(
            "behaviors",
            FieldValue::ReferenceList(vec![EntityId::parse("BEH-SF-1")]),
        );

        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("INV-SF-1", EntityKind::Invariant, FieldMap::new()),
                make_entity("BEH-SF-1", EntityKind::Behavior, beh_fields),
                make_entity("FEAT-SF-1", EntityKind::Feature, feat_fields),
            ],
        )];

        let registry = FieldRegistry::with_builtins();
        let result = build_graph(&files, &registry);
        assert_eq!(result.graph.node_count(), 3);
        assert_eq!(result.graph.edge_count(), 2);

        // BEH-SF-1 -> INV-SF-1 (references)
        let beh_out = result.graph.outgoing_edges("BEH-SF-1");
        assert_eq!(beh_out.len(), 1);
        assert_eq!(beh_out[0].1.edge_type, EdgeType::References);

        // FEAT-SF-1 -> BEH-SF-1 (implements)
        let feat_out = result.graph.outgoing_edges("FEAT-SF-1");
        assert_eq!(feat_out.len(), 1);
        assert_eq!(feat_out[0].1.edge_type, EdgeType::Implements);
    }

    #[test]
    fn orphan_detection_through_builder() {
        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("INV-SF-1", EntityKind::Invariant, FieldMap::new()),
                make_entity("INV-SF-2", EntityKind::Invariant, FieldMap::new()),
            ],
        )];

        let registry = FieldRegistry::with_builtins();
        let result = build_graph(&files, &registry);
        let orphans = result.graph.orphans();
        assert_eq!(orphans.len(), 2);
    }

    #[test]
    fn enforced_by_creates_enforces_edges() {
        let mut inv_fields = FieldMap::new();
        inv_fields.insert(
            "enforced_by",
            FieldValue::ReferenceList(vec![EntityId::parse("validate_input")]),
        );
        inv_fields.insert("guarantee", FieldValue::String("must hold".to_string()));

        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("validate_input", EntityKind::Behavior, FieldMap::new()),
                make_entity("data_persistence", EntityKind::Invariant, inv_fields),
            ],
        )];

        let registry = FieldRegistry::with_builtins();
        let result = build_graph(&files, &registry);
        assert_eq!(result.graph.node_count(), 2);
        assert_eq!(result.graph.edge_count(), 1);

        let inv_out = result.graph.outgoing_edges("data_persistence");
        assert_eq!(inv_out.len(), 1);
        assert_eq!(inv_out[0].1.edge_type, EdgeType::Enforces);
        assert_eq!(inv_out[0].0.id.raw(), "validate_input");
    }

    #[test]
    fn dangling_ref_skipped() {
        let mut fields = FieldMap::new();
        fields.insert(
            "invariants",
            FieldValue::ReferenceList(vec![EntityId::parse("INV-SF-99")]),
        );

        let files = vec![make_file(
            "test.spec",
            vec![make_entity("BEH-SF-1", EntityKind::Behavior, fields)],
        )];

        let registry = FieldRegistry::with_builtins();
        let result = build_graph(&files, &registry);
        // INV-SF-99 doesn't exist as a node, so edge creation returns false
        assert_eq!(result.graph.node_count(), 1);
        assert_eq!(result.graph.edge_count(), 0);
    }

    #[test]
    fn enhanced_reference_field_creates_enhanced_edge() {
        let mut registry = FieldRegistry::with_builtins();
        let enhancements = vec![FieldEnhancement {
            target_entity: "behavior".to_string(),
            field_name: "adapter_ref".to_string(),
            field_type: EnhancedFieldType::Reference {
                edge_label: "adapts".to_string(),
                target_kind: None,
            },
            required: false,
            description: String::new(),
        }];
        registry.register_plugin("@test/hex", &enhancements, &[], &EnhancementPolicy::default(), &HashMap::new());

        let mut fields = FieldMap::new();
        fields.insert(
            "adapter_ref",
            FieldValue::Reference(EntityId::parse("some_port")),
        );

        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("my_beh", EntityKind::Behavior, fields),
                make_entity("some_port", EntityKind::Port, FieldMap::new()),
            ],
        )];

        let result = build_graph(&files, &registry);
        let edges = result.graph.outgoing_edges("my_beh");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].1.edge_type, EdgeType::Enhanced);
        assert_eq!(
            edges[0].1.enhanced_label,
            Some("adapts".to_string())
        );
    }
}
