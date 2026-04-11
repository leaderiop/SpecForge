use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Graph, Node, Edge};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test_macros::test as spec;

fn span(file: &str, line: usize, col: usize, end_col: usize) -> SourceSpan {
    SourceSpan {
        file: Sym::new(file),
        start_line: line, start_col: col,
        end_line: line, end_col,
    }
}

fn node_at(id: &str, kind: &str, file: &str, line: usize, col: usize) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: None,
        fields: FieldMap::new(),
        source_span: span(file, line, col, col + id.len()),
    }
}

fn graph_with_refs() -> Graph {
    let mut g = Graph::new();
    g.add_node(node_at("auth_token", "type", "types.spec", 5, 5));
    g.add_node(node_at("user_login", "behavior", "auth.spec", 10, 9));
    g.add_edge(Edge {
        source: "user_login".into(), target: "auth_token".into(), label: "types".into(),
    });
    g
}

// -- prepare_rename -----------------------------------------------------------

#[spec(behavior = "prepare_rename", verify = "prepare rename on entity ID returns token range")]
#[test]
fn prepare_rename_returns_range() {
    let g = graph_with_refs();
    let result = specforge_lsp::prepare_rename(&g, "auth_token");
    let range = result.expect("should return range");
    assert_eq!(range.file, "types.spec");
    assert_eq!(range.start_line, 5);
    assert_eq!(range.start_col, 5);
    assert_eq!(range.end_col, 5 + "auth_token".len());
}

#[spec(behavior = "prepare_rename", verify = "prepare rename on non-renameable token returns not available")]
#[test]
fn prepare_rename_returns_none_for_missing() {
    let g = graph_with_refs();
    let result = specforge_lsp::prepare_rename(&g, "nonexistent");
    assert!(result.is_none());
}

// -- rename_entity_id ---------------------------------------------------------

#[spec(behavior = "rename_entity_id", verify = "rename updates declaration and all references")]
#[test]
fn rename_updates_all_sites() {
    let g = graph_with_refs();
    let edits = specforge_lsp::compute_rename_edits(&g, "auth_token", "session_token");
    let edits = edits.expect("should produce edits");
    // Declaration (types.spec) + reference from user_login (auth.spec)
    assert!(edits.len() >= 2);
    assert!(edits.iter().any(|e| e.file == "types.spec"));
    assert!(edits.iter().any(|e| e.file == "auth.spec"));
}

#[spec(behavior = "rename_entity_id", verify = "rename is atomic — all or nothing")]
#[test]
fn rename_is_atomic() {
    let g = graph_with_refs();
    // Valid rename produces all edits at once
    let edits = specforge_lsp::compute_rename_edits(&g, "auth_token", "session_token");
    assert!(edits.is_some());
    // All edits are returned together (atomicity is at the edit-set level)
    let edits = edits.unwrap();
    assert!(edits.len() >= 2);
}

#[spec(behavior = "rename_entity_id", verify = "rename across multiple files")]
#[test]
fn rename_across_files() {
    let mut g = Graph::new();
    g.add_node(node_at("tok", "type", "a.spec", 0, 5));
    g.add_node(node_at("b1", "behavior", "b.spec", 0, 9));
    g.add_node(node_at("b2", "behavior", "c.spec", 0, 9));
    g.add_edge(Edge { source: "b1".into(), target: "tok".into(), label: "types".into() });
    g.add_edge(Edge { source: "b2".into(), target: "tok".into(), label: "types".into() });

    let edits = specforge_lsp::compute_rename_edits(&g, "tok", "token").unwrap();
    let files: Vec<&str> = edits.iter().map(|e| e.file.as_str()).collect();
    assert!(files.contains(&"a.spec"));
    assert!(files.contains(&"b.spec"));
    assert!(files.contains(&"c.spec"));
}

#[spec(behavior = "rename_entity_id", verify = "rename rejects new name that duplicates existing entity ID")]
#[test]
fn rename_rejects_duplicate() {
    let g = graph_with_refs();
    let edits = specforge_lsp::compute_rename_edits(&g, "auth_token", "user_login");
    assert!(edits.is_none(), "rename to existing ID should be rejected");
}
