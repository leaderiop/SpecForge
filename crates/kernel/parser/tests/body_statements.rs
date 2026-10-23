//! The lexer's body-statement rule (`specforge_parser::lex::body_statement_at`, ADR 0241)
//! held to tree-sitter's grammar.

use specforge_parser::lex::{BodyStatement, body_statement_at, lex};
use specforge_parser::parse_incremental;
use specforge_test_macros::test as specforge_test;
use std::path::Path;
use tree_sitter_specforge::{field, kind};

/// Every node of the tree under `node`, depth first.
fn walk<'t>(node: tree_sitter::Node<'t>, out: &mut Vec<tree_sitter::Node<'t>>) {
    out.push(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, out);
    }
}

/// Whether `node` sits in a nested block.
fn within_nested_block(node: tree_sitter::Node) -> bool {
    std::iter::successors(node.parent(), |p| p.parent()).any(|p| p.kind() == kind::NESTED_BLOCK)
}

#[specforge_test(
    behavior = "lex_spec_text",
    verify = "a body line starts a verify or method statement exactly where the grammar reads one, on every spec file of the repository"
)]
fn body_statements_are_the_grammars() {
    let root = specforge_repo_root::repo_root();
    let mut files = Vec::new();
    for corpus in ["spec", "integrations/rust/spec", "examples"] {
        crate::lex::spec_files(&root.join(corpus), &mut files);
    }
    // The forms of the language the corpora do not write.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lexemes.spec");
    files.push(fixture.clone());
    let mut checked = 0;
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        let (file, tree) = parse_incremental(&text, &path.to_string_lossy(), None);
        let tree = tree.expect("a tree");
        if *path == fixture {
            assert!(file.errors.is_empty(), "the fixture: {:?}", file.errors);
        }
        if tree.root_node().has_error() {
            continue;
        }
        let lexemes = lex(&text);
        let mut nodes = Vec::new();
        walk(tree.root_node(), &mut nodes);
        for node in nodes {
            let word = match node.kind() {
                kind::VERIFY_STATEMENT | kind::METHOD_STATEMENT => node.start_byte(),
                // A nested block holds fields only; the caller knows it from its frames.
                kind::FIELD if !within_nested_block(node) => {
                    node.child_by_field_name(field::KEY).unwrap().start_byte()
                }
                _ => continue,
            };
            let i = lexemes.iter().position(|l| l.start == word).unwrap();
            let got = body_statement_at(&text, &lexemes, i);
            let at = format!(
                "{}:{} {:?}",
                path.display(),
                node.start_position().row + 1,
                &text[node.byte_range()]
            );
            match node.kind() {
                kind::VERIFY_STATEMENT => assert_eq!(got, BodyStatement::Verify, "{at}"),
                kind::METHOD_STATEMENT => assert_eq!(got, BodyStatement::Method, "{at}"),
                _ => assert!(
                    !matches!(got, BodyStatement::Verify | BodyStatement::Method),
                    "{at}"
                ),
            }
            checked += 1;
        }
    }
    assert!(checked > 5_000, "checked only {checked} statements");
}

#[specforge_test(
    behavior = "lex_spec_text",
    verify = "a verify line being typed is told apart from a verify statement and from a field"
)]
fn a_verify_line_being_typed_is_told_apart() {
    use BodyStatement::{Field, Method, Verify, VerifyTyped};
    let rows: [(&str, BodyStatement); 16] = [
        ("verify \"x\"", Verify),
        ("verify unit \"x\"", Verify),
        ("verify unit \"x", Verify),
        ("verify unit\n\"x\"", Verify),
        ("verify\n\"x\"", Verify),
        ("verify", VerifyTyped),
        ("verify unit", VerifyTyped),
        ("verify string", VerifyTyped),
        ("verify true", VerifyTyped),
        ("verify\ncontract \"c\"", VerifyTyped),
        ("verify \"\"\"x\"\"\"", Field),
        ("verify unit \"\"\"x\"\"\"", Field),
        ("verify string @optional", Field),
        ("verify 42", Field),
        ("method get(id: string)", Method),
        ("method get", Method),
    ];
    for (text, want) in rows {
        assert_eq!(body_statement_at(text, &lex(text), 0), want, "{text:?}");
    }
    for text in ["method", "method a b"] {
        assert_eq!(body_statement_at(text, &lex(text), 0), Field, "{text:?}");
    }
}
