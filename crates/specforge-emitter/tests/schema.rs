use specforge_common::{SourceSpan, Sym};
use specforge_emitter::{
    compute_schema_version, detect_breaking_with_diagnostics, diff_schemas,
    diff_schemas_optional, emit_brief_scoped_with_schema, emit_brief_with_schema,
    emit_context_scoped_with_schema, emit_context_with_schema,
    emit_json, emit_json_scoped_with_schema, emit_json_with_schema, emit_schema,
    emit_schema_for_kind, generate_schema, load_schema_cache, negotiate_version,
    negotiate_version_or_latest, persist_schema_cache, publish_json_schema,
    GraphProtocolSchema, SchemaCacheEntry, SchemaEdgeType, SchemaEntityKind,
    SchemaExtensionInfo, SchemaField, SchemaMigration, SchemaMigrationChange,
    SchemaVersion, SchemaVersionError,
};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_registry::{
    EdgeRegistry, EdgeRegistryEntry, FieldRegistry, FieldRegistryEntry, KindRegistry,
    KindRegistryEntry, ManifestFieldType,
};
use specforge_test::prelude::*;

fn span() -> SourceSpan {
    SourceSpan {
        file: Sym::new("test.spec"),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

fn node(id: &str, kind: &str, title: Option<&str>) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: title.map(|s| s.to_string()),
        fields: FieldMap::new(),
        source_span: span(),
    }
}

fn make_kind_entry(name: &str, ext: &str, testable: bool) -> KindRegistryEntry {
    KindRegistryEntry {
        kind_name: name.to_string(),
        description: None,
        source_extension: ext.to_string(),
        testable,
        singleton: false,
        supports_verify: testable,
        allowed_verify_kinds: vec![],
        semantic_token: None,
        lsp_icon: None,
        dot_shape: None,
        dot_color: None,
        dot_fillcolor: None,
        open_fields: false,
    }
}

fn make_edge_entry(label: &str, ext: &str, src: Option<&str>, tgt: Option<&str>) -> EdgeRegistryEntry {
    EdgeRegistryEntry {
        label: label.to_string(),
        source_kind: src.map(|s| s.to_string()),
        target_kind: tgt.map(|s| s.to_string()),
        source_extension: ext.to_string(),
        edge_style: None,
        edge_color: None,
        edge_arrowhead: None,
    }
}

fn make_field_entry(kind: &str, field: &str, ft: ManifestFieldType, required: bool) -> FieldRegistryEntry {
    FieldRegistryEntry {
        kind_name: kind.to_string(),
        field_name: field.to_string(),
        description: None,
        field_type: ft,
        source_extension: "@specforge/software".to_string(),
        edge: None,
        target_kind: None,
        file_reference: false,
        required,
    }
}

fn sample_schema() -> GraphProtocolSchema {
    GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 2, 3),
        extensions: vec![SchemaExtensionInfo {
            name: "@specforge/software".to_string(),
            version: "1.0.0".to_string(),
        }],
        entity_kinds: vec![
            SchemaEntityKind {
                name: "behavior".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: true,
                fields: vec![SchemaField {
                    name: "contract".to_string(),
                    field_type: "string".to_string(),
                    required: false,
                    enum_values: None,
                    edge: None,
                    target_kind: None,
                    description: None,
                    default_value: None,
                    source_extension: "@specforge/software".to_string(),
                }],
            },
            SchemaEntityKind {
                name: "feature".to_string(),
                source_extension: "@specforge/product".to_string(),
                testable: false,
                fields: vec![],
            },
        ],
        edge_types: vec![SchemaEdgeType {
            label: "implements".to_string(),
            source_extension: "@specforge/software".to_string(),
            source_kinds: Some(vec!["behavior".to_string()]),
            target_kinds: Some(vec!["feature".to_string()]),
        }],
    }
}

// ===========================================================================
// Slice 1: Schema Types
// ===========================================================================

// B:generate_schema_from_registries — verify unit "SchemaVersion Display produces MAJOR.MINOR.PATCH"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "SchemaVersion Display")]
fn schema_version_display() {
    assert_eq!(SchemaVersion::new(1, 0, 0).to_string(), "1.0.0");
    assert_eq!(SchemaVersion::new(2, 3, 4).to_string(), "2.3.4");
    let mut v = SchemaVersion::new(1, 0, 0);
    v.label = Some("beta".to_string());
    assert_eq!(v.to_string(), "1.0.0-beta");
}

// B:generate_schema_from_registries — verify unit "SchemaVersion FromStr round-trips"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "SchemaVersion FromStr")]
fn schema_version_from_str() {
    let v: SchemaVersion = "1.2.3".parse().unwrap();
    assert_eq!(v, SchemaVersion::new(1, 2, 3));
    assert!(v.label.is_none());

    let v2: SchemaVersion = "2.0.0-alpha".parse().unwrap();
    assert_eq!(v2.major, 2);
    assert_eq!(v2.label, Some("alpha".to_string()));

    assert!("invalid".parse::<SchemaVersion>().is_err());
    assert!("1.2".parse::<SchemaVersion>().is_err());
}

// B:generate_schema_from_registries — verify unit "SchemaVersion Ord compares correctly"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "SchemaVersion Ord")]
fn schema_version_ord() {
    let v1 = SchemaVersion::new(1, 0, 0);
    let v2 = SchemaVersion::new(1, 1, 0);
    let v3 = SchemaVersion::new(2, 0, 0);
    assert!(v1 < v2);
    assert!(v2 < v3);
    assert!(v1 < v3);
    assert_eq!(v1, SchemaVersion::new(1, 0, 0));
}

// B:generate_schema_from_registries — verify unit "empty schema round-trips through serde"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "empty schema serde round-trip")]
fn empty_schema_serde_round_trip() {
    let schema = GraphProtocolSchema::empty();
    let json = serde_json::to_string(&schema).unwrap();
    let deserialized: GraphProtocolSchema = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, schema);
    assert!(deserialized.entity_kinds.is_empty());
    assert!(deserialized.edge_types.is_empty());
    assert!(deserialized.extensions.is_empty());
}

// B:generate_schema_from_registries — verify unit "populated schema round-trips through serde"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "populated schema serde round-trip")]
fn populated_schema_serde_round_trip() {
    let schema = sample_schema();
    let json = serde_json::to_string_pretty(&schema).unwrap();
    let deserialized: GraphProtocolSchema = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, schema);
}

// ===========================================================================
// Slice 2: Generate Schema from Registries
// ===========================================================================

// B:generate_schema_from_registries — verify unit "generate from registries with kinds and edges"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "schema includes all registered entity kinds")]
fn generate_schema_includes_kinds_edges_fields() {
    let mut kinds = KindRegistry::new();
    kinds.register(make_kind_entry("behavior", "@specforge/software", true));
    kinds.register(make_kind_entry("feature", "@specforge/product", false));

    let mut edges = EdgeRegistry::new();
    edges.register(make_edge_entry("implements", "@specforge/software", Some("behavior"), Some("feature")));

    let mut fields = FieldRegistry::new();
    fields.register(make_field_entry("behavior", "contract", ManifestFieldType::String, false));
    fields.register(make_field_entry("behavior", "status", ManifestFieldType::Enum(vec!["draft".into(), "done".into()]), false));

    let extensions = vec![
        ("@specforge/software".to_string(), "1.0.0".to_string()),
        ("@specforge/product".to_string(), "1.0.0".to_string()),
    ];

    let schema = generate_schema(&kinds, &edges, &fields, &extensions);

    assert_eq!(schema.entity_kinds.len(), 2);
    assert_eq!(schema.entity_kinds[0].name, "behavior");
    assert!(schema.entity_kinds[0].testable);
    assert_eq!(schema.entity_kinds[0].fields.len(), 2);
    assert_eq!(schema.entity_kinds[1].name, "feature");
    assert!(!schema.entity_kinds[1].testable);
    assert_eq!(schema.entity_kinds[1].fields.len(), 0);

    assert_eq!(schema.edge_types.len(), 1);
    assert_eq!(schema.edge_types[0].label, "implements");
    assert_eq!(schema.edge_types[0].source_kinds, Some(vec!["behavior".to_string()]));
    assert_eq!(schema.edge_types[0].target_kinds, Some(vec!["feature".to_string()]));

    assert_eq!(schema.extensions.len(), 2);
}

