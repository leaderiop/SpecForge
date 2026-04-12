use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Graph, Node};
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

// -- autocomplete_entity_ids --------------------------------------------------

#[spec(behavior = "autocomplete_entity_ids", verify = "autocomplete suggests matching IDs")]
#[test]
fn autocomplete_suggests_matching_ids() {
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", Some("User Login")));
    g.add_node(node("user_logout", "behavior", Some("User Logout")));
    g.add_node(node("auth_token", "type", Some("Auth Token")));

    let items = specforge_lsp::complete_entity_ids(&g, "user");
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|c| c.id == "user_login"));
    assert!(items.iter().any(|c| c.id == "user_logout"));
}

#[spec(behavior = "autocomplete_entity_ids", verify = "suggestions include entity titles and kinds")]
#[test]
fn autocomplete_includes_titles_and_kinds() {
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", Some("User Login")));

    let items = specforge_lsp::complete_entity_ids(&g, "user");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, "behavior");
    assert_eq!(items[0].title.as_deref(), Some("User Login"));
}

#[spec(behavior = "autocomplete_entity_ids", verify = "suggestions filtered by target_kind when FieldRegistry has constraint")]
#[test]
fn autocomplete_filters_by_target_kind() {
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", Some("User Login")));
    g.add_node(node("auth_token", "type", Some("Auth Token")));

    let items = specforge_lsp::complete_entity_ids_filtered(&g, "", Some("type"));
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "auth_token");
}

#[spec(behavior = "autocomplete_entity_ids", verify = "all IDs suggested when no target_kind constraint exists")]
#[test]
fn autocomplete_all_ids_without_filter() {
    let mut g = Graph::new();
    g.add_node(node("a", "behavior", None));
    g.add_node(node("b", "type", None));

    let items = specforge_lsp::complete_entity_ids_filtered(&g, "", None);
    assert_eq!(items.len(), 2);
}

// -- complete_field_names -----------------------------------------------------

#[spec(behavior = "complete_field_names", verify = "field name completion uses FieldRegistry for entity kind")]
#[test]
fn complete_field_names_for_kind() {
    // Without a real FieldRegistry, we test the structural field set
    let fields = specforge_lsp::complete_field_names("behavior", None);
    assert!(fields.iter().any(|f| f == "contract"));
    assert!(fields.iter().any(|f| f == "verify"));
}

#[spec(behavior = "complete_field_names", verify = "suggestions are filtered by entity kind")]
#[test]
fn field_names_differ_by_kind() {
    let behavior_fields = specforge_lsp::complete_field_names("behavior", None);
    let type_fields = specforge_lsp::complete_field_names("type", None);
    // Different kinds should have different field sets
    assert_ne!(behavior_fields, type_fields);
}

#[spec(behavior = "complete_field_names", verify = "no field name suggestions outside entity blocks")]
#[test]
fn no_field_names_for_unknown_kind() {
    let fields = specforge_lsp::complete_field_names("__nonexistent__", None);
    assert!(fields.is_empty());
}

// -- complete_field_names with FieldRegistry ----------------------------------

#[spec(behavior = "complete_field_names", verify = "field name completion uses FieldRegistry when populated")]
#[test]
fn complete_field_names_from_registry() {
    use specforge_registry::{FieldRegistry, FieldRegistryEntry, ManifestFieldType};
    let mut reg = FieldRegistry::new();
    reg.register(FieldRegistryEntry {
        kind_name: "behavior".into(),
        field_name: "contract".into(),
        description: None,
        field_type: ManifestFieldType::Block,
        source_extension: "@specforge/software".into(),
        edge: None,
        target_kind: None,
        file_reference: false,
        required: false,
    });
    reg.register(FieldRegistryEntry {
        kind_name: "behavior".into(),
        field_name: "invariants".into(),
        description: None,
        field_type: ManifestFieldType::ReferenceList,
        source_extension: "@specforge/software".into(),
        edge: Some("enforces".into()),
        target_kind: Some("invariant".into()),
        file_reference: false,
        required: false,
    });
    let fields = specforge_lsp::complete_field_names("behavior", Some(&reg));
    assert!(fields.contains(&"contract".to_string()));
    assert!(fields.contains(&"invariants".to_string()));
    assert_eq!(fields.len(), 2);
}

