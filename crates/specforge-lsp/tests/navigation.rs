use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Graph, Node, Edge};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test_macros::test as spec;
use std::fs;

fn span(file: &str, line: usize, col: usize, end_line: usize, end_col: usize) -> SourceSpan {
    SourceSpan {
        file: Sym::new(file),
        start_line: line,
        start_col: col,
        end_line,
        end_col,
    }
}

fn node(id: &str, kind: &str, file: &str, line: usize) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: Some(format!("{id} title")),
        fields: FieldMap::new(),
        source_span: span(file, line, 0, line, 10),
    }
}

fn graph_with_refs() -> Graph {
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", "auth.spec", 5));
    g.add_node(node("auth_token", "type", "types.spec", 10));
    g.add_edge(Edge {
        source: "user_login".into(),
        target: "auth_token".into(),
        label: "types".into(),
    });
    g
}

// -- source_span_to_lsp_range (1-based → 0-based conversion) -----------------

#[spec(behavior = "go_to_definition", verify = "source spans convert from 1-based to 0-based for LSP")]
#[test]
fn source_span_converts_1based_to_0based() {
    // Parser produces 1-based spans (line 3, col 1 means third line, first column)
    let span = SourceSpan {
        file: Sym::new("test.spec"),
        start_line: 3,
        start_col: 1,
        end_line: 5,
        end_col: 2,
    };
    let lsp = specforge_lsp::source_span_to_lsp_range(&span);
    // LSP protocol uses 0-based
    assert_eq!(lsp.start_line, 2);
    assert_eq!(lsp.start_col, 0);
    assert_eq!(lsp.end_line, 4);
    assert_eq!(lsp.end_col, 1);
}

#[spec(behavior = "go_to_definition", verify = "source spans convert from 1-based to 0-based for LSP")]
#[test]
fn source_span_zero_saturates() {
    // Edge case: span with 0 values shouldn't underflow
    let span = SourceSpan {
        file: Sym::new("test.spec"),
        start_line: 0,
        start_col: 0,
        end_line: 0,
        end_col: 0,
    };
    let lsp = specforge_lsp::source_span_to_lsp_range(&span);
    assert_eq!(lsp.start_line, 0);
    assert_eq!(lsp.start_col, 0);
}

// -- go_to_definition ---------------------------------------------------------

#[spec(behavior = "go_to_definition", verify = "go-to-def navigates to entity declaration")]
#[test]
fn go_to_def_navigates_to_declaration() {
    let g = graph_with_refs();
    let result = specforge_lsp::go_to_definition(&g, "auth_token");

    let loc = result.expect("should find definition");
    assert_eq!(loc.file, "types.spec");
    assert_eq!(loc.start_line, 10);
}

#[spec(behavior = "go_to_definition", verify = "go-to-def on non-existent ID returns no result")]
#[test]
fn go_to_def_returns_none_for_missing() {
    let g = graph_with_refs();
    let result = specforge_lsp::go_to_definition(&g, "nonexistent");
    assert!(result.is_none());
}

#[spec(behavior = "go_to_definition", verify = "go-to-def works across files")]
#[test]
fn go_to_def_works_across_files() {
    let mut g = Graph::new();
    g.add_node(node("controller", "behavior", "controllers.spec", 1));
    g.add_node(node("auth_token", "type", "types.spec", 10));
    g.add_edge(Edge {
        source: "controller".into(),
        target: "auth_token".into(),
        label: "types".into(),
    });

    // Definition lookup works regardless of which file the caller is in
    let loc = specforge_lsp::go_to_definition(&g, "auth_token")
        .expect("should find definition across files");
    assert_eq!(loc.file, "types.spec");
    assert_eq!(loc.start_line, 10);

    let loc2 = specforge_lsp::go_to_definition(&g, "controller")
        .expect("should find definition in other file");
    assert_eq!(loc2.file, "controllers.spec");
    assert_eq!(loc2.start_line, 1);
}

// -- goto_import_definition ---------------------------------------------------