// B:generate_schema_from_registries — verify unit "zero-extension registries produce valid empty schema"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "zero extensions produces valid empty schema")]
fn generate_schema_empty_registries() {
    let kinds = KindRegistry::new();
    let edges = EdgeRegistry::new();
    let fields = FieldRegistry::new();

    let schema = generate_schema(&kinds, &edges, &fields, &[]);
    assert!(schema.entity_kinds.is_empty());
    assert!(schema.edge_types.is_empty());
    assert!(schema.extensions.is_empty());
    assert_eq!(schema.schema_version, SchemaVersion::new(1, 0, 0));
}

// B:generate_schema_from_registries — verify unit "field types map correctly"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "schema fields match FieldRegistry entries")]
fn generate_schema_field_type_mapping() {
    let mut kinds = KindRegistry::new();
    kinds.register(make_kind_entry("behavior", "@specforge/software", true));

    let mut fields = FieldRegistry::new();
    fields.register(make_field_entry("behavior", "contract", ManifestFieldType::String, false));
    fields.register(make_field_entry("behavior", "priority", ManifestFieldType::Integer, false));
    fields.register(make_field_entry("behavior", "active", ManifestFieldType::Bool, false));
    fields.register(make_field_entry("behavior", "invariants", ManifestFieldType::ReferenceList, false));
    fields.register(make_field_entry("behavior", "status", ManifestFieldType::Enum(vec!["draft".into(), "done".into()]), false));

    let schema = generate_schema(&kinds, &EdgeRegistry::new(), &fields, &[]);

    let kind = &schema.entity_kinds[0];
    assert_eq!(kind.fields.len(), 5);

    let find_field = |name: &str| kind.fields.iter().find(|f| f.name == name).unwrap();
    assert_eq!(find_field("contract").field_type, "string");
    assert_eq!(find_field("priority").field_type, "integer");
    assert_eq!(find_field("active").field_type, "boolean");
    assert_eq!(find_field("invariants").field_type, "reference_list");
    assert_eq!(find_field("status").field_type, "enum");
    assert_eq!(find_field("status").enum_values, Some(vec!["draft".to_string(), "done".to_string()]));
}

// B:generate_schema_from_registries — verify unit "entity_kinds sorted by name"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "schema includes all registered edge types")]
fn generate_schema_deterministic_sort() {
    let mut kinds = KindRegistry::new();
    kinds.register(make_kind_entry("type", "@specforge/software", false));
    kinds.register(make_kind_entry("behavior", "@specforge/software", true));
    kinds.register(make_kind_entry("invariant", "@specforge/software", true));

    let mut edges = EdgeRegistry::new();
    edges.register(make_edge_entry("implements", "@specforge/software", None, None));
    edges.register(make_edge_entry("enforces", "@specforge/software", None, None));

    let schema = generate_schema(&kinds, &edges, &FieldRegistry::new(), &[]);

    let kind_names: Vec<&str> = schema.entity_kinds.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(kind_names, vec!["behavior", "invariant", "type"]);

    let edge_labels: Vec<&str> = schema.edge_types.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(edge_labels, vec!["enforces", "implements"]);
}

// ===========================================================================
// Slice 3: Embed Schema in Export
// ===========================================================================

// B:embed_schema_in_export — verify unit "emit_json_with_schema produces format_version 2.0"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "format_version set to 2.0 with schema")]
fn emit_json_with_schema_has_format_version() {
    let graph = Graph::new();
    let schema = GraphProtocolSchema::empty();
    let json = emit_json_with_schema(&graph, &schema);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["format_version"], "2.0");
    assert!(parsed["schema"].is_object());
    assert!(parsed["schema_version"].is_string());
}

// B:embed_schema_in_export — verify unit "emit_json_with_schema includes schema key"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "schema embedded as top-level key in JSON export")]
fn emit_json_with_schema_includes_schema() {
    let mut graph = Graph::new();
    graph.add_node(node("alpha", "behavior", Some("Alpha")));
    let schema = sample_schema();
    let json = emit_json_with_schema(&graph, &schema);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(parsed["schema"]["entity_kinds"].is_array());
    assert!(parsed["schema"]["edge_types"].is_array());
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 1);
}

// B:embed_schema_in_export — verify unit "existing emit_json has no schema key"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "backward compat emit_json")]
fn existing_emit_json_has_no_schema_key() {
    let graph = Graph::new();
    let json = emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("schema").is_none());
    assert!(parsed.get("format_version").is_none());
}

// B:embed_schema_in_export — verify unit "emit_context_with_schema produces format_version 2.0"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "context with schema")]
fn emit_context_with_schema_has_format_version() {
    let graph = Graph::new();
    let schema = GraphProtocolSchema::empty();
    let json = emit_context_with_schema(&graph, &schema);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["format_version"], "2.0");
    assert!(parsed["schema"].is_object());
}

// B:embed_schema_in_export — verify unit "emit_brief_with_schema produces format_version 2.0"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "brief with schema")]
fn emit_brief_with_schema_has_format_version() {
    let graph = Graph::new();
    let schema = GraphProtocolSchema::empty();
    let json = emit_brief_with_schema(&graph, &schema);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["format_version"], "2.0");
    assert!(parsed["schema"].is_object());
}

// B:embed_schema_in_export — verify unit "schema_version in V2 matches schema object"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "schema_version consistency")]
fn emit_json_with_schema_version_consistency() {
    let graph = Graph::new();
    let schema = sample_schema();
    let json = emit_json_with_schema(&graph, &schema);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let top_level = parsed["schema_version"].as_str().unwrap();
    assert_eq!(top_level, "1.2.3");
}

// ===========================================================================
// Slice 4: Schema Diffing
// ===========================================================================

// B:detect_breaking_schema_changes — verify unit "removed kind is breaking"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "removed entity kind detected as breaking")]
fn diff_removed_kind_is_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.entity_kinds.retain(|k| k.name != "feature");

    let migration = diff_schemas(&old, &new);
    assert!(migration.has_breaking_changes());
    assert!(migration.changes.iter().any(|c| matches!(c, SchemaMigrationChange::KindRemoved(name) if name == "feature")));
}

// B:detect_breaking_schema_changes — verify unit "added kind is non-breaking"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "added kind is non-breaking")]
fn diff_added_kind_is_non_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.entity_kinds.push(SchemaEntityKind {
        name: "event".to_string(),
        source_extension: "@specforge/software".to_string(),
        testable: true,
        fields: vec![],
    });

    let migration = diff_schemas(&old, &new);
    assert!(!migration.has_breaking_changes());
    assert!(migration.has_additions());
}

// B:detect_breaking_schema_changes — verify unit "added optional field is non-breaking"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "added optional field detected as non-breaking")]
fn diff_added_optional_field_non_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.entity_kinds[0].fields.push(SchemaField {
        name: "description".to_string(),
        field_type: "string".to_string(),
        required: false,
        enum_values: None,
        edge: None,
        target_kind: None,
        description: None,
        default_value: None,
        source_extension: "@specforge/software".to_string(),
    });

    let migration = diff_schemas(&old, &new);
    assert!(!migration.has_breaking_changes());
    assert!(migration.has_additions());
}

