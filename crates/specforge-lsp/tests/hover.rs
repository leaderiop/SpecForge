use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test_macros::test as spec;

fn node(id: &str, kind: &str, title: Option<&str>) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: title.map(|t| t.to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 0, start_col: 0, end_line: 0, end_col: 0,
        },
    }
}

fn edge(source: &str, target: &str, label: &str) -> Edge {
    Edge {
        source: source.into(),
        target: target.into(),
        label: label.into(),
    }
}

#[spec(behavior = "hover_information", verify = "hover delegates to provide_extension_entity_hover")]
#[test]
fn hover_returns_entity_info() {
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", Some("User Login")));
    g.add_node(node("auth_token", "type", Some("Auth Token")));
    g.add_edge(edge("user_login", "auth_token", "types"));

    let hover = specforge_lsp::hover_info(&g, "user_login");
    let text = hover.expect("should produce hover");
    assert!(text.contains("behavior"));
    assert!(text.contains("user_login"));
    assert!(text.contains("User Login"));
}

#[spec(behavior = "hover_information", verify = "hover returns markdown-formatted content")]
#[test]
fn hover_returns_markdown() {
    let mut g = Graph::new();
    g.add_node(node("my_type", "type", Some("My Type")));

    let hover = specforge_lsp::hover_info(&g, "my_type");
    let text = hover.expect("should produce hover");
    assert!(text.contains("**") || text.contains("#"));
}

#[test]
fn hover_shows_outgoing_edges() {
    let mut g = Graph::new();
    g.add_node(node("create_user", "behavior", Some("Create User")));
    g.add_node(node("user_management", "feature", Some("User Management")));
    g.add_node(node("user_type", "type", None));
    g.add_edge(edge("create_user", "user_management", "features"));
    g.add_edge(edge("create_user", "user_type", "types"));

    let text = specforge_lsp::hover_info(&g, "create_user").unwrap();
    assert!(text.contains("**References:**"), "should have References section:\n{text}");
    assert!(text.contains("features: user_management"), "should list feature ref:\n{text}");
    assert!(text.contains("types: user_type"), "should list type ref:\n{text}");
}

#[test]
fn hover_shows_incoming_edges() {
    let mut g = Graph::new();
    g.add_node(node("user_management", "feature", Some("User Management")));
    g.add_node(node("create_user", "behavior", Some("Create User")));
    g.add_node(node("delete_user", "behavior", Some("Delete User")));
    g.add_edge(edge("create_user", "user_management", "features"));
    g.add_edge(edge("delete_user", "user_management", "features"));

    let text = specforge_lsp::hover_info(&g, "user_management").unwrap();
    assert!(text.contains("**Referenced by:**"), "should have Referenced by section:\n{text}");
    assert!(text.contains("behavior (features):"), "should group by kind+label:\n{text}");
    assert!(text.contains("create_user"), "should list source ID:\n{text}");
    assert!(text.contains("delete_user"), "should list source ID:\n{text}");
}

#[test]
fn hover_shows_both_directions() {
    let mut g = Graph::new();
    g.add_node(node("create_user", "behavior", Some("Create User")));
    g.add_node(node("user_management", "feature", None));
    g.add_node(node("data_integrity", "invariant", None));
    // create_user → user_management (outgoing)
    g.add_edge(edge("create_user", "user_management", "features"));
    // data_integrity → create_user (incoming)
    g.add_edge(edge("data_integrity", "create_user", "enforced_by"));

    let text = specforge_lsp::hover_info(&g, "create_user").unwrap();
    assert!(text.contains("**References:**"), "should have outgoing:\n{text}");
    assert!(text.contains("**Referenced by:**"), "should have incoming:\n{text}");
    assert!(text.contains("features: user_management"));
    assert!(text.contains("invariant (enforced_by): data_integrity"));
}

#[test]
fn hover_no_edges_shows_no_sections() {
    let mut g = Graph::new();
    g.add_node(node("orphan", "type", Some("Orphan Type")));

    let text = specforge_lsp::hover_info(&g, "orphan").unwrap();
    assert!(!text.contains("References:"), "no outgoing section:\n{text}");
    assert!(!text.contains("Referenced by:"), "no incoming section:\n{text}");
    assert!(text.contains("**type** `orphan` — Orphan Type"));
}

