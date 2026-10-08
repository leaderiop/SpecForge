use insta::assert_snapshot;
use specforge_emitter::model::*;
use specforge_emitter::schema::*;
use specforge_registry::FieldType;

// =========================================================================
// Tracer bullet: types + Display impls
// =========================================================================

#[test]
fn cardinality_display_strings() {
    assert_eq!(Cardinality::OneToOne.to_string(), "1:1");
    assert_eq!(Cardinality::OneToMany.to_string(), "1:N");
    assert_eq!(Cardinality::ManyToOne.to_string(), "N:1");
    assert_eq!(Cardinality::ManyToMany.to_string(), "N:M");
}

#[test]
fn model_options_defaults() {
    let opts = ModelOptions::default();
    assert_eq!(opts.format, ModelFormat::Markdown);
    assert_eq!(opts.group_by, GroupBy::Extension);
    assert_eq!(opts.fields, FieldLevel::Keys);
    assert!(opts.extension_filter.is_none());
    assert!(opts.kind_filter.is_none());
    assert!(opts.root.is_none());
    assert!(opts.depth.is_none());
}

// =========================================================================
// Builder: empty schema -> empty IR
// =========================================================================

#[test]
fn empty_schema_produces_empty_model() {
    let schema = GraphProtocolSchema::empty();
    let model = ModelIntermediate_from_schema(&schema);

    assert_eq!(model.model_version, "1.0.0");
    assert!(model.extensions.is_empty());
    assert!(model.entities.is_empty());
    assert!(model.relationships.is_empty());
}

// =========================================================================
// Builder: single entity kind -> one ModelEntity with synthetic id field
// =========================================================================

#[test]
fn single_entity_kind_maps_to_model_entity_with_id() {
    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![SchemaExtensionInfo {
            name: "@specforge/software".to_string(),
            version: "1.0.0".to_string(),
        }],
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: vec![],
        }],
        edge_types: vec![],
    };

    let model = ModelIntermediate_from_schema(&schema);

    assert_eq!(model.entities.len(), 1);
    let entity = &model.entities[0];
    assert_eq!(entity.name, "behavior");
    assert_eq!(entity.extension, "@specforge/software");

    // Synthetic id field should be first
    assert!(!entity.fields.is_empty());
    let id_field = &entity.fields[0];
    assert_eq!(id_field.name, "id");
    assert_eq!(id_field.field_type, FieldType::String);
    assert!(id_field.required);
    assert!(id_field.is_primary_key);
}

// =========================================================================
// Builder: entity with schema fields -> correct field mapping
// =========================================================================

#[test]
fn entity_fields_mapped_from_schema() {
    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![SchemaExtensionInfo {
            name: "@specforge/software".to_string(),
            version: "1.0.0".to_string(),
        }],
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: vec![
                SchemaField {
                    name: "status".to_string(),
                    field_type: FieldType::Enum,
                    required: true,
                    enum_values: Some(vec!["draft".to_string(), "approved".to_string()]),
                    edge: None,
                    target_kind: None,
                    description: Some("Current status".to_string()),
                    default_value: None,
                    source_extension: "@specforge/software".to_string(),
                },
                SchemaField {
                    name: "features".to_string(),
                    field_type: FieldType::ReferenceList,
                    required: false,
                    enum_values: None,
                    edge: Some("BehaviorImplementsFeature".to_string()),
                    target_kind: Some("feature".to_string()),
                    description: None,
                    default_value: None,
                    source_extension: "@specforge/software".to_string(),
                },
            ],
        }],
        edge_types: vec![],
    };

    let model = ModelIntermediate_from_schema(&schema);

    let entity = &model.entities[0];
    // id + 2 schema fields = 3 fields
    assert_eq!(entity.fields.len(), 3);

    // First field is always the synthetic id
    assert_eq!(entity.fields[0].name, "id");
    assert!(entity.fields[0].is_primary_key);

    // Status field
    let status = &entity.fields[1];
    assert_eq!(status.name, "status");
    assert_eq!(status.field_type, FieldType::Enum);
    assert!(status.required);
    assert_eq!(
        status.enum_values,
        Some(vec!["draft".to_string(), "approved".to_string()])
    );
    assert_eq!(status.description, Some("Current status".to_string()));
    assert!(!status.is_primary_key);

    // Features field (reference_list -> has references)
    let features = &entity.fields[2];
    assert_eq!(features.name, "features");
    assert_eq!(features.field_type, FieldType::ReferenceList);
    assert!(!features.required);
    assert_eq!(features.references, Some("feature".to_string()));
}