// B:detect_breaking_schema_changes — verify unit "added required field is breaking"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "new required field detected as breaking")]
fn diff_added_required_field_is_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.entity_kinds[0].fields.push(SchemaField {
        name: "severity".to_string(),
        field_type: "string".to_string(),
        required: true,
        enum_values: None,
        edge: None,
        target_kind: None,
        description: None,
        default_value: None,
        source_extension: "@specforge/software".to_string(),
    });

    let migration = diff_schemas(&old, &new);
    assert!(migration.has_breaking_changes());
}

// B:detect_breaking_schema_changes — verify unit "removed field is breaking"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "removed field is breaking")]
fn diff_removed_field_is_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.entity_kinds[0].fields.clear();

    let migration = diff_schemas(&old, &new);
    assert!(migration.has_breaking_changes());
    assert!(migration.changes.iter().any(|c| matches!(c, SchemaMigrationChange::FieldRemoved { kind, field } if kind == "behavior" && field == "contract")));
}

// B:detect_breaking_schema_changes — verify unit "removed edge is breaking"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "removed edge is breaking")]
fn diff_removed_edge_is_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.edge_types.clear();

    let migration = diff_schemas(&old, &new);
    assert!(migration.has_breaking_changes());
}

// B:detect_breaking_schema_changes — verify unit "added edge is non-breaking"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "added edge is non-breaking")]
fn diff_added_edge_non_breaking() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.edge_types.push(SchemaEdgeType {
        label: "enforces".to_string(),
        source_extension: "@specforge/software".to_string(),
        source_kinds: None,
        target_kinds: None,
    });

    let migration = diff_schemas(&old, &new);
    assert!(!migration.has_breaking_changes());
    assert!(migration.has_additions());
}

// B:detect_breaking_schema_changes — verify unit "no previous schema means all non-breaking"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "no previous schema treats all changes as non-breaking")]
fn diff_no_previous_schema_all_non_breaking() {
    let new = sample_schema();
    let migration = diff_schemas_optional(None, &new);
    assert!(!migration.has_breaking_changes());
    assert!(migration.has_additions());
}

// B:detect_breaking_schema_changes — verify unit "identical schemas have no changes"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "identical schemas no changes")]
fn diff_identical_schemas_no_changes() {
    let old = sample_schema();
    let new = sample_schema();
    let migration = diff_schemas(&old, &new);
    assert!(migration.is_empty());
    assert!(!migration.has_breaking_changes());
    assert!(!migration.has_additions());
}

// ===========================================================================
// Slice 5: Compute Schema Version
// ===========================================================================

// B:compute_schema_version — verify unit "no cache yields 1.0.0"
#[test]
#[specforge_test(behavior = "compute_schema_version", verify = "first compilation without cache produces version 1.0.0")]
fn compute_version_no_cache() {
    let migration = SchemaMigration { changes: vec![] };
    let version = compute_schema_version(&migration, None);
    assert_eq!(version, SchemaVersion::new(1, 0, 0));
}

// B:compute_schema_version — verify unit "new kind bumps minor"
#[test]
#[specforge_test(behavior = "compute_schema_version", verify = "new entity kind triggers minor version bump")]
fn compute_version_new_kind_bumps_minor() {
    let migration = SchemaMigration {
        changes: vec![SchemaMigrationChange::KindAdded("event".to_string())],
    };
    let version = compute_schema_version(&migration, Some(&SchemaVersion::new(1, 2, 3)));
    assert_eq!(version, SchemaVersion::new(1, 3, 0));
}

// B:compute_schema_version — verify unit "removed kind bumps major"
#[test]
#[specforge_test(behavior = "compute_schema_version", verify = "removed entity kind triggers major version bump")]
fn compute_version_removed_kind_bumps_major() {
    let migration = SchemaMigration {
        changes: vec![SchemaMigrationChange::KindRemoved("event".to_string())],
    };
    let version = compute_schema_version(&migration, Some(&SchemaVersion::new(1, 2, 3)));
    assert_eq!(version, SchemaVersion::new(2, 0, 0));
}

// B:compute_schema_version — verify unit "no changes returns previous unchanged"
#[test]
#[specforge_test(behavior = "compute_schema_version", verify = "no changes returns previous")]
fn compute_version_no_changes_returns_previous() {
    let migration = SchemaMigration { changes: vec![] };
    let version = compute_schema_version(&migration, Some(&SchemaVersion::new(3, 1, 4)));
    assert_eq!(version, SchemaVersion::new(3, 1, 4));
}

// B:compute_schema_version — verify unit "breaking + non-breaking = major bump"
#[test]
#[specforge_test(behavior = "compute_schema_version", verify = "mixed breaking and non-breaking")]
fn compute_version_mixed_changes_major_wins() {
    let migration = SchemaMigration {
        changes: vec![
            SchemaMigrationChange::KindAdded("event".to_string()),
            SchemaMigrationChange::KindRemoved("legacy".to_string()),
        ],
    };
    let version = compute_schema_version(&migration, Some(&SchemaVersion::new(1, 5, 2)));
    assert_eq!(version, SchemaVersion::new(2, 0, 0));
}

// ===========================================================================
// Slice 6: Version Negotiation
// ===========================================================================

// B:negotiate_schema_version — verify unit "in-range version succeeds"
#[test]
#[specforge_test(behavior = "negotiate_schema_version", verify = "compatible version within range is resolved")]
fn negotiate_in_range_succeeds() {
    let min = SchemaVersion::new(1, 0, 0);
    let max = SchemaVersion::new(1, 5, 0);
    let requested = SchemaVersion::new(1, 3, 0);

    let result = negotiate_version(&requested, &min, &max);
    assert!(result.is_ok());
    let compat = result.unwrap();
    assert_eq!(compat.requested, requested);
    assert_eq!(compat.resolved, requested);
}

// B:negotiate_schema_version — verify unit "exact min boundary succeeds"
#[test]
#[specforge_test(behavior = "negotiate_schema_version", verify = "exact min boundary")]
fn negotiate_exact_min_succeeds() {
    let min = SchemaVersion::new(1, 0, 0);
    let max = SchemaVersion::new(1, 5, 0);
    let result = negotiate_version(&min, &min, &max);
    assert!(result.is_ok());
}

// B:negotiate_schema_version — verify unit "exact max boundary succeeds"
#[test]
#[specforge_test(behavior = "negotiate_schema_version", verify = "exact max boundary")]
fn negotiate_exact_max_succeeds() {
    let min = SchemaVersion::new(1, 0, 0);
    let max = SchemaVersion::new(1, 5, 0);
    let result = negotiate_version(&max, &min, &max);
    assert!(result.is_ok());
}

// B:negotiate_schema_version — verify unit "out-of-range version fails with E027"
#[test]
#[specforge_test(behavior = "negotiate_schema_version", verify = "incompatible version produces E027 with supported range")]
fn negotiate_out_of_range_fails() {
    let min = SchemaVersion::new(1, 2, 0);
    let max = SchemaVersion::new(1, 5, 0);
    let requested = SchemaVersion::new(1, 1, 0);

    let result = negotiate_version(&requested, &min, &max);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.reason.contains("out of range"));
}

