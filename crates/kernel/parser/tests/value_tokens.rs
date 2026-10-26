//! One reading of a value token (ADR 0247): what the parse stores is what `value_token` reads.

use specforge_parser::cst::Cst;
use specforge_parser::{FieldValue, Written, parse, value_token};
use specforge_test_macros::test as specforge_test;
use tree_sitter::Node;
use tree_sitter_specforge::{field, kind};

/// Every field's value node of the tree under `node`.
fn values<'t>(node: Node<'t>, out: &mut Vec<Node<'t>>) {
    if node.kind() == kind::FIELD {
        out.extend(node.child_by_field_name(field::VALUE));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        values(child, out);
    }
}

#[specforge_test(
    behavior = "parse_all_block_types",
    verify = "a value token reads to the text the parse stores: a quoted one unescaped, a triple-quoted one dedented, neither trimmed"
)]
fn a_value_token_reads_as_the_parse_stores_it() {
    for body in [
        "k \" draft\"",
        "k \"a\\\"b\"",
        "k \"tab\\there\"",
        "k \"\"\" x \"\"\"",
        "k \"\"\"\n    one\n    two\n  \"\"\"",
        "k bare",
        "k 7",
        "k -3",
        "k true",
        "k 2026-03-01",
        "k [\" a \", b, gh.issue:1, true, 3]",
    ] {
        let source = format!("thing t \"T\" {{\n  {body}\n}}\n");
        let file = parse(&source, "t.spec");
        assert!(file.errors.is_empty(), "{body}: {:?}", file.errors);
        let entry = &file.entities[0].fields.entries()[0];
        let cst = Cst::read(&source, None).unwrap();
        let mut nodes = Vec::new();
        for piece in cst.pieces() {
            values(piece.root(), &mut nodes);
        }
        assert_eq!(nodes.len(), 1, "{body}");
        let piece = &cst.pieces()[0];
        let node = nodes[0];
        match &entry.value {
            FieldValue::List(items) => {
                assert_eq!(value_token(node, piece.text()), None, "the list itself");
                let mut cursor = node.walk();
                let read: Vec<_> = node
                    .children(&mut cursor)
                    .filter_map(|c| value_token(c, piece.text()))
                    .collect();
                let stored: Vec<_> = items.iter().map(|i| (i.written, i.text.clone())).collect();
                assert_eq!(read, stored, "{body}");
            }
            FieldValue::String(s) => {
                assert_eq!(
                    value_token(node, piece.text()),
                    Some((Written::Quoted, s.clone()))
                );
            }
            FieldValue::Identifier(s) => {
                assert_eq!(
                    value_token(node, piece.text()),
                    Some((Written::Bare, s.clone()))
                );
            }
            FieldValue::Integer(n) => {
                assert_eq!(
                    value_token(node, piece.text()),
                    Some((Written::Literal, n.to_string()))
                );
            }
            FieldValue::Boolean(b) => {
                assert_eq!(
                    value_token(node, piece.text()),
                    Some((Written::Literal, b.to_string()))
                );
            }
            FieldValue::Date(d) => {
                assert_eq!(
                    value_token(node, piece.text()),
                    Some((Written::Literal, d.clone()))
                );
            }
            other => panic!("{body}: {other:?}"),
        }
    }
}