// =========================================================================
// Builder: edge types -> ModelRelationship with cardinality
// =========================================================================

#[test]
fn edge_type_maps_to_relationship() {
    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![SchemaExtensionInfo {
            name: "@specforge/software".to_string(),
            version: "1.0.0".to_string(),
        }],
        entity_kinds: vec![
            SchemaEntityKind {
                name: "behavior".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: true,
                dot_color: None,
                fields: vec![SchemaField {
                    name: "features".to_string(),
                    field_type: FieldType::ReferenceList,
                    required: false,
                    enum_values: None,
                    edge: Some("BehaviorImplementsFeature".to_string()),
                    target_kind: Some("feature".to_string()),
                    description: None,
                    default_value: None,
                    source_extension: "@specforge/software".to_string(),
                }],
            },
            SchemaEntityKind {
                name: "feature".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: false,
                dot_color: None,
                fields: vec![],
            },
        ],
        edge_types: vec![SchemaEdgeType {
            label: "BehaviorImplementsFeature".to_string(),
            source_extension: "@specforge/software".to_string(),
            source_kinds: Some(vec!["behavior".to_string()]),
            target_kinds: Some(vec!["feature".to_string()]),
        }],
    };

    let model = ModelIntermediate_from_schema(&schema);

    assert_eq!(model.relationships.len(), 1);
    let rel = &model.relationships[0];
    assert_eq!(rel.name, "BehaviorImplementsFeature");
    assert_eq!(rel.source, "behavior");
    assert_eq!(rel.target, "feature");
    // reference_list field -> ManyToMany
    assert_eq!(rel.cardinality, Cardinality::ManyToMany);
    assert_eq!(rel.source_field, Some("features".to_string()));
}

// =========================================================================
// Builder: extension metadata counts
// =========================================================================

#[test]
fn extension_metadata_counts() {
    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![
            SchemaExtensionInfo {
                name: "@specforge/software".to_string(),
                version: "1.0.0".to_string(),
            },
            SchemaExtensionInfo {
                name: "@specforge/product".to_string(),
                version: "1.0.0".to_string(),
            },
        ],
        entity_kinds: vec![
            SchemaEntityKind {
                name: "behavior".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: true,
                dot_color: None,
                fields: vec![],
            },
            SchemaEntityKind {
                name: "event".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: false,
                dot_color: None,
                fields: vec![],
            },
            SchemaEntityKind {
                name: "feature".to_string(),
                source_extension: "@specforge/product".to_string(),
                testable: false,
                dot_color: None,
                fields: vec![],
            },
        ],
        edge_types: vec![
            SchemaEdgeType {
                label: "Triggers".to_string(),
                source_extension: "@specforge/software".to_string(),
                source_kinds: Some(vec!["behavior".to_string()]),
                target_kinds: Some(vec!["event".to_string()]),
            },
            SchemaEdgeType {
                label: "BehaviorImplementsFeature".to_string(),
                source_extension: "@specforge/software".to_string(),
                source_kinds: Some(vec!["behavior".to_string()]),
                target_kinds: Some(vec!["feature".to_string()]),
            },
        ],
    };

    let model = ModelIntermediate_from_schema(&schema);

    assert_eq!(model.extensions.len(), 2);

    let sw = model
        .extensions
        .iter()
        .find(|e| e.name == "@specforge/software")
        .unwrap();
    assert_eq!(sw.entity_count, 2);
    assert_eq!(sw.edge_count, 2);

    let prod = model
        .extensions
        .iter()
        .find(|e| e.name == "@specforge/product")
        .unwrap();
    assert_eq!(prod.entity_count, 1);
    assert_eq!(prod.edge_count, 0);
}

// =========================================================================
// Cardinality: reference field -> ManyToOne
// =========================================================================