// B:negotiate_schema_version — verify unit "different major version fails"
#[test]
#[specforge_test(behavior = "negotiate_schema_version", verify = "different major version fails")]
fn negotiate_different_major_fails() {
    let min = SchemaVersion::new(1, 0, 0);
    let max = SchemaVersion::new(1, 5, 0);
    let requested = SchemaVersion::new(2, 0, 0);

    let result = negotiate_version(&requested, &min, &max);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.reason.contains("incompatible major version"));
}

// B:negotiate_schema_version — verify unit "SchemaVersionError Display includes E027"
#[test]
#[specforge_test(behavior = "negotiate_schema_version", verify = "error display")]
fn schema_version_error_display() {
    let err = SchemaVersionError {
        requested: SchemaVersion::new(2, 0, 0),
        min: SchemaVersion::new(1, 0, 0),
        max: SchemaVersion::new(1, 5, 0),
        reason: "incompatible".to_string(),
    };
    assert!(err.to_string().contains("E027"));
}

// ===========================================================================
// Slice 7: Schema Cache Persistence
// ===========================================================================

// B:persist_schema_cache — verify unit "write + read round-trip"
#[test]
#[specforge_test(behavior = "persist_schema_cache", verify = "schema-cache.json written after schema generation")]
fn cache_write_read_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let schema = sample_schema();

    persist_schema_cache(&schema, dir.path()).unwrap();
    let loaded = load_schema_cache(dir.path()).unwrap();

    assert!(loaded.is_some());
    let entry = loaded.unwrap();
    assert_eq!(entry.schema, schema);
    assert!(!entry.content_hash.is_empty());
}

// B:persist_schema_cache — verify unit "missing cache returns None"
#[test]
#[specforge_test(behavior = "persist_schema_cache", verify = "missing cache returns None")]
fn cache_missing_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = load_schema_cache(dir.path()).unwrap();
    assert!(loaded.is_none());
}

// B:persist_schema_cache — verify unit "atomic overwrite"
#[test]
#[specforge_test(behavior = "persist_schema_cache", verify = "cache file overwritten atomically via temp+rename")]
fn cache_atomic_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let schema1 = GraphProtocolSchema::empty();
    let schema2 = sample_schema();

    persist_schema_cache(&schema1, dir.path()).unwrap();
    persist_schema_cache(&schema2, dir.path()).unwrap();

    let loaded = load_schema_cache(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.schema, schema2);
}

// B:persist_schema_cache — verify unit "content hash changes with schema"
#[test]
#[specforge_test(behavior = "persist_schema_cache", verify = "content hash changes")]
fn cache_content_hash_changes() {
    let dir = tempfile::tempdir().unwrap();
    let schema1 = GraphProtocolSchema::empty();
    persist_schema_cache(&schema1, dir.path()).unwrap();
    let hash1 = load_schema_cache(dir.path()).unwrap().unwrap().content_hash;

    let schema2 = sample_schema();
    persist_schema_cache(&schema2, dir.path()).unwrap();
    let hash2 = load_schema_cache(dir.path()).unwrap().unwrap().content_hash;

    assert_ne!(hash1, hash2);
}

// B:persist_schema_cache — verify unit "cache independent of export"
#[test]
#[specforge_test(behavior = "persist_schema_cache", verify = "cache updated even when no JSON export is performed")]
fn cache_independent_of_export() {
    let dir = tempfile::tempdir().unwrap();
    let schema = sample_schema();
    persist_schema_cache(&schema, dir.path()).unwrap();

    // Cache file exists independently — no graph export needed
    let cache_file = dir.path().join("schema-cache.json");
    assert!(cache_file.exists());
    let contents = std::fs::read_to_string(&cache_file).unwrap();
    let entry: SchemaCacheEntry = serde_json::from_str(&contents).unwrap();
    assert_eq!(entry.schema.entity_kinds.len(), 2);
}

// ===========================================================================
// Slice 8: Serve Schema
// ===========================================================================

// B:serve_schema_resource — verify unit "full schema serializes to JSON"
#[test]
#[specforge_test(behavior = "serve_schema_resource", verify = "specforge schema outputs full schema as JSON")]
fn emit_schema_full() {
    let schema = sample_schema();
    let json = emit_schema(&schema);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["entity_kinds"].is_array());
    assert!(parsed["edge_types"].is_array());
    assert!(parsed["schema_version"].is_object());
}

// B:serve_schema_resource — verify unit "filter by kind returns single kind"
#[test]
#[specforge_test(behavior = "serve_schema_resource", verify = "--kind filter restricts to single entity kind")]
fn emit_schema_for_kind_single() {
    let schema = sample_schema();
    let json = emit_schema_for_kind(&schema, "behavior").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["name"], "behavior");
    assert!(parsed["fields"].is_array());
}

// B:serve_schema_resource — verify unit "missing kind returns error"
#[test]
#[specforge_test(behavior = "serve_schema_resource", verify = "missing kind error")]
fn emit_schema_for_kind_missing() {
    let schema = sample_schema();
    let result = emit_schema_for_kind(&schema, "nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown entity kind"));
}

// ===========================================================================
// Slice 9: Publish JSON Schema
// ===========================================================================

// B:publish_schema_specification — verify unit "valid JSON Schema with $schema draft-2020-12"
#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "published schema is valid JSON Schema")]
fn publish_json_schema_valid() {
    let schema = sample_schema();
    let json_schema_str = publish_json_schema(&schema);
    let parsed: serde_json::Value = serde_json::from_str(&json_schema_str).unwrap();

    assert_eq!(parsed["$schema"], "https://json-schema.org/draft/2020-12/schema");
    assert!(parsed["title"].is_string());
    assert_eq!(parsed["type"], "object");
}

// B:publish_schema_specification — verify unit "all kinds in node kind enum"
#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "kinds in enum")]
fn publish_json_schema_kinds_in_enum() {
    let schema = sample_schema();
    let json_schema_str = publish_json_schema(&schema);
    let parsed: serde_json::Value = serde_json::from_str(&json_schema_str).unwrap();

    let kind_enum = &parsed["properties"]["nodes"]["items"]["properties"]["kind"]["enum"];
    assert!(kind_enum.is_array());
    let kinds: Vec<&str> = kind_enum.as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert!(kinds.contains(&"behavior"));
    assert!(kinds.contains(&"feature"));
}

// B:publish_schema_specification — verify unit "all edge labels in edge label enum"
#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "edge labels in enum")]
fn publish_json_schema_edge_labels_in_enum() {
    let schema = sample_schema();
    let json_schema_str = publish_json_schema(&schema);
    let parsed: serde_json::Value = serde_json::from_str(&json_schema_str).unwrap();

    let label_enum = &parsed["properties"]["edges"]["items"]["properties"]["label"]["enum"];
    assert!(label_enum.is_array());
    let labels: Vec<&str> = label_enum.as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert!(labels.contains(&"implements"));
}

// B:publish_schema_specification — verify unit "required properties present"
#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "required properties")]
fn publish_json_schema_required_properties() {
    let schema = sample_schema();
    let json_schema_str = publish_json_schema(&schema);
    let parsed: serde_json::Value = serde_json::from_str(&json_schema_str).unwrap();

    let required = parsed["required"].as_array().unwrap();
    let required_strs: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(required_strs.contains(&"format_version"));
    assert!(required_strs.contains(&"schema_version"));
    assert!(required_strs.contains(&"nodes"));
    assert!(required_strs.contains(&"edges"));
}

// B:publish_schema_specification — verify unit "empty schema produces valid JSON Schema"
#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "third-party validator can use published schema")]
fn publish_json_schema_empty_schema() {
    let schema = GraphProtocolSchema::empty();
    let json_schema_str = publish_json_schema(&schema);
    let parsed: serde_json::Value = serde_json::from_str(&json_schema_str).unwrap();

    assert_eq!(parsed["$schema"], "https://json-schema.org/draft/2020-12/schema");
    // No enum for kind when no kinds registered
    let kind_prop = &parsed["properties"]["nodes"]["items"]["properties"]["kind"];
    assert!(kind_prop.get("enum").is_none());
    assert_eq!(kind_prop["type"], "string");
}

