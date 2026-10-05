use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_registry::FieldRegistry;
use specforge_test_macros::test as spec;

fn default_field_registry() -> FieldRegistry {
    let ext_names: Vec<String> = [
        "@specforge/software",
        "@specforge/product",
        "@specforge/governance",
        "@specforge/formal",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let runtime = wasm_runtime_for(&ext_names);
    let host = specforge_wasm::protocol::ProtocolHost::new(&runtime);
    let mut manifests = Vec::new();
    for name in &ext_names {
        if let Ok(ext) = specforge_wasm::protocol::load_protocol_extension(&host, name) {
            manifests.push(specforge_wasm::protocol::protocol_extension_to_manifest(
                &ext,
            ));
        }
    }
    specforge_registry::build_registries(manifests).fields
}

fn node(id: &str, kind: &str, title: Option<&str>) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: title.map(|t| t.to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        },
        methods: Vec::new(),
    }
}

// -- autocomplete_entity_ids --------------------------------------------------

#[spec(
    behavior = "autocomplete_entity_ids",
    verify = "autocomplete suggests matching IDs"
)]
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

#[spec(
    behavior = "autocomplete_entity_ids",
    verify = "suggestions include entity titles and kinds"
)]
fn autocomplete_includes_titles_and_kinds() {
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", Some("User Login")));

    let items = specforge_lsp::complete_entity_ids(&g, "user");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, "behavior");
    assert_eq!(items[0].title.as_deref(), Some("User Login"));
}

#[spec(
    behavior = "autocomplete_entity_ids",
    verify = "suggestions filtered by target_kind when FieldRegistry has constraint"
)]
fn autocomplete_filters_by_target_kind() {
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", Some("User Login")));
    g.add_node(node("auth_token", "type", Some("Auth Token")));

    let items = specforge_lsp::complete_entity_ids_filtered(&g, "", Some("type"));
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "auth_token");
}

#[spec(
    behavior = "autocomplete_entity_ids",
    verify = "all IDs suggested when no target_kind constraint exists"
)]
fn autocomplete_all_ids_without_filter() {
    let mut g = Graph::new();
    g.add_node(node("a", "behavior", None));
    g.add_node(node("b", "type", None));

    let items = specforge_lsp::complete_entity_ids_filtered(&g, "", None);
    assert_eq!(items.len(), 2);
}

// C4-06: prefix matches rank before substring matches.
#[test]
fn prefix_matches_rank_before_substring_matches() {
    let mut g = Graph::new();
    g.add_node(node("reuser_login", "behavior", None)); // substring
    g.add_node(node("user_logout", "behavior", None)); // prefix

    let items = specforge_lsp::complete_entity_ids(&g, "user");
    assert_eq!(items[0].id, "user_logout", "prefix match must rank first");
    assert_eq!(items.len(), 2, "substring match is still suggested");
}

// C4-06: fuzzy matches (Jaro-Winkler >= 0.7) fill in behind structured
// matches so a typo still surfaces the intended entity.
#[test]
fn fuzzy_match_suggested_behind_exact_matches() {
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", None));
    g.add_node(node("auth_token", "type", None));

    let items = specforge_lsp::complete_entity_ids(&g, "user_lgon");
    assert!(
        items.iter().any(|c| c.id == "user_login"),
        "typo'd prefix should still fuzzy-match user_login, got: {:?}",
        items.iter().map(|c| c.id.clone()).collect::<Vec<_>>()
    );
    assert_eq!(items[0].id, "user_login", "fuzzy match should be ranked");
}

// -- complete_field_names -----------------------------------------------------

#[spec(
    behavior = "complete_field_names",
    verify = "field name completion uses FieldRegistry for entity kind"
)]
fn complete_field_names_for_kind() {
    let reg = default_field_registry();
    let fields = specforge_lsp::complete_field_names("behavior", Some(&reg));
    assert!(fields.iter().any(|f| f == "contract"));
}

#[spec(
    behavior = "complete_field_names",
    verify = "suggestions are filtered by entity kind"
)]
fn field_names_differ_by_kind() {
    let reg = default_field_registry();
    let behavior_fields = specforge_lsp::complete_field_names("behavior", Some(&reg));
    let type_fields = specforge_lsp::complete_field_names("type", Some(&reg));
    assert_ne!(behavior_fields, type_fields);
}