#[test]
fn reference_field_infers_many_to_one() {
    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![SchemaExtensionInfo {
            name: "@specforge/software".to_string(),
            version: "1.0.0".to_string(),
        }],
        entity_kinds: vec![
            SchemaEntityKind {
                name: "behavior".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: true,
                dot_color: None,
                fields: vec![SchemaField {
                    name: "parent".to_string(),
                    field_type: FieldType::Reference,
                    required: false,
                    enum_values: None,
                    edge: Some("BelongsTo".to_string()),
                    target_kind: Some("feature".to_string()),
                    description: None,
                    default_value: None,
                    source_extension: "@specforge/software".to_string(),
                }],
            },
            SchemaEntityKind {
                name: "feature".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: false,
                dot_color: None,
                fields: vec![],
            },
        ],
        edge_types: vec![SchemaEdgeType {
            label: "BelongsTo".to_string(),
            source_extension: "@specforge/software".to_string(),
            source_kinds: Some(vec!["behavior".to_string()]),
            target_kinds: Some(vec!["feature".to_string()]),
        }],
    };

    let model = ModelIntermediate_from_schema(&schema);

    assert_eq!(model.relationships.len(), 1);
    assert_eq!(model.relationships[0].cardinality, Cardinality::ManyToOne);
    assert_eq!(
        model.relationships[0].source_field,
        Some("parent".to_string())
    );
}

// =========================================================================
// Cardinality: no matching field -> ManyToMany default
// =========================================================================

#[test]
fn no_matching_field_defaults_to_many_to_many() {
    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![SchemaExtensionInfo {
            name: "@specforge/software".to_string(),
            version: "1.0.0".to_string(),
        }],
        entity_kinds: vec![
            SchemaEntityKind {
                name: "behavior".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: true,
                dot_color: None,
                fields: vec![], // no fields at all
            },
            SchemaEntityKind {
                name: "event".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: false,
                dot_color: None,
                fields: vec![],
            },
        ],
        edge_types: vec![SchemaEdgeType {
            label: "Triggers".to_string(),
            source_extension: "@specforge/software".to_string(),
            source_kinds: Some(vec!["behavior".to_string()]),
            target_kinds: Some(vec!["event".to_string()]),
        }],
    };

    let model = ModelIntermediate_from_schema(&schema);

    assert_eq!(model.relationships.len(), 1);
    assert_eq!(model.relationships[0].cardinality, Cardinality::ManyToMany);
    assert!(model.relationships[0].source_field.is_none());
}

// =========================================================================
// Cardinality: edge with no source_kinds -> skipped
// =========================================================================

#[test]
fn edge_with_no_source_kinds_skipped() {
    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![],
        entity_kinds: vec![],
        edge_types: vec![SchemaEdgeType {
            label: "Phantom".to_string(),
            source_extension: "@specforge/software".to_string(),
            source_kinds: None,
            target_kinds: Some(vec!["feature".to_string()]),
        }],
    };

    let model = ModelIntermediate_from_schema(&schema);
    assert!(model.relationships.is_empty());
}

// =========================================================================
// Cardinality: reference (singular) field -> ManyToOne (term.module -> module)
// =========================================================================

#[test]
fn reference_singular_field_infers_many_to_one_for_term_module() {
    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![SchemaExtensionInfo {
            name: "@specforge/product".to_string(),
            version: "1.0.0".to_string(),
        }],
        entity_kinds: vec![
            SchemaEntityKind {
                name: "term".to_string(),
                source_extension: "@specforge/product".to_string(),
                testable: false,
                dot_color: None,
                fields: vec![SchemaField {
                    name: "module".to_string(),
                    field_type: FieldType::Reference,
                    required: false,
                    enum_values: None,
                    edge: Some("TermBelongsToModule".to_string()),
                    target_kind: Some("module".to_string()),
                    description: Some("The module that owns this term".to_string()),
                    default_value: None,
                    source_extension: "@specforge/product".to_string(),
                }],
            },
            SchemaEntityKind {
                name: "module".to_string(),
                source_extension: "@specforge/product".to_string(),
                testable: false,
                dot_color: None,
                fields: vec![],
            },
        ],
        edge_types: vec![SchemaEdgeType {
            label: "TermBelongsToModule".to_string(),
            source_extension: "@specforge/product".to_string(),
            source_kinds: Some(vec!["term".to_string()]),
            target_kinds: Some(vec!["module".to_string()]),
        }],
    };

    let model = ModelIntermediate_from_schema(&schema);

    assert_eq!(model.relationships.len(), 1);
    let rel = &model.relationships[0];
    assert_eq!(rel.name, "TermBelongsToModule");
    assert_eq!(rel.source, "term");
    assert_eq!(rel.target, "module");
    assert_eq!(rel.cardinality, Cardinality::ManyToOne);
    assert_eq!(rel.source_field, Some("module".to_string()));

    // term entity should have contribution info on the module field
    let term = model.entities.iter().find(|e| e.name == "term").unwrap();
    let module_field = term.fields.iter().find(|f| f.name == "module").unwrap();
    assert_eq!(
        module_field.contribution.as_deref(),
        Some("TermBelongsToModule -> module")
    );
}