// B:publish_schema_specification — verify unit "has title"
#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "has title")]
fn publish_json_schema_has_title() {
    let schema = sample_schema();
    let json_schema_str = publish_json_schema(&schema);
    let parsed: serde_json::Value = serde_json::from_str(&json_schema_str).unwrap();
    assert_eq!(parsed["title"], "SpecForge Graph Protocol");
}

// ===========================================================================
// Integration: End-to-end
// ===========================================================================

// B:generate_schema_from_registries — verify integration "full pipeline: registries → schema → embed → diff"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "full pipeline integration")]
fn full_pipeline_registries_to_schema_to_embed() {
    // Build registries
    let mut kinds = KindRegistry::new();
    kinds.register(make_kind_entry("behavior", "@specforge/software", true));
    kinds.register(make_kind_entry("feature", "@specforge/product", false));

    let mut edges = EdgeRegistry::new();
    edges.register(make_edge_entry("implements", "@specforge/software", Some("behavior"), Some("feature")));

    let mut fields = FieldRegistry::new();
    fields.register(make_field_entry("behavior", "contract", ManifestFieldType::String, false));

    let extensions = vec![
        ("@specforge/software".to_string(), "1.0.0".to_string()),
    ];

    // Generate
    let schema = generate_schema(&kinds, &edges, &fields, &extensions);
    assert_eq!(schema.entity_kinds.len(), 2);

    // Embed
    let mut graph = Graph::new();
    graph.add_node(node("alpha", "behavior", Some("Alpha")));
    graph.add_node(node("beta", "feature", Some("Beta")));
    graph.add_edge(Edge {
        source: Sym::new("alpha"),
        target: Sym::new("beta"),
        label: Sym::new("implements"),
    });

    let json = emit_json_with_schema(&graph, &schema);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["format_version"], "2.0");
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["edges"].as_array().unwrap().len(), 1);
    assert_eq!(parsed["schema"]["entity_kinds"].as_array().unwrap().len(), 2);

    // Diff with empty
    let migration = diff_schemas_optional(None, &schema);
    assert!(!migration.has_breaking_changes());

    // Version
    let version = compute_schema_version(&migration, None);
    assert_eq!(version, SchemaVersion::new(1, 0, 0));
}

// B:compute_schema_version — verify unit "new edge bumps minor"
#[test]
#[specforge_test(behavior = "compute_schema_version", verify = "new edge bumps minor")]
fn compute_version_new_edge_bumps_minor() {
    let migration = SchemaMigration {
        changes: vec![SchemaMigrationChange::EdgeAdded("enforces".to_string())],
    };
    let version = compute_schema_version(&migration, Some(&SchemaVersion::new(1, 0, 0)));
    assert_eq!(version, SchemaVersion::new(1, 1, 0));
}

// B:compute_schema_version — verify unit "removed edge bumps major"
#[test]
#[specforge_test(behavior = "compute_schema_version", verify = "removed edge bumps major")]
fn compute_version_removed_edge_bumps_major() {
    let migration = SchemaMigration {
        changes: vec![SchemaMigrationChange::EdgeRemoved("enforces".to_string())],
    };
    let version = compute_schema_version(&migration, Some(&SchemaVersion::new(1, 0, 0)));
    assert_eq!(version, SchemaVersion::new(2, 0, 0));
}

// B:compute_schema_version — verify unit "added optional field bumps minor"
#[test]
#[specforge_test(behavior = "compute_schema_version", verify = "added optional field bumps minor")]
fn compute_version_added_optional_field_bumps_minor() {
    let migration = SchemaMigration {
        changes: vec![SchemaMigrationChange::FieldAdded {
            kind: "behavior".to_string(),
            field: "description".to_string(),
            required: false,
        }],
    };
    let version = compute_schema_version(&migration, Some(&SchemaVersion::new(1, 0, 0)));
    assert_eq!(version, SchemaVersion::new(1, 1, 0));
}

// B:compute_schema_version — verify unit "added required field bumps major"
#[test]
#[specforge_test(behavior = "compute_schema_version", verify = "added required field bumps major")]
fn compute_version_added_required_field_bumps_major() {
    let migration = SchemaMigration {
        changes: vec![SchemaMigrationChange::FieldAdded {
            kind: "behavior".to_string(),
            field: "severity".to_string(),
            required: true,
        }],
    };
    let version = compute_schema_version(&migration, Some(&SchemaVersion::new(1, 0, 0)));
    assert_eq!(version, SchemaVersion::new(2, 0, 0));
}

// B:compute_schema_version — verify unit "removed field bumps major"
#[test]
#[specforge_test(behavior = "compute_schema_version", verify = "removed field bumps major")]
fn compute_version_removed_field_bumps_major() {
    let migration = SchemaMigration {
        changes: vec![SchemaMigrationChange::FieldRemoved {
            kind: "behavior".to_string(),
            field: "contract".to_string(),
        }],
    };
    let version = compute_schema_version(&migration, Some(&SchemaVersion::new(1, 0, 0)));
    assert_eq!(version, SchemaVersion::new(2, 0, 0));
}

// B:embed_schema_in_export — verify unit "V2 nodes contain all fields"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "V2 nodes contain fields")]
fn emit_json_with_schema_nodes_have_fields() {
    let mut graph = Graph::new();
    let mut fields = FieldMap::new();
    fields.push(Sym::new("contract"), specforge_graph::FieldValue::String("MUST work".to_string()));
    graph.add_node(Node {
        id: EntityId { raw: Sym::new("alpha") },
        kind: EntityKind { raw: Sym::new("behavior") },
        title: Some("Alpha".to_string()),
        fields,
        source_span: span(),
    });

    let schema = GraphProtocolSchema::empty();
    let json = emit_json_with_schema(&graph, &schema);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let node = &parsed["nodes"].as_array().unwrap()[0];
    assert_eq!(node["fields"]["contract"], "MUST work");
    assert_eq!(node["file"], "test.spec");
    assert_eq!(node["line"], 1);
}

// B:embed_schema_in_export — verify unit "V2 edges present"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "V2 edges present")]
fn emit_json_with_schema_has_edges() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "behavior", None));
    graph.add_node(node("b", "feature", None));
    graph.add_edge(Edge {
        source: Sym::new("a"),
        target: Sym::new("b"),
        label: Sym::new("implements"),
    });

    let schema = GraphProtocolSchema::empty();
    let json = emit_json_with_schema(&graph, &schema);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let edges = parsed["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["source"], "a");
    assert_eq!(edges[0]["label"], "implements");
}

// B:negotiate_schema_version — verify unit "above max fails"
#[test]
#[specforge_test(behavior = "negotiate_schema_version", verify = "above max fails")]
fn negotiate_above_max_fails() {
    let min = SchemaVersion::new(1, 0, 0);
    let max = SchemaVersion::new(1, 5, 0);
    let requested = SchemaVersion::new(1, 6, 0);
    let result = negotiate_version(&requested, &min, &max);
    assert!(result.is_err());
}

// B:generate_schema_from_registries — verify unit "edge with no source/target kinds"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "edge with no source/target")]
fn generate_schema_edge_no_source_target() {
    let mut edges = EdgeRegistry::new();
    edges.register(make_edge_entry("custom_rel", "@specforge/software", None, None));

    let schema = generate_schema(&KindRegistry::new(), &edges, &FieldRegistry::new(), &[]);
    assert_eq!(schema.edge_types.len(), 1);
    assert!(schema.edge_types[0].source_kinds.is_none());
    assert!(schema.edge_types[0].target_kinds.is_none());
}

