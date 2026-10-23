//! Semantic tokens held to tree-sitter's grammar (ADR 0023 D1, D4).

use std::path::Path;

use specforge_lsp::{Document, MOD_DECLARATION, MOD_REFERENCE};
use specforge_ops::view::ProjectView;
use specforge_test_macros::test as spec;
use tree_sitter_specforge::{field, kind};

/// Whether `node` has an ancestor of kind `kind`.
fn within(node: tree_sitter::Node, kinds: &[&str]) -> bool {
    std::iter::successors(node.parent(), |p| p.parent()).any(|p| kinds.contains(&p.kind()))
}

/// Every node of `tree` under `node`, depth first.
fn walk<'t>(node: tree_sitter::Node<'t>, out: &mut Vec<tree_sitter::Node<'t>>) {
    out.push(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, out);
    }
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "classification agrees with the grammar on every spec file of the repository"
)]
fn tokens_agree_with_the_grammar() {
    let root = specforge_repo_root::repo_root();
    let project = specforge_project::testing::compile_at(
        root,
        Some(specforge_project::testing::component_runtime()),
    );
    let view = ProjectView::of(&project);
    let declared: Vec<&str> = view
        .registries()
        .kinds()
        .iter()
        .filter_map(|(_, entry)| entry.declared.semantic_token.as_deref())
        .collect();
    let mut files = Vec::new();
    crate::spec_files(&root.join("spec"), &mut files);
    assert!(files.len() > 150, "found only {} spec files", files.len());
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        agrees(path, &text, &view, &declared);
    }
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "classification agrees with the grammar on the forms the repository's spec does not write"
)]
#[ignore = "flipped by T4b"]
fn tokens_agree_with_the_grammar_on_the_forms_the_corpus_does_not_write() {
    let root = specforge_repo_root::repo_root();
    let project = specforge_project::testing::compile_at(
        root,
        Some(specforge_project::testing::component_runtime()),
    );
    let view = ProjectView::of(&project);
    let declared: Vec<&str> = view
        .registries()
        .kinds()
        .iter()
        .filter_map(|(_, entry)| entry.declared.semantic_token.as_deref())
        .collect();
    let fixture = root.join("crates/kernel/parser/tests/fixtures/lexemes.spec");
    let text = std::fs::read_to_string(&fixture).unwrap();
    agrees(&fixture, &text, &view, &declared);
}

/// Every token of `text` (read from `path`) agrees with the grammar's node it sits on.
fn agrees(path: &Path, text: &str, view: &ProjectView, declared: &[&str]) {
    let doc = Document::new(text.to_string());
    let index = doc.index();
    let tokens = doc.tokens(view);
    // Each token as (start byte, end byte) -> (type, modifiers).
    let mut spans = std::collections::HashMap::new();
    let mut previous_end = 0;
    for t in &tokens {
        let at = |character| {
            index
                .offset(tower_lsp::lsp_types::Position::new(t.line, character))
                .unwrap_or_else(|| panic!("{}: {t:?} leaves its line", path.display()))
        };
        let (start, end) = (at(t.col), at(t.col + t.length));
        assert!(start >= previous_end, "{}: {t:?} overlaps", path.display());
        previous_end = end;
        spans.insert((start, end), (t.token_type, t.modifiers));
    }
    let (_, tree) = specforge_parser::parse_incremental(text, "spec.spec", None);
    let tree = tree.unwrap();
    let mut nodes = Vec::new();
    walk(tree.root_node(), &mut nodes);
    let token_at = |node: tree_sitter::Node| spans.get(&(node.start_byte(), node.end_byte()));
    let where_ = |node: tree_sitter::Node| {
        format!(
            "{}:{} {:?}",
            path.display(),
            node.start_position().row + 1,
            &text[node.byte_range()]
        )
    };
    for node in nodes {
        if within(node, &[kind::DEFINE_BLOCK]) {
            continue;
        }
        match node.kind() {
            kind::ENTITY_BLOCK => {
                let kind = node.child_by_field_name(field::KIND).unwrap();
                assert_eq!(
                    token_at(kind).map(|t| t.0),
                    Some("type"),
                    "{}",
                    where_(kind)
                );
                let name = node.child_by_field_name(field::NAME).unwrap();
                let token = token_at(name).unwrap_or_else(|| panic!("{}", where_(name)));
                if &text[kind.byte_range()] == "defaults" {
                    // The name a defaults block gives is a kind, not a declaration (ADR 0149).
                    assert_eq!(token.1 & MOD_DECLARATION, 0, "{}", where_(name));
                } else {
                    assert_ne!(token.1 & MOD_DECLARATION, 0, "{}", where_(name));
                }
            }
            kind::FIELD => {
                let key = node.child_by_field_name(field::KEY).unwrap();
                // A verify line being typed (`verify string`, `verify true`) reads as the
                // statement it is becoming (ADR 0241 D3).
                let typed = &text[key.byte_range()] == "verify"
                    && !within(node, &[kind::NESTED_BLOCK])
                    && node
                        .child_by_field_name(field::VALUE)
                        .is_some_and(|v| matches!(v.kind(), kind::IDENTIFIER | kind::BOOLEAN))
                    && node.named_child_count() == 2;
                assert_eq!(
                    token_at(key).map(|t| t.0),
                    Some(if typed { "keyword" } else { "property" }),
                    "{}",
                    where_(key)
                );
            }
            kind::VERIFY_STATEMENT => {
                let first = node.child(0).unwrap();
                assert_eq!(
                    token_at(first).map(|t| t.0),
                    Some("keyword"),
                    "{}",
                    where_(first)
                );
                if let Some(kind) = node.child_by_field_name(field::KIND) {
                    assert_eq!(
                        token_at(kind).map(|t| t.0),
                        Some("enumMember"),
                        "{}",
                        where_(kind)
                    );
                }
            }
            kind::METHOD_STATEMENT => {
                let first = node.child(0).unwrap();
                assert_eq!(
                    token_at(first).map(|t| t.0),
                    Some("keyword"),
                    "{}",
                    where_(first)
                );
                let name = node.child_by_field_name(field::NAME).unwrap();
                let token = token_at(name).unwrap_or_else(|| panic!("{}", where_(name)));
                assert_eq!(token.0, "method", "{}", where_(name));
                assert_ne!(token.1 & MOD_DECLARATION, 0, "{}", where_(name));
            }
            kind::STRING | kind::TRIPLE_QUOTED_STRING | kind::COMMENT => {
                let expected = if node.kind() == kind::COMMENT {
                    "comment"
                } else {
                    "string"
                };
                // Each line of it is one token.
                let (mut from, end) = (node.start_byte(), node.end_byte());
                while from < end {
                    let line_end = text[from..end].find('\n').map_or(end, |at| from + at);
                    if from < line_end {
                        assert_eq!(
                            spans.get(&(from, line_end)).map(|t| t.0),
                            Some(expected),
                            "{}",
                            where_(node)
                        );
                    }
                    from = line_end + 1;
                }
            }
            kind::IDENTIFIER | kind::SCHEME_REF_ID
                if node.parent().is_some_and(|p| p.kind() == kind::LIST)
                    && !within(node, &[kind::NESTED_BLOCK]) =>
            {
                let token = token_at(node).unwrap_or_else(|| panic!("{}", where_(node)));
                assert_ne!(token.1 & MOD_REFERENCE, 0, "{}", where_(node));
                assert!(
                    token.0 == "variable" || token.0 == "function" || declared.contains(&token.0),
                    "{}: {token:?}",
                    where_(node)
                );
            }
            _ => {}
        }
    }
}