// =========================================================================
// Helper: build a multi-extension model for filter tests
// =========================================================================

fn multi_extension_schema() -> GraphProtocolSchema {
    GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![
            SchemaExtensionInfo {
                name: "@specforge/software".to_string(),
                version: "1.0.0".to_string(),
            },
            SchemaExtensionInfo {
                name: "@specforge/product".to_string(),
                version: "1.0.0".to_string(),
            },
        ],
        entity_kinds: vec![
            SchemaEntityKind {
                name: "behavior".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: true,
                dot_color: None,
                fields: vec![
                    SchemaField {
                        name: "contract".to_string(),
                        field_type: FieldType::String,
                        required: true,
                        enum_values: None,
                        edge: None,
                        target_kind: None,
                        description: Some("The contract".to_string()),
                        default_value: None,
                        source_extension: "@specforge/software".to_string(),
                    },
                    SchemaField {
                        name: "features".to_string(),
                        field_type: FieldType::ReferenceList,
                        required: false,
                        enum_values: None,
                        edge: Some("BehaviorImplementsFeature".to_string()),
                        target_kind: Some("feature".to_string()),
                        description: None,
                        default_value: None,
                        source_extension: "@specforge/software".to_string(),
                    },
                ],
            },
            SchemaEntityKind {
                name: "event".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: false,
                dot_color: None,
                fields: vec![],
            },
            SchemaEntityKind {
                name: "feature".to_string(),
                source_extension: "@specforge/product".to_string(),
                testable: false,
                dot_color: None,
                fields: vec![SchemaField {
                    name: "priority".to_string(),
                    field_type: FieldType::Enum,
                    required: false,
                    enum_values: Some(vec!["low".into(), "medium".into(), "high".into()]),
                    edge: None,
                    target_kind: None,
                    description: None,
                    default_value: None,
                    source_extension: "@specforge/product".to_string(),
                }],
            },
            SchemaEntityKind {
                name: "journey".to_string(),
                source_extension: "@specforge/product".to_string(),
                testable: false,
                dot_color: None,
                fields: vec![],
            },
        ],
        edge_types: vec![
            SchemaEdgeType {
                label: "BehaviorImplementsFeature".to_string(),
                source_extension: "@specforge/software".to_string(),
                source_kinds: Some(vec!["behavior".to_string()]),
                target_kinds: Some(vec!["feature".to_string()]),
            },
            SchemaEdgeType {
                label: "Triggers".to_string(),
                source_extension: "@specforge/software".to_string(),
                source_kinds: Some(vec!["behavior".to_string()]),
                target_kinds: Some(vec!["event".to_string()]),
            },
            SchemaEdgeType {
                label: "JourneyExercisesFeature".to_string(),
                source_extension: "@specforge/product".to_string(),
                source_kinds: Some(vec!["journey".to_string()]),
                target_kinds: Some(vec!["feature".to_string()]),
            },
        ],
    }
}

// =========================================================================
// Filter: extension filter keeps only matching entities
// =========================================================================

#[test]
fn filter_by_extension() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);
    let opts = ModelOptions {
        extension_filter: Some("@specforge/software".to_string()),
        ..ModelOptions::default()
    };

    let filtered = filter_entities(&model, &opts);

    let names: Vec<&str> = filtered.entities.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["behavior", "event"]);
    // Implements goes behavior->feature, but feature is filtered out, so pruned
    // Triggers stays (behavior->event, both in software)
    assert_eq!(filtered.relationships.len(), 1);
    assert_eq!(filtered.relationships[0].name, "Triggers");
}

// =========================================================================
// Filter: kind filter with known kinds
// =========================================================================

#[test]
fn filter_by_kinds() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);
    let opts = ModelOptions {
        kind_filter: Some(vec!["behavior".to_string(), "feature".to_string()]),
        ..ModelOptions::default()
    };

    let filtered = filter_entities(&model, &opts);

    let names: Vec<&str> = filtered.entities.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["behavior", "feature"]);
    // Implements stays (behavior->feature), Triggers pruned (event not in filter)
    assert_eq!(filtered.relationships.len(), 1);
    assert_eq!(filtered.relationships[0].name, "BehaviorImplementsFeature");
}