// B:detect_breaking_schema_changes — verify unit "multiple field changes tracked"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "multiple field changes")]
fn diff_multiple_field_changes() {
    let old = sample_schema();
    let mut new = sample_schema();
    // Remove existing field and add a new one
    new.entity_kinds[0].fields.clear();
    new.entity_kinds[0].fields.push(SchemaField {
        name: "description".to_string(),
        field_type: "string".to_string(),
        required: false,
        enum_values: None,
        edge: None,
        target_kind: None,
        description: None,
        default_value: None,
        source_extension: "@specforge/software".to_string(),
    });

    let migration = diff_schemas(&old, &new);
    assert!(migration.has_breaking_changes()); // removed "contract"
    assert!(migration.has_additions()); // added "description"
    assert!(migration.changes.len() >= 2);
}

// B:publish_schema_specification — verify unit "description includes version"
#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "description includes version")]
fn publish_json_schema_description_includes_version() {
    let schema = sample_schema();
    let json_schema_str = publish_json_schema(&schema);
    let parsed: serde_json::Value = serde_json::from_str(&json_schema_str).unwrap();
    let desc = parsed["description"].as_str().unwrap();
    assert!(desc.contains("1.2.3"));
}

// B:persist_schema_cache — verify integration "cache → load → diff → version"
#[test]
#[specforge_test(behavior = "persist_schema_cache", verify = "cache load diff version pipeline")]
fn cache_load_diff_version_pipeline() {
    let dir = tempfile::tempdir().unwrap();

    // Save initial schema
    let schema1 = GraphProtocolSchema::empty();
    persist_schema_cache(&schema1, dir.path()).unwrap();

    // Load and diff with new schema
    let cached = load_schema_cache(dir.path()).unwrap().unwrap();
    let schema2 = sample_schema();
    let migration = diff_schemas(&cached.schema, &schema2);

    // Schema2 has additions but no removals from empty
    assert!(migration.has_additions());

    let version = compute_schema_version(&migration, Some(&SchemaVersion::new(1, 0, 0)));
    assert_eq!(version, SchemaVersion::new(1, 1, 0));
}

// ===========================================================================
// Gap coverage: CLI integration (--no-schema, --schema-version)
// ===========================================================================

// B:embed_schema_in_export — verify unit "--no-schema suppresses schema and keeps format_version 1.0"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "--no-schema suppresses schema and keeps format_version 1.0")]
fn no_schema_flag_suppresses_schema() {
    // When --no-schema is set, the existing emit_json() is used (V1 format)
    // V1 format has no "schema" key and no "format_version" key
    let graph = Graph::new();
    let v1_json = emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&v1_json).unwrap();
    assert!(parsed.get("schema").is_none(), "V1 format must not have schema key");
    assert!(parsed.get("format_version").is_none(), "V1 format must not have format_version key");
    assert!(parsed["schema_version"].is_string(), "V1 format has schema_version (legacy)");
}

// B:negotiate_schema_version — verify unit "--schema-version CLI flag selects requested version"
#[test]
#[specforge_test(behavior = "negotiate_schema_version", verify = "--schema-version CLI flag selects requested version")]
fn schema_version_cli_flag_selects_version() {
    let requested: SchemaVersion = "1.2.0".parse().unwrap();
    let min = SchemaVersion::new(1, 0, 0);
    let max = SchemaVersion::new(1, 5, 0);
    let result = negotiate_version(&requested, &min, &max);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().resolved, SchemaVersion::new(1, 2, 0));
}

// B:negotiate_schema_version — verify unit "schema_version MCP query parameter selects requested version"
#[test]
#[specforge_test(behavior = "negotiate_schema_version", verify = "schema_version MCP query parameter selects requested version")]
fn schema_version_mcp_query_parameter() {
    let requested: SchemaVersion = "1.3.0".parse().unwrap();
    let min = SchemaVersion::new(1, 0, 0);
    let max = SchemaVersion::new(1, 5, 0);
    let result = negotiate_version(&requested, &min, &max);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().resolved, SchemaVersion::new(1, 3, 0));
}

// B:negotiate_schema_version — verify unit "no version requested defaults to latest"
#[test]
#[specforge_test(behavior = "negotiate_schema_version", verify = "no version requested defaults to latest")]
fn negotiate_no_version_defaults_to_latest() {
    let min = SchemaVersion::new(1, 0, 0);
    let max = SchemaVersion::new(1, 5, 0);
    let result = negotiate_version_or_latest(None, &min, &max);
    assert!(result.is_ok());
    let compat = result.unwrap();
    assert_eq!(compat.resolved, SchemaVersion::new(1, 5, 0));
}

// ===========================================================================
// Gap coverage: Diagnostic integration (I016)
// ===========================================================================

// B:detect_breaking_schema_changes — verify unit "missing cache with prior exports emits I016 info diagnostic"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "missing cache with prior exports emits I016 info diagnostic")]
fn detect_breaking_missing_cache_emits_i016() {
    let dir = tempfile::tempdir().unwrap();
    let current = sample_schema();

    let (migration, diagnostics) = detect_breaking_with_diagnostics(
        dir.path(),
        &current,
        true, // prior exports exist
    );

    assert!(!migration.has_breaking_changes());
    assert!(migration.has_additions());

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "I016");
    assert_eq!(diagnostics[0].severity, specforge_common::Severity::Info);
    assert!(diagnostics[0].message.contains("Schema cache not found"));
}

// B:detect_breaking_schema_changes — verify unit "missing cache without prior exports emits no diagnostic"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "missing cache without prior exports emits no diagnostic")]
fn detect_breaking_missing_cache_no_exports_no_diagnostic() {
    let dir = tempfile::tempdir().unwrap();
    let current = sample_schema();

    let (_migration, diagnostics) = detect_breaking_with_diagnostics(
        dir.path(),
        &current,
        false,
    );

    assert!(diagnostics.is_empty());
}

// B:detect_breaking_schema_changes — verify unit "cached schema used for diff"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "cached schema used for diff")]
fn detect_breaking_with_cached_schema() {
    let dir = tempfile::tempdir().unwrap();
    let old_schema = GraphProtocolSchema::empty();
    persist_schema_cache(&old_schema, dir.path()).unwrap();

    let current = sample_schema();
    let (migration, diagnostics) = detect_breaking_with_diagnostics(
        dir.path(),
        &current,
        false,
    );

    assert!(diagnostics.is_empty());
    assert!(migration.has_additions());
    assert!(!migration.has_breaking_changes());
}

// B:detect_breaking_schema_changes — verify unit "SchemaMigration record emitted on version change"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "SchemaMigration record emitted on version change")]
fn detect_breaking_migration_record_emitted() {
    let old = GraphProtocolSchema::empty();
    let new = sample_schema();
    let migration = diff_schemas(&old, &new);

    assert!(!migration.is_empty());
    assert!(migration.changes.len() >= 2);
}

// ===========================================================================
// Gap coverage: Runtime/lifecycle
// ===========================================================================

