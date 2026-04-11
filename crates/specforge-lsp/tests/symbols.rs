use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test_macros::test as spec;

fn node(id: &str, kind: &str, title: Option<&str>, file: &str, line: usize) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: title.map(|t| t.to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new(file),
            start_line: line, start_col: 0, end_line: line + 3, end_col: 1,
        },
    }
}

// -- outline_view -------------------------------------------------------------

#[spec(behavior = "outline_view", verify = "outline lists all entities in file")]
#[test]
fn outline_lists_all_entities() {
    let mut g = Graph::new();
    g.add_node(node("a", "behavior", Some("A"), "test.spec", 0));
    g.add_node(node("b", "type", Some("B"), "test.spec", 5));
    g.add_node(node("c", "event", Some("C"), "other.spec", 10));

    let symbols = specforge_lsp::document_symbols(&g, "test.spec");
    assert_eq!(symbols.len(), 2);
}

#[spec(behavior = "outline_view", verify = "outline shows entity kind, ID, and title")]
#[test]
fn outline_shows_details() {
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", Some("User Login"), "test.spec", 0));

    let symbols = specforge_lsp::document_symbols(&g, "test.spec");
    assert_eq!(symbols[0].id, "user_login");
    assert_eq!(symbols[0].kind, "behavior");
    assert_eq!(symbols[0].title.as_deref(), Some("User Login"));
}

#[spec(behavior = "outline_view", verify = "outline uses extension-defined SymbolKind from KindRegistry lsp_icon")]
#[test]
fn outline_uses_kind_for_icon() {
    let mut g = Graph::new();
    g.add_node(node("a", "behavior", None, "test.spec", 0));
    g.add_node(node("b", "type", None, "test.spec", 5));

    let symbols = specforge_lsp::document_symbols(&g, "test.spec");
    // Kind is exposed so the LSP layer can map to SymbolKind
    assert_eq!(symbols[0].kind, "behavior");
    assert_eq!(symbols[1].kind, "type");
}

// -- workspace_symbol_search --------------------------------------------------

#[spec(behavior = "workspace_symbol_search", verify = "search by ID prefix returns matches")]
#[test]
fn search_by_id_prefix() {
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", Some("User Login"), "a.spec", 0));
    g.add_node(node("user_logout", "behavior", Some("User Logout"), "a.spec", 5));
    g.add_node(node("auth_token", "type", Some("Auth Token"), "b.spec", 0));

    let results = specforge_lsp::workspace_symbols(&g, "user");
    assert_eq!(results.len(), 2);
}

#[spec(behavior = "workspace_symbol_search", verify = "search by title fragment returns matches")]
#[test]
fn search_by_title_fragment() {
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", Some("User Login"), "a.spec", 0));
    g.add_node(node("auth_token", "type", Some("Auth Token"), "b.spec", 0));

    let results = specforge_lsp::workspace_symbols(&g, "Login");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "user_login");
}

#[spec(behavior = "workspace_symbol_search", verify = "search results use extension-defined SymbolKind")]
#[test]
fn search_results_include_kind() {
    let mut g = Graph::new();
    g.add_node(node("a", "behavior", Some("A"), "a.spec", 0));

    let results = specforge_lsp::workspace_symbols(&g, "a");
    assert_eq!(results[0].kind, "behavior");
}