// =========================================================================
// Filter: unknown kind silently ignored
// =========================================================================

#[test]
fn filter_unknown_kind_ignored() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);
    let opts = ModelOptions {
        kind_filter: Some(vec!["nonexistent".to_string()]),
        ..ModelOptions::default()
    };

    let filtered = filter_entities(&model, &opts);
    assert!(filtered.entities.is_empty());
    assert!(filtered.relationships.is_empty());
}

// =========================================================================
// Filter: root+depth=0 keeps only root
// =========================================================================

#[test]
fn filter_root_depth_zero() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);
    let opts = ModelOptions {
        root: Some("behavior".to_string()),
        depth: Some(0),
        ..ModelOptions::default()
    };

    let filtered = filter_entities(&model, &opts);

    let names: Vec<&str> = filtered.entities.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["behavior"]);
    assert!(filtered.relationships.is_empty());
}

// =========================================================================
// Filter: root+depth=1 keeps root + direct neighbors
// =========================================================================

#[test]
fn filter_root_depth_one() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);
    let opts = ModelOptions {
        root: Some("behavior".to_string()),
        depth: Some(1),
        ..ModelOptions::default()
    };

    let filtered = filter_entities(&model, &opts);

    let mut names: Vec<&str> = filtered.entities.iter().map(|e| e.name.as_str()).collect();
    names.sort();
    // behavior is root, direct neighbors via edges: event (Triggers), feature (Implements)
    assert_eq!(names, vec!["behavior", "event", "feature"]);
}

// =========================================================================
// Filter: intersection of extension + kind filter
// =========================================================================

#[test]
fn filter_intersection_extension_and_kind() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);
    let opts = ModelOptions {
        extension_filter: Some("@specforge/software".to_string()),
        kind_filter: Some(vec!["behavior".to_string()]),
        ..ModelOptions::default()
    };

    let filtered = filter_entities(&model, &opts);

    let names: Vec<&str> = filtered.entities.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["behavior"]);
}

// =========================================================================
// Filter: field level none -> empty fields
// =========================================================================

#[test]
fn filter_fields_none() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);

    let filtered = filter_fields(&model, FieldLevel::None);

    for entity in &filtered.entities {
        assert!(
            entity.fields.is_empty(),
            "entity {} should have no fields",
            entity.name
        );
    }
}

// =========================================================================
// Filter: field level keys -> pk + required + refs
// =========================================================================

#[test]
fn filter_fields_keys() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);

    let filtered = filter_fields(&model, FieldLevel::Keys);

    let behavior = filtered
        .entities
        .iter()
        .find(|e| e.name == "behavior")
        .unwrap();
    let field_names: Vec<&str> = behavior.fields.iter().map(|f| f.name.as_str()).collect();
    // id (pk) + contract (required) + features (reference_list)
    assert_eq!(field_names, vec!["id", "contract", "features"]);

    let feature = filtered
        .entities
        .iter()
        .find(|e| e.name == "feature")
        .unwrap();
    let field_names: Vec<&str> = feature.fields.iter().map(|f| f.name.as_str()).collect();
    // id (pk) only — priority is not required and not a reference
    assert_eq!(field_names, vec!["id"]);
}

// =========================================================================
// Filter: field level all -> all fields unchanged
// =========================================================================

#[test]
fn filter_fields_all() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);

    let filtered = filter_fields(&model, FieldLevel::All);

    let behavior = filtered
        .entities
        .iter()
        .find(|e| e.name == "behavior")
        .unwrap();
    assert_eq!(behavior.fields.len(), 3); // id + contract + features

    let feature = filtered
        .entities
        .iter()
        .find(|e| e.name == "feature")
        .unwrap();
    assert_eq!(feature.fields.len(), 2); // id + priority
}

// =========================================================================
// Renderer tests — use multi_extension_schema for all snapshots
// =========================================================================

fn build_model_keys() -> ModelIntermediate {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);
    filter_fields(&model, FieldLevel::Keys)
}

fn default_options(format: ModelFormat) -> ModelOptions {
    ModelOptions {
        format,
        ..ModelOptions::default()
    }
}