// B:generate_schema_from_registries — verify unit "schema generated once per compilation and cached"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "schema generated once per compilation and cached")]
fn schema_generated_deterministically_for_caching() {
    let mut kinds = KindRegistry::new();
    kinds.register(make_kind_entry("behavior", "@specforge/software", true));
    kinds.register(make_kind_entry("feature", "@specforge/product", false));

    let mut edges = EdgeRegistry::new();
    edges.register(make_edge_entry("implements", "@specforge/software", Some("behavior"), Some("feature")));

    let mut fields = FieldRegistry::new();
    fields.register(make_field_entry("behavior", "contract", ManifestFieldType::String, false));

    let ext = vec![("@specforge/software".to_string(), "1.0.0".to_string())];

    let schema1 = generate_schema(&kinds, &edges, &fields, &ext);
    let schema2 = generate_schema(&kinds, &edges, &fields, &ext);
    assert_eq!(schema1, schema2, "Schema generation must be deterministic for caching");
}

// B:serve_schema_resource — verify unit "schema reflects current compilation state"
#[test]
#[specforge_test(behavior = "serve_schema_resource", verify = "schema reflects current compilation state")]
fn schema_reflects_current_state() {
    let mut kinds1 = KindRegistry::new();
    kinds1.register(make_kind_entry("behavior", "@specforge/software", true));

    let schema1 = generate_schema(&kinds1, &EdgeRegistry::new(), &FieldRegistry::new(), &[]);

    let mut kinds2 = KindRegistry::new();
    kinds2.register(make_kind_entry("behavior", "@specforge/software", true));
    kinds2.register(make_kind_entry("event", "@specforge/software", true));

    let schema2 = generate_schema(&kinds2, &EdgeRegistry::new(), &FieldRegistry::new(), &[]);

    assert_ne!(schema1, schema2);
    assert_eq!(schema1.entity_kinds.len(), 1);
    assert_eq!(schema2.entity_kinds.len(), 2);
}

// ===========================================================================
// Gap coverage: JSON Schema validation
// ===========================================================================

// B:publish_schema_specification — verify unit "published schema validates known-good export"
#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "published schema validates known-good export")]
fn published_schema_validates_known_good_export() {
    let schema = sample_schema();
    let json_schema_str = publish_json_schema(&schema);
    let json_schema: serde_json::Value = serde_json::from_str(&json_schema_str).unwrap();

    let mut graph = Graph::new();
    graph.add_node(node("alpha", "behavior", Some("Alpha")));
    let export = emit_json_with_schema(&graph, &schema);
    let export_val: serde_json::Value = serde_json::from_str(&export).unwrap();

    // 1. Check required properties
    let required = json_schema["required"].as_array().unwrap();
    for req in required {
        let key = req.as_str().unwrap();
        assert!(export_val.get(key).is_some(), "required key '{}' missing from export", key);
    }

    // 2. Check format_version
    assert_eq!(export_val["format_version"], "2.0");

    // 3. Check node required fields
    let node_required = json_schema["properties"]["nodes"]["items"]["required"]
        .as_array().unwrap();
    for node_val in export_val["nodes"].as_array().unwrap() {
        for req in node_required {
            let key = req.as_str().unwrap();
            assert!(node_val.get(key).is_some(), "node missing required key '{}'", key);
        }
    }

    // 4. Check node kind is in enum
    let kind_enum = json_schema["properties"]["nodes"]["items"]["properties"]["kind"]["enum"]
        .as_array().unwrap();
    for node_val in export_val["nodes"].as_array().unwrap() {
        let kind = &node_val["kind"];
        assert!(kind_enum.contains(kind), "node kind '{}' not in schema enum", kind);
    }
}

// B:publish_schema_specification — verify unit "published schema describes all registered entity kinds"
#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "published schema describes all registered entity kinds")]
fn published_schema_describes_all_kinds() {
    let schema = sample_schema();
    let json_schema_str = publish_json_schema(&schema);
    let json_schema: serde_json::Value = serde_json::from_str(&json_schema_str).unwrap();

    let kind_enum = json_schema["properties"]["nodes"]["items"]["properties"]["kind"]["enum"]
        .as_array().unwrap();
    for ek in &schema.entity_kinds {
        assert!(kind_enum.contains(&serde_json::Value::String(ek.name.clone())),
            "entity kind '{}' not in published JSON Schema", ek.name);
    }
}

// B:publish_schema_specification — verify unit "published schema describes all edge types"
#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "published schema describes all edge types")]
fn published_schema_describes_all_edge_types() {
    let schema = sample_schema();
    let json_schema_str = publish_json_schema(&schema);
    let json_schema: serde_json::Value = serde_json::from_str(&json_schema_str).unwrap();

    let label_enum = json_schema["properties"]["edges"]["items"]["properties"]["label"]["enum"]
        .as_array().unwrap();
    for et in &schema.edge_types {
        assert!(label_enum.contains(&serde_json::Value::String(et.label.clone())),
            "edge type '{}' not in published JSON Schema", et.label);
    }
}

// ===========================================================================
// Gap coverage: Contract tests (DbC requires/ensures)
// ===========================================================================

// B:generate_schema_from_registries — verify contract "requires/ensures consistency for schema generation from registries"
#[test]
#[specforge_test(behavior = "generate_schema_from_registries", verify = "requires/ensures consistency for schema generation from registries")]
fn generate_schema_contract() {
    let mut kinds = KindRegistry::new();
    kinds.register(make_kind_entry("behavior", "@specforge/software", true));

    let mut fields = FieldRegistry::new();
    fields.register(make_field_entry("behavior", "contract", ManifestFieldType::String, false));

    let mut edges = EdgeRegistry::new();
    edges.register(make_edge_entry("implements", "@specforge/software", Some("behavior"), Some("feature")));

    let schema = generate_schema(&kinds, &edges, &fields, &[("@specforge/software".to_string(), "1.0.0".to_string())]);

    // ensures: all_kinds_in_schema
    assert_eq!(schema.entity_kinds.len(), kinds.len());
    for (_, entry) in kinds.iter() {
        assert!(schema.entity_kinds.iter().any(|k| k.name == entry.kind_name));
    }

    // ensures: all_edges_in_schema
    assert_eq!(schema.edge_types.len(), edges.len());
    for (_, entry) in edges.iter() {
        assert!(schema.edge_types.iter().any(|e| e.label == entry.label));
    }
}

// B:embed_schema_in_export — verify contract "requires/ensures consistency for schema embedding in export"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "requires/ensures consistency for schema embedding in export")]
fn embed_schema_contract() {
    let schema = sample_schema();
    let mut graph = Graph::new();
    graph.add_node(node("alpha", "behavior", Some("Alpha")));

    let json = emit_json_with_schema(&graph, &schema);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // ensures: schema_embedded
    assert!(parsed["schema"].is_object());
    // ensures: format_version_set
    assert_eq!(parsed["format_version"], "2.0");
    // ensures: full_project_schema
    assert_eq!(parsed["schema"]["entity_kinds"].as_array().unwrap().len(), schema.entity_kinds.len());
}

// B:detect_breaking_schema_changes — verify contract "requires/ensures consistency for breaking schema change detection"
#[test]
#[specforge_test(behavior = "detect_breaking_schema_changes", verify = "requires/ensures consistency for breaking schema change detection")]
fn detect_breaking_contract() {
    let old = sample_schema();

    let mut new_breaking = sample_schema();
    new_breaking.entity_kinds.retain(|k| k.name != "feature");
    let migration = diff_schemas(&old, &new_breaking);
    // ensures: breaking_changes_classified
    assert!(migration.has_breaking_changes());

    let mut new_nonbreaking = sample_schema();
    new_nonbreaking.entity_kinds[0].fields.push(SchemaField {
        name: "notes".to_string(),
        field_type: "string".to_string(),
        required: false,
        enum_values: None,
        edge: None,
        target_kind: None,
        description: None,
        default_value: None,
        source_extension: "@specforge/software".to_string(),
    });
    let migration2 = diff_schemas(&old, &new_nonbreaking);
    // ensures: nonbreaking_changes_classified
    assert!(!migration2.has_breaking_changes());
    assert!(migration2.has_additions());

    // ensures: migration_record_emitted
    assert!(!migration.changes.is_empty());
}