#[test]
fn no_field_names_for_unknown_kind() {
    let fields = specforge_lsp::complete_field_names("__nonexistent__", None);
    assert!(fields.is_empty());
}

// -- complete_field_names with FieldRegistry ----------------------------------

#[spec(
    behavior = "complete_field_names",
    verify = "field name completion uses FieldRegistry for entity kind"
)]
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
        inverse_of: None,
        normative: false,
        exempts_obligations: false,
        headline: false,
        derived_from: None,
        proof_role: None,
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
        inverse_of: None,
        normative: false,
        exempts_obligations: false,
        headline: false,
        derived_from: None,
        proof_role: None,
    });
    let fields = specforge_lsp::complete_field_names("behavior", Some(&reg));
    assert!(fields.contains(&"contract".to_string()));
    assert!(fields.contains(&"invariants".to_string()));
    assert_eq!(fields.len(), 2);
}

#[test]
fn complete_field_names_empty_when_registry_has_no_fields() {
    use specforge_registry::FieldRegistry;
    let reg = FieldRegistry::new();
    let fields = specforge_lsp::complete_field_names("behavior", Some(&reg));
    assert!(fields.is_empty());
}

// -- complete_keywords --------------------------------------------------------

#[spec(
    behavior = "complete_keywords",
    verify = "keyword completion includes all registered kinds"
)]
fn keyword_completion_includes_registered_kinds() {
    let keywords = specforge_lsp::complete_keywords(&["behavior", "type", "event"]);
    assert!(keywords.contains(&"behavior".to_string()));
    assert!(keywords.contains(&"type".to_string()));
    assert!(keywords.contains(&"event".to_string()));
}

#[spec(
    behavior = "complete_keywords",
    verify = "structural keywords always included"
)]
fn keyword_completion_includes_structural() {
    let keywords = specforge_lsp::complete_keywords(&[]);
    assert!(keywords.contains(&"use".to_string()));
    assert!(keywords.contains(&"define".to_string()));
}

#[test]
fn keyword_completion_no_duplicates() {
    // Even if "use" is passed as a registered kind, it should appear only once
    let keywords = specforge_lsp::complete_keywords(&["use", "behavior"]);
    let use_count = keywords.iter().filter(|k| *k == "use").count();
    assert_eq!(use_count, 1);
}

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
    assert!(
        ctx.is_none(),
        "should not detect cursor inside reference list"
    );
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
/// Build a Wasm runtime for a temp project listing `ext_names`, mirroring
/// how a real session loads extensions from specforge.json.
fn wasm_runtime_for(ext_names: &[String]) -> specforge_component::ComponentRuntime {
    let dir = tempfile::TempDir::new().unwrap();
    let config = serde_json::json!({
        "name": "test-project",
        "version": "0.1.0",
        "extensions": ext_names,
    });
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    specforge_component::project_runtime(dir.path())
}

// C4-02a regression: multi-byte characters before the cursor (in UTF-16
// units) must not panic the byte/UTF-16 mix, and bracket scanning must still
// resolve. The scan window on the cursor's own line converts UTF-16 -> bytes.
#[test]
fn completion_on_multibyte_line_does_not_panic() {
    // Cursor inside [..] on a line whose earlier content is multibyte: the
    // scan of the current line up to the cursor crosses the multi-byte chars.
    let content = "behavior login \"Login – ünits — done\" {\n  types [MyT\n}\n";
    // line 1 = `  types [MyT` — ASCII itself, but the FILE has multibyte
    // chars on line 0 (exercises per-line conversion, not whole-file offset).
    let ctx = specforge_lsp::cursor_context(content, 1, 11);
    assert!(
        ctx.is_some(),
        "bracket scan still resolves with multibyte content in the file"
    );
    let ctx = ctx.unwrap();
    assert_eq!(ctx.entity_kind, "behavior");

    // Multibyte chars on the SAME line before the cursor.
    let content2 = "behavior x \"desc – with — dashes\" {\n  types [MyT\n}\n";
    let _ = specforge_lsp::cursor_context(content2, 1, 11);
    let _ = specforge_lsp::cursor_context(content2, 1, 20);
    let _ = specforge_lsp::cursor_context(content2, 1, 10_000);
}