// --- Markdown ---

#[test]
fn render_markdown_keys_grouped() {
    let model = build_model_keys();
    let output = render(&model, &default_options(ModelFormat::Markdown));
    assert_snapshot!(output);
}

#[test]
fn render_markdown_none_fields() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);
    let model = filter_fields(&model, FieldLevel::None);
    let output = render(&model, &default_options(ModelFormat::Markdown));
    assert_snapshot!(output);
}

#[test]
fn render_markdown_flat() {
    let model = build_model_keys();
    let opts = ModelOptions {
        format: ModelFormat::Markdown,
        group_by: GroupBy::None,
        ..ModelOptions::default()
    };
    let output = render(&model, &opts);
    assert_snapshot!(output);
}

#[test]
fn render_markdown_empty() {
    let schema = GraphProtocolSchema::empty();
    let model = ModelIntermediate_from_schema(&schema);
    let output = render(&model, &default_options(ModelFormat::Markdown));
    assert_snapshot!(output);
}

// --- Mermaid ---

#[test]
fn render_mermaid_keys_grouped() {
    let model = build_model_keys();
    let output = render(&model, &default_options(ModelFormat::Mermaid));
    assert_snapshot!(output);
}

#[test]
fn render_mermaid_none_fields() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);
    let model = filter_fields(&model, FieldLevel::None);
    let output = render(&model, &default_options(ModelFormat::Mermaid));
    assert_snapshot!(output);
}

#[test]
fn render_mermaid_empty() {
    let schema = GraphProtocolSchema::empty();
    let model = ModelIntermediate_from_schema(&schema);
    let output = render(&model, &default_options(ModelFormat::Mermaid));
    assert_snapshot!(output);
}

// --- DOT ---

/// The model with the `theme_color`s the builtin manifests declare.
fn themed(mut model: ModelIntermediate) -> ModelIntermediate {
    for ext in &mut model.extensions {
        ext.color = match ext.name.as_str() {
            "@specforge/software" => Some("#4a90d9".to_string()),
            "@specforge/product" => Some("#2ecc71".to_string()),
            "@specforge/governance" => Some("#e74c3c".to_string()),
            "@specforge/formal" => Some("#9b59b6".to_string()),
            _ => None,
        };
    }
    model
}

#[test]
fn render_dot_keys_grouped() {
    let model = themed(build_model_keys());
    let output = render(&model, &default_options(ModelFormat::Dot));
    assert_snapshot!(output);
}

#[test]
fn render_dot_none_fields() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);
    let model = themed(filter_fields(&model, FieldLevel::None));
    let output = render(&model, &default_options(ModelFormat::Dot));
    assert_snapshot!(output);
}

#[test]
fn render_dot_empty() {
    let schema = GraphProtocolSchema::empty();
    let model = ModelIntermediate_from_schema(&schema);
    let output = render(&model, &default_options(ModelFormat::Dot));
    assert_snapshot!(output);
}

// --- JSON ---

#[test]
fn render_json_keys() {
    let model = build_model_keys();
    let output = render(&model, &default_options(ModelFormat::Json));
    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");
    assert_eq!(parsed["model_version"], "1.0.0");
    assert!(parsed["entities"].is_array());
    assert!(parsed["relationships"].is_array());
    assert_snapshot!(output);
}

#[test]
fn render_json_cardinality_strings() {
    let model = build_model_keys();
    let output = render(&model, &default_options(ModelFormat::Json));
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");
    for rel in parsed["relationships"].as_array().unwrap() {
        let card = rel["cardinality"].as_str().unwrap();
        assert!(
            ["1:1", "1:N", "N:1", "N:M"].contains(&card),
            "unexpected cardinality: {}",
            card
        );
    }
}

#[test]
fn render_json_empty() {
    let schema = GraphProtocolSchema::empty();
    let model = ModelIntermediate_from_schema(&schema);
    let output = render(&model, &default_options(ModelFormat::Json));
    assert_snapshot!(output);
}

// --- DBML ---

#[test]
fn render_dbml_keys_grouped() {
    let model = build_model_keys();
    let output = render(&model, &default_options(ModelFormat::Dbml));
    assert_snapshot!(output);
}

#[test]
fn render_dbml_empty() {
    let schema = GraphProtocolSchema::empty();
    let model = ModelIntermediate_from_schema(&schema);
    let output = render(&model, &default_options(ModelFormat::Dbml));
    assert_snapshot!(output);
}