// B:compute_schema_version — verify contract "requires/ensures consistency for schema version computation"
#[test]
#[specforge_test(behavior = "compute_schema_version", verify = "requires/ensures consistency for schema version computation")]
fn compute_version_contract() {
    // ensures: first_compilation_baseline
    let empty = SchemaMigration { changes: vec![] };
    let v = compute_schema_version(&empty, None);
    assert_eq!(v, SchemaVersion::new(1, 0, 0));

    // ensures: version_auto_computed — major for breaking
    let breaking = SchemaMigration {
        changes: vec![SchemaMigrationChange::KindRemoved("x".into())],
    };
    let v = compute_schema_version(&breaking, Some(&SchemaVersion::new(1, 2, 3)));
    assert_eq!(v.major, 2);

    // ensures: version_auto_computed — minor for additions
    let addition = SchemaMigration {
        changes: vec![SchemaMigrationChange::KindAdded("y".into())],
    };
    let v = compute_schema_version(&addition, Some(&SchemaVersion::new(1, 2, 3)));
    assert_eq!(v, SchemaVersion::new(1, 3, 0));
}

// B:negotiate_schema_version — verify contract "requires/ensures consistency for schema version negotiation"
#[test]
#[specforge_test(behavior = "negotiate_schema_version", verify = "requires/ensures consistency for schema version negotiation")]
fn negotiate_version_contract() {
    let min = SchemaVersion::new(1, 0, 0);
    let max = SchemaVersion::new(1, 5, 0);

    // ensures: compatible_version_resolved
    let result = negotiate_version(&SchemaVersion::new(1, 3, 0), &min, &max);
    assert!(result.is_ok());

    // ensures: incompatible_version_rejected
    let result = negotiate_version(&SchemaVersion::new(2, 0, 0), &min, &max);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("E027"));

    // ensures: default_to_latest
    let result = negotiate_version_or_latest(None, &min, &max);
    assert_eq!(result.unwrap().resolved, max);
}

// B:persist_schema_cache — verify contract "requires/ensures consistency for schema cache persistence"
#[test]
#[specforge_test(behavior = "persist_schema_cache", verify = "requires/ensures consistency for schema cache persistence")]
fn persist_cache_contract() {
    let dir = tempfile::tempdir().unwrap();
    let schema = sample_schema();

    persist_schema_cache(&schema, dir.path()).unwrap();
    // ensures: cache_written_atomically
    assert!(!dir.path().join(".schema-cache.tmp").exists());
    // ensures: cache_always_updated
    assert!(dir.path().join("schema-cache.json").exists());

    let loaded = load_schema_cache(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.schema, schema);
}

// B:serve_schema_resource — verify contract "requires/ensures consistency for schema resource serving"
#[test]
#[specforge_test(behavior = "serve_schema_resource", verify = "requires/ensures consistency for schema resource serving")]
fn serve_schema_contract() {
    let schema = sample_schema();

    // ensures: full_schema_output
    let json = emit_schema(&schema);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["entity_kinds"].is_array());

    // ensures: kind_filter_supported
    let filtered = emit_schema_for_kind(&schema, "behavior").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&filtered).unwrap();
    assert_eq!(parsed["name"], "behavior");

    // error for unknown kind
    assert!(emit_schema_for_kind(&schema, "nonexistent").is_err());
}

// B:publish_schema_specification — verify contract "requires/ensures consistency for schema specification publication"
#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "requires/ensures consistency for schema specification publication")]
fn publish_schema_contract() {
    let schema = sample_schema();
    let json_schema_str = publish_json_schema(&schema);
    let parsed: serde_json::Value = serde_json::from_str(&json_schema_str).unwrap();

    // ensures: valid_json_schema_produced
    assert_eq!(parsed["$schema"], "https://json-schema.org/draft/2020-12/schema");
    // ensures: all_kinds_described
    let kind_enum = parsed["properties"]["nodes"]["items"]["properties"]["kind"]["enum"]
        .as_array().unwrap();
    assert_eq!(kind_enum.len(), schema.entity_kinds.len());
    // ensures: third_party_usable
    assert!(parsed["properties"].is_object());
    assert!(parsed["required"].is_array());
}

// ===========================================================================
// Gap coverage: MCP resource (specforge://schema)
// ===========================================================================

// B:serve_schema_resource — verify unit "MCP resource specforge://schema returns schema"
#[test]
#[specforge_test(behavior = "serve_schema_resource", verify = "MCP resource specforge://schema returns schema")]
fn mcp_schema_resource_returns_graph_protocol_schema() {
    let schema = GraphProtocolSchema::empty();
    let json = emit_schema(&schema);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_object());
    assert!(parsed["entity_kinds"].is_array());
    assert!(parsed["edge_types"].is_array());
    assert!(parsed["extensions"].is_array());
}

// ===========================================================================
// Gap coverage: Scoped V2 exports
// ===========================================================================

// B:embed_schema_in_export — verify unit "scoped V2 export has full schema"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "scoped V2 export has full schema")]
fn scoped_v2_export_has_full_schema() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "behavior", Some("A")));
    graph.add_node(node("b", "feature", Some("B")));
    graph.add_edge(Edge {
        source: Sym::new("b"),
        target: Sym::new("a"),
        label: Sym::new("behaviors"),
    });

    let schema = sample_schema();
    let json = emit_json_scoped_with_schema(&graph, "a", &schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["format_version"], "2.0");
    assert_eq!(parsed["schema"]["entity_kinds"].as_array().unwrap().len(), 2);
}

// B:embed_schema_in_export — verify unit "scoped context V2 export works"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "scoped context V2 export")]
fn scoped_context_v2_export() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "behavior", Some("A")));
    let schema = GraphProtocolSchema::empty();
    let json = emit_context_scoped_with_schema(&graph, "a", &schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["format_version"], "2.0");
    assert!(parsed["schema"].is_object());
}

// B:embed_schema_in_export — verify unit "scoped brief V2 export works"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "scoped brief V2 export")]
fn scoped_brief_v2_export() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "behavior", Some("A")));
    let schema = GraphProtocolSchema::empty();
    let json = emit_brief_scoped_with_schema(&graph, "a", &schema).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["format_version"], "2.0");
    assert!(parsed["schema"].is_object());
}

// B:embed_schema_in_export — verify unit "scoped V2 export with nonexistent scope returns error"
#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "scoped V2 nonexistent scope error")]
fn scoped_v2_nonexistent_scope_error() {
    let graph = Graph::new();
    let schema = GraphProtocolSchema::empty();
    let result = emit_json_scoped_with_schema(&graph, "nonexistent", &schema);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("E001"));
}

// ===========================================================================
// Gap coverage: compute_schema_version — metadata-only change
// ===========================================================================

// B:compute_schema_version — verify unit "field metadata change triggers patch version bump"
#[test]
#[specforge_test(behavior = "compute_schema_version", verify = "field metadata change triggers patch version bump")]
fn compute_version_metadata_only_no_bump() {
    let old = sample_schema();
    let mut new = sample_schema();
    new.extensions[0].version = "2.0.0".to_string();

    let migration = diff_schemas(&old, &new);
    assert!(migration.is_empty());
    let version = compute_schema_version(&migration, Some(&SchemaVersion::new(1, 2, 3)));
    assert_eq!(version, SchemaVersion::new(1, 2, 3), "no structural change = no version bump");
}