#[test]
fn hover_groups_multiple_incoming_by_kind() {
    let mut g = Graph::new();
    g.add_node(node("auth_feature", "feature", None));
    g.add_node(node("login", "behavior", None));
    g.add_node(node("logout", "behavior", None));
    g.add_node(node("v1_launch", "milestone", None));
    g.add_edge(edge("login", "auth_feature", "features"));
    g.add_edge(edge("logout", "auth_feature", "features"));
    g.add_edge(edge("v1_launch", "auth_feature", "features"));

    let text = specforge_lsp::hover_info(&g, "auth_feature").unwrap();
    // Should have two groups: behavior (features) and milestone (features)
    assert!(text.contains("behavior (features):"), "should group behaviors:\n{text}");
    assert!(text.contains("milestone (features):"), "should group milestones:\n{text}");
}

#[test]
fn hover_nonexistent_entity_returns_none() {
    let g = Graph::new();
    assert!(specforge_lsp::hover_info(&g, "nonexistent").is_none());
}

// -- hover_info_with_registries -----------------------------------------------

#[test]
fn hover_shows_extension_source() {
    use specforge_registry::{KindRegistry, KindRegistryEntry};
    let mut g = Graph::new();
    g.add_node(node("login", "behavior", Some("User Login")));

    let mut kind_reg = KindRegistry::new();
    kind_reg.register(KindRegistryEntry {
        kind_name: "behavior".into(),
        description: None,
        source_extension: "@specforge/software".into(),
        testable: true,
        singleton: false,
        supports_verify: true,
        allowed_verify_kinds: vec![],
        semantic_token: None,
        lsp_icon: None,
        dot_shape: None,
        dot_color: None,
        dot_fillcolor: None,
        open_fields: false,
    });

    let text = specforge_lsp::hover_info_with_registries(&g, "login", Some(&kind_reg), None)
        .unwrap();
    assert!(text.contains("@specforge/software"), "should show extension source:\n{text}");
    assert!(text.contains("**behavior** `login`"), "should still show basic info:\n{text}");
}

#[test]
fn hover_shows_registered_fields() {
    use specforge_registry::{FieldRegistry, FieldRegistryEntry, ManifestFieldType};
    let mut g = Graph::new();
    g.add_node(node("login", "behavior", Some("Login")));

    let mut field_reg = FieldRegistry::new();
    field_reg.register(FieldRegistryEntry {
        kind_name: "behavior".into(),
        field_name: "contract".into(),
        description: None,
        field_type: ManifestFieldType::Block,
        source_extension: "@specforge/software".into(),
        edge: None,
        target_kind: None,
        file_reference: false,
        required: false,
        inverse_of: None,
    });
    field_reg.register(FieldRegistryEntry {
        kind_name: "behavior".into(),
        field_name: "invariants".into(),
        description: None,
        field_type: ManifestFieldType::ReferenceList,
        source_extension: "@specforge/software".into(),
        edge: Some("enforces".into()),
        target_kind: Some("invariant".into()),
        file_reference: false,
        required: false,
        inverse_of: None,
    });

    let text = specforge_lsp::hover_info_with_registries(&g, "login", None, Some(&field_reg))
        .unwrap();
    assert!(text.contains("**Fields:**"), "should have Fields section:\n{text}");
    assert!(text.contains("`contract`: block"), "should show contract field:\n{text}");
    assert!(text.contains("`invariants`: reference_list"), "should show invariants field:\n{text}");
}

#[test]
fn hover_no_fields_section_without_registry() {
    let mut g = Graph::new();
    g.add_node(node("login", "behavior", Some("Login")));

    let text = specforge_lsp::hover_info(&g, "login").unwrap();
    assert!(!text.contains("Fields:"), "should not show Fields without registry:\n{text}");
}

#[test]
fn hover_shows_entity_kind_description() {
    use specforge_registry::{KindRegistry, KindRegistryEntry};
    let mut g = Graph::new();
    g.add_node(node("login", "behavior", Some("User Login")));

    let mut kind_reg = KindRegistry::new();
    kind_reg.register(KindRegistryEntry {
        kind_name: "behavior".into(),
        description: Some("A testable unit of system functionality".into()),
        source_extension: "@specforge/software".into(),
        testable: true,
        singleton: false,
        supports_verify: true,
        allowed_verify_kinds: vec![],
        semantic_token: None,
        lsp_icon: None,
        dot_shape: None,
        dot_color: None,
        dot_fillcolor: None,
        open_fields: false,
    });

    let text = specforge_lsp::hover_info_with_registries(&g, "login", Some(&kind_reg), None)
        .unwrap();
    assert!(
        text.contains("A testable unit of system functionality"),
        "should show entity kind description:\n{text}"
    );
}