// C13-03: registry-declared dot_color wins over the extension palette.
#[test]
fn declared_dot_color_reaches_model_dot() {
    let mut schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![SchemaExtensionInfo {
            name: "@specforge/software".to_string(),
            version: "1.0.0".to_string(),
        }],
        entity_kinds: vec![
            SchemaEntityKind {
                name: "behavior".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: true,
                dot_color: Some("#123456".to_string()),
                fields: vec![],
            },
            SchemaEntityKind {
                name: "event".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: false,
                dot_color: None,
                fields: vec![],
            },
        ],
        edge_types: vec![],
    };
    schema.extensions = vec![SchemaExtensionInfo {
        name: "@specforge/software".to_string(),
        version: "1.0.0".to_string(),
    }];

    let model = themed(ModelIntermediate_from_schema(&schema));
    let output = render(&model, &default_options(ModelFormat::Dot));

    assert!(
        output.contains("#123456"),
        "declared per-kind color emitted: {output}"
    );
    let software_palette = "#4a90d9";
    assert!(
        output.contains(software_palette),
        "undeclared kinds keep the extension's theme colour: {output}"
    );
}

// C13-10: DBML renders real column types and cardinality-matching Ref
// operators instead of all-string columns and hardcoded `>`.
#[test]
fn dbml_maps_real_types_and_cardinality_operators() {
    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![SchemaExtensionInfo {
            name: "@specforge/software".to_string(),
            version: "1.0.0".to_string(),
        }],
        entity_kinds: vec![
            SchemaEntityKind {
                name: "behavior".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: true,
                dot_color: None,
                fields: vec![
                    SchemaField {
                        name: "count".to_string(),
                        field_type: FieldType::Integer,
                        required: false,
                        enum_values: None,
                        edge: None,
                        target_kind: None,
                        description: None,
                        default_value: None,
                        source_extension: "@specforge/software".to_string(),
                    },
                    SchemaField {
                        name: "flag".to_string(),
                        field_type: FieldType::Bool,
                        required: false,
                        enum_values: None,
                        edge: None,
                        target_kind: None,
                        description: None,
                        default_value: None,
                        source_extension: "@specforge/software".to_string(),
                    },
                    SchemaField {
                        name: "tags".to_string(),
                        field_type: FieldType::StringList,
                        required: false,
                        enum_values: None,
                        edge: None,
                        target_kind: None,
                        description: None,
                        default_value: None,
                        source_extension: "@specforge/software".to_string(),
                    },
                    SchemaField {
                        name: "body".to_string(),
                        field_type: FieldType::Block,
                        required: false,
                        enum_values: None,
                        edge: None,
                        target_kind: None,
                        description: None,
                        default_value: None,
                        source_extension: "@specforge/software".to_string(),
                    },
                    SchemaField {
                        name: "features".to_string(),
                        field_type: FieldType::ReferenceList,
                        required: false,
                        enum_values: None,
                        edge: Some("BehaviorImplementsFeature".to_string()),
                        target_kind: Some("feature".to_string()),
                        description: None,
                        default_value: None,
                        source_extension: "@specforge/software".to_string(),
                    },
                ],
            },
            SchemaEntityKind {
                name: "feature".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: false,
                dot_color: None,
                fields: vec![],
            },
        ],
        edge_types: vec![SchemaEdgeType {
            label: "BehaviorImplementsFeature".to_string(),
            source_extension: "@specforge/software".to_string(),
            source_kinds: Some(vec!["behavior".to_string()]),
            target_kinds: Some(vec!["feature".to_string()]),
        }],
    };

    let model = ModelIntermediate_from_schema(&schema);
    let output = render(&model, &default_options(ModelFormat::Dbml));

    assert!(output.contains("count integer"), "integer mapped: {output}");
    assert!(output.contains("flag boolean"), "boolean mapped: {output}");
    assert!(output.contains("tags text"), "string_list mapped: {output}");
    assert!(output.contains("body json"), "block mapped: {output}");
    // reference_list -> ManyToMany -> `<>`, target column is the PK.
    assert!(
        output.contains("Ref BehaviorImplementsFeature: behavior.features <> feature.id"),
        "cardinality operator and PK column: {output}"
    );
}