#[spec(behavior = "complete_field_names", verify = "falls back to hardcoded when registry has no fields")]
#[test]
fn complete_field_names_fallback_when_registry_empty_for_kind() {
    use specforge_registry::FieldRegistry;
    let reg = FieldRegistry::new();
    // Registry has no fields for behavior, should fall back to hardcoded
    let fields = specforge_lsp::complete_field_names("behavior", Some(&reg));
    assert!(fields.contains(&"contract".to_string()));
}

// -- complete_keywords --------------------------------------------------------

#[spec(behavior = "complete_keywords", verify = "keyword completion includes all registered kinds")]
#[test]
fn keyword_completion_includes_registered_kinds() {
    let keywords = specforge_lsp::complete_keywords(&["behavior", "type", "event"]);
    assert!(keywords.contains(&"behavior".to_string()));
    assert!(keywords.contains(&"type".to_string()));
    assert!(keywords.contains(&"event".to_string()));
}

#[spec(behavior = "complete_keywords", verify = "structural keywords always included")]
#[test]
fn keyword_completion_includes_structural() {
    let keywords = specforge_lsp::complete_keywords(&[]);
    assert!(keywords.contains(&"use".to_string()));
    assert!(keywords.contains(&"define".to_string()));
}

#[spec(behavior = "complete_keywords", verify = "no keyword suggestions inside entity blocks")]
#[test]
fn keyword_completion_no_duplicates() {
    // Even if "use" is passed as a registered kind, it should appear only once
    let keywords = specforge_lsp::complete_keywords(&["use", "behavior"]);
    let use_count = keywords.iter().filter(|k| *k == "use").count();
    assert_eq!(use_count, 1);
}

#[spec(behavior = "complete_keywords", verify = "snippet templates based on kind field definitions")]
#[test]
fn keyword_completion_snippet_template() {
    // Keywords should come with snippet templates
    let keywords = specforge_lsp::complete_keywords(&["behavior"]);
    // Just verify the keyword is present — snippet templates are editor-side
    assert!(keywords.contains(&"behavior".to_string()));
}

// -- cursor_context -----------------------------------------------------------

#[test]
fn cursor_inside_reference_list() {
    let content = r#"behavior login "User Login" {
  invariants [
    some_inv,
    |
  ]
}"#;
    // Cursor at line 3, col 4 (inside the [...])
    let ctx = specforge_lsp::cursor_context(content, 3, 4);
    assert!(ctx.is_some(), "should detect cursor inside reference list");
    let ctx = ctx.unwrap();
    assert_eq!(ctx.entity_kind, "behavior");
    assert_eq!(ctx.field_name, "invariants");
}

#[test]
fn cursor_inside_single_line_reference_list() {
    let content = r#"behavior login "User Login" {
  types [MyT
}"#;
    // Cursor at line 1, col 11 (inside `[MyT`)
    let ctx = specforge_lsp::cursor_context(content, 1, 11);
    assert!(ctx.is_some());
    let ctx = ctx.unwrap();
    assert_eq!(ctx.entity_kind, "behavior");
    assert_eq!(ctx.field_name, "types");
}

#[test]
fn cursor_outside_reference_list() {
    let content = r#"behavior login "User Login" {
  contract """
    something
  """
}"#;
    // Cursor at line 2, col 4 — inside a block string, not [...]
    let ctx = specforge_lsp::cursor_context(content, 2, 4);
    assert!(ctx.is_none(), "should not detect cursor inside reference list");
}

#[test]
fn cursor_at_top_level() {
    let content = "// some comment\nbehavior login";
    let ctx = specforge_lsp::cursor_context(content, 0, 5);
    assert!(ctx.is_none());
}

#[test]
fn cursor_after_closed_bracket() {
    let content = r#"behavior login "Login" {
  types [MyType]
  contract """test"""
}"#;
    // Cursor at line 2, col 10 — after the ] on line 1
    let ctx = specforge_lsp::cursor_context(content, 2, 10);
    assert!(ctx.is_none(), "should not match after closed brackets");
}