#[test]
fn hover_shows_field_description() {
    use specforge_registry::{FieldRegistry, FieldRegistryEntry, ManifestFieldType};
    let mut reg = FieldRegistry::new();
    reg.register(FieldRegistryEntry {
        kind_name: "behavior".into(),
        field_name: "contract".into(),
        description: Some("The behavioral contract this entity fulfills".into()),
        field_type: ManifestFieldType::String,
        source_extension: "@specforge/software".into(),
        edge: None,
        target_kind: None,
        file_reference: false,
        required: false,
        inverse_of: None,
    });

    let text = specforge_lsp::hover_field_info("contract", "behavior", &reg).unwrap();
    assert!(
        text.contains("The behavioral contract this entity fulfills"),
        "should show field description:\n{text}"
    );
}

#[test]
fn hover_no_extension_source_without_registry() {
    let mut g = Graph::new();
    g.add_node(node("login", "behavior", Some("Login")));

    let text = specforge_lsp::hover_info(&g, "login").unwrap();
    assert!(!text.contains("from"), "should not show extension source without registry:\n{text}");
}

// -- hover_field_info --------------------------------------------------------

fn make_field_registry() -> specforge_registry::FieldRegistry {
    use specforge_registry::{FieldRegistry, FieldRegistryEntry, ManifestFieldType};
    let mut reg = FieldRegistry::new();
    reg.register(FieldRegistryEntry {
        kind_name: "behavior".into(),
        field_name: "contract".into(),
        description: None,
        field_type: ManifestFieldType::String,
        source_extension: "@specforge/software".into(),
        edge: None,
        target_kind: None,
        file_reference: false,
        required: false,
        inverse_of: None,
    });
    reg.register(FieldRegistryEntry {
        kind_name: "behavior".into(),
        field_name: "features".into(),
        description: None,
        field_type: ManifestFieldType::ReferenceList,
        source_extension: "@specforge/software".into(),
        edge: Some("BehaviorImplementsFeature".into()),
        target_kind: Some("feature".into()),
        file_reference: false,
        required: true,
        inverse_of: None,
    });
    reg
}

#[test]
fn field_hover_shows_type_and_extension() {
    let reg = make_field_registry();
    let text = specforge_lsp::hover_field_info("contract", "behavior", &reg).unwrap();
    assert!(text.contains("`contract`"), "should show field name:\n{text}");
    assert!(text.contains("string"), "should show type:\n{text}");
    assert!(text.contains("@specforge/software"), "should show extension:\n{text}");
}

#[test]
fn field_hover_shows_target_kind_and_edge() {
    let reg = make_field_registry();
    let text = specforge_lsp::hover_field_info("features", "behavior", &reg).unwrap();
    assert!(text.contains("→ **feature**"), "should show target kind:\n{text}");
    assert!(text.contains("Edge: `BehaviorImplementsFeature`"), "should show edge type:\n{text}");
    assert!(text.contains("*required*"), "should show required:\n{text}");
}

#[test]
fn field_hover_unknown_field_returns_none() {
    let reg = make_field_registry();
    assert!(specforge_lsp::hover_field_info("nonexistent", "behavior", &reg).is_none());
}

#[test]
fn field_hover_unknown_kind_returns_none() {
    let reg = make_field_registry();
    assert!(specforge_lsp::hover_field_info("contract", "unknown_kind", &reg).is_none());
}

// -- enclosing_entity_kind ---------------------------------------------------

#[test]
fn enclosing_entity_kind_finds_block() {
    let content = "behavior login \"Login\" {\n  contract \"test\"\n}\n";
    let kind = specforge_lsp::enclosing_entity_kind(content, 1);
    assert_eq!(kind.as_deref(), Some("behavior"));
}

#[test]
fn enclosing_entity_kind_outside_block_returns_none() {
    let content = "behavior login \"Login\" {\n  contract \"test\"\n}\n\n// file level\n";
    let kind = specforge_lsp::enclosing_entity_kind(content, 4);
    assert_eq!(kind, None);
}

#[test]
fn enclosing_entity_kind_on_header_line() {
    let content = "feature user_mgmt \"User Management\" {\n  problem \"x\"\n}\n";
    let kind = specforge_lsp::enclosing_entity_kind(content, 0);
    assert_eq!(kind.as_deref(), Some("feature"));
}

#[test]
fn enclosing_entity_kind_skips_nested_braces() {
    // requires/ensures blocks have their own { } but are indented
    let content = r#"behavior load "Load" {
  requires {
    x "something"
  }
  ensures {
    y "result"
  }
  features [some_feature]
}"#;
    // Line 7 is `  features [some_feature]` — should find `behavior` despite `}` on lines 3 and 6
    let kind = specforge_lsp::enclosing_entity_kind(content, 7);
    assert_eq!(kind.as_deref(), Some("behavior"));
}