#[test]
fn an_extension_without_a_theme_color_is_drawn_grey() {
    let model = ModelIntermediate_from_schema(&multi_extension_schema());
    let output = render(&model, &default_options(ModelFormat::Dot));
    assert!(output.contains("color=\"#95a5a6\";"), "{output}");
    assert!(!output.contains("#4a90d9"), "no palette by name: {output}");
}

// =========================================================================
// Declared text in the text diagrams (plan 16)
// =========================================================================

/// `multi_extension_schema()` with `behavior` renamed to `kind` and its
/// `contract` field's description replaced.
fn with_text(kind: &str, description: &str) -> GraphProtocolSchema {
    let mut schema = multi_extension_schema();
    for entity in &mut schema.entity_kinds {
        if entity.name == "behavior" {
            entity.name = kind.to_string();
            for field in &mut entity.fields {
                if field.name == "contract" {
                    field.description = Some(description.to_string());
                }
            }
        }
    }
    for edge in &mut schema.edge_types {
        if let Some(sources) = &mut edge.source_kinds {
            for source in sources.iter_mut().filter(|s| *s == "behavior") {
                *source = kind.to_string();
            }
        }
    }
    schema
}

fn rendered(schema: &GraphProtocolSchema, fields: FieldLevel, format: ModelFormat) -> String {
    let model = filter_fields(&ModelIntermediate_from_schema(schema), fields);
    render(&model, &default_options(format))
}

// pin (16-T0): today's behaviour; flipped by 16-T8
#[test]
fn pin_a_quote_in_a_description_is_written_as_rust_escape_text() {
    let schema = with_text("behavior", "the \"body\"\nnext");
    let output = rendered(&schema, FieldLevel::All, ModelFormat::Mermaid);
    assert!(
        output.contains("\"the \\u{0022}\\u{0022}body\\u{0022}\\u{0022}\nnext\""),
        "{output}"
    );
}

// pin (16-T0): today's behaviour; flipped by 16-T8
#[test]
fn pin_a_kind_name_is_written_bare_in_the_er_diagram() {
    let schema = with_text("no\"te", "the contract");
    let output = rendered(&schema, FieldLevel::All, ModelFormat::Mermaid);
    assert!(output.contains("    no\"te {"), "{output}");
}

// pin (16-T0): today's behaviour; flipped by 16-T9
#[test]
fn pin_an_apostrophe_ends_a_dbml_note() {
    let schema = with_text("behavior", "this event's shape");
    let output = rendered(&schema, FieldLevel::All, ModelFormat::Dbml);
    assert!(output.contains("note: 'this event's shape'"), "{output}");
}

// pin (16-T0): today's behaviour; flipped by 16-T9
#[test]
fn pin_a_reference_is_written_inline_and_as_a_named_ref() {
    let output = rendered(
        &multi_extension_schema(),
        FieldLevel::All,
        ModelFormat::Dbml,
    );
    assert!(
        output.contains(
            "features text [ref: > feature.id, note: 'BehaviorImplementsFeature -> feature']"
        ),
        "{output}"
    );
    assert!(
        output.contains("Ref BehaviorImplementsFeature: behavior.features <> feature.id"),
        "{output}"
    );
}

// pin (16-T0): today's behaviour; flipped by 16-T9
#[test]
fn pin_a_ref_names_a_column_the_table_does_not_write() {
    let output = rendered(
        &multi_extension_schema(),
        FieldLevel::None,
        ModelFormat::Dbml,
    );
    let table = output.split("Table behavior {").nth(1).expect("a table");
    assert!(table.trim_start().starts_with('}'), "{output}");
    assert!(
        output.contains("Ref BehaviorImplementsFeature: behavior.features <> feature.id"),
        "{output}"
    );
}

// pin (16-T0): today's behaviour; flipped by 16-T9
#[test]
fn pin_an_extension_name_is_a_bare_dbml_table_group() {
    let mut schema = multi_extension_schema();
    for info in &mut schema.extensions {
        if info.name == "@specforge/software" {
            info.name = "@acme/x".to_string();
        }
    }
    for entity in &mut schema.entity_kinds {
        if entity.source_extension == "@specforge/software" {
            entity.source_extension = "@acme/x".to_string();
        }
    }
    let output = rendered(&schema, FieldLevel::Keys, ModelFormat::Dbml);
    assert!(output.contains("TableGroup @acme/x {"), "{output}");
}