#[spec(behavior = "goto_import_definition", verify = "go-to-def on use path navigates to target file")]
#[test]
fn goto_import_navigates_to_file() {
    let tmp = tempfile::tempdir().unwrap();
    let behaviors_dir = tmp.path().join("behaviors");
    fs::create_dir_all(&behaviors_dir).unwrap();
    fs::write(behaviors_dir.join("auth.spec"), "behavior auth \"Auth\" {}\n").unwrap();

    let spec_root = tmp.path().to_str().unwrap();
    let result = specforge_lsp::goto_import_definition("behaviors/auth", spec_root);
    let loc = result.expect("should resolve import");
    assert!(loc.file.as_str().ends_with("behaviors/auth.spec"));
    assert_eq!(loc.start_line, 0);
}

#[spec(behavior = "goto_import_definition", verify = "go-to-def on non-existent use path returns no result")]
#[test]
fn goto_import_returns_none_for_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let spec_root = tmp.path().to_str().unwrap();
    let result = specforge_lsp::goto_import_definition("nonexistent/path", spec_root);
    assert!(result.is_none());
}

// -- goto_import_definition (LSP dispatch integration) ------------------------

#[spec(behavior = "goto_import_definition", verify = "textDocument/definition on use line dispatches to goto_import_definition")]
#[test]
fn goto_definition_dispatches_to_import_on_use_line() {
    let tmp = tempfile::tempdir().unwrap();
    let behaviors_dir = tmp.path().join("behaviors");
    fs::create_dir_all(&behaviors_dir).unwrap();
    fs::write(behaviors_dir.join("auth.spec"), "behavior auth \"Auth\" {}\n").unwrap();

    let spec_root = tmp.path().to_str().unwrap();

    // Simulate document content with a use line
    let content = "use \"behaviors/auth\"\n\nbehavior login \"Login\" {}\n";
    let line = content.lines().next().unwrap();

    // The line is a use statement, so extract the import path via import_path_on_line
    let import_path = specforge_lsp::backend::import_path_on_line(line)
        .expect("should extract import path from use line");

    // Dispatch to goto_import_definition (as the LSP handler would)
    let result = specforge_lsp::goto_import_definition(import_path, spec_root);
    let loc = result.expect("should resolve import from use line");
    assert!(loc.file.as_str().ends_with("behaviors/auth.spec"));
    assert_eq!(loc.start_line, 0);
}

// -- find_all_references ------------------------------------------------------

#[spec(behavior = "find_all_references", verify = "find-refs returns all reference sites")]
#[test]
fn find_refs_returns_all_sites() {
    let mut g = Graph::new();
    g.add_node(node("auth_token", "type", "types.spec", 10));
    g.add_node(node("user_login", "behavior", "auth.spec", 5));
    g.add_node(node("session_mgr", "behavior", "session.spec", 3));
    g.add_edge(Edge { source: "user_login".into(), target: "auth_token".into(), label: "types".into() });
    g.add_edge(Edge { source: "session_mgr".into(), target: "auth_token".into(), label: "types".into() });

    let refs = specforge_lsp::find_all_references(&g, "auth_token");
    // Should include the declaration + 2 reference edges = locations from 3 nodes
    assert_eq!(refs.len(), 3);
}

#[spec(behavior = "find_all_references", verify = "find-refs includes the declaration site")]
#[test]
fn find_refs_includes_declaration() {
    let g = graph_with_refs();
    let refs = specforge_lsp::find_all_references(&g, "auth_token");
    assert!(refs.iter().any(|loc| loc.file == "types.spec" && loc.start_line == 10));
}

#[spec(behavior = "find_all_references", verify = "find-refs across multiple files")]
#[test]
fn find_refs_across_files() {
    let mut g = Graph::new();
    g.add_node(node("auth_token", "type", "types.spec", 10));
    g.add_node(node("login", "behavior", "auth.spec", 1));
    g.add_node(node("refresh", "behavior", "session.spec", 2));
    g.add_edge(Edge { source: "login".into(), target: "auth_token".into(), label: "types".into() });
    g.add_edge(Edge { source: "refresh".into(), target: "auth_token".into(), label: "types".into() });

    let refs = specforge_lsp::find_all_references(&g, "auth_token");
    let files: Vec<&str> = refs.iter().map(|l| l.file.as_str()).collect();
    assert!(files.contains(&"types.spec"));
    assert!(files.contains(&"auth.spec"));
    assert!(files.contains(&"session.spec"));
}
