//! The one lexer of the `.spec` language (`specforge_parser::lex`),
//! checked against tree-sitter's grammar over the repository's spec.

use specforge_parser::lex::{Lexeme, LexemeKind, lex};
use specforge_parser::parse_incremental;
use specforge_test_macros::test as specforge_test;
use std::path::{Path, PathBuf};
use tree_sitter_specforge::kind;

/// The `.spec` files under `dir`, recursively.
fn spec_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            spec_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "spec") {
            out.push(path);
        }
    }
}

/// The lexeme kind a grammar leaf of kind `node` is, when it is one the
/// lexer reads whole.
fn kind_of(node: &str) -> Option<LexemeKind> {
    Some(match node {
        kind::IDENTIFIER => LexemeKind::Ident,
        kind::SCHEME_REF_ID => LexemeKind::RefId,
        kind::STRING | kind::TRIPLE_QUOTED_STRING => LexemeKind::Str,
        kind::COMMENT => LexemeKind::Comment,
        kind::INTEGER => LexemeKind::Number,
        _ => return None,
    })
}

/// Every leaf of `node`'s tree the lexer must read whole, as
/// (start, end, kind).
fn grammar_leaves(node: tree_sitter::Node, out: &mut Vec<(usize, usize, LexemeKind)>) {
    if node.child_count() == 0 {
        if node.is_named()
            && let Some(kind) = kind_of(node.kind())
        {
            out.push((node.start_byte(), node.end_byte(), kind));
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        grammar_leaves(child, out);
    }
}

#[specforge_test(
    behavior = "lex_spec_text",
    verify = "the lexer agrees with the grammar on every spec file of the repository"
)]
fn lexer_agrees_with_the_grammar() {
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec");
    let mut files = Vec::new();
    spec_files(&spec, &mut files);
    assert!(files.len() > 150, "found only {} spec files", files.len());
    let mut checked = 0;
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        let (_, tree) = parse_incremental(&text, &path.to_string_lossy(), None);
        let tree = tree.expect("a tree");
        let mut leaves = Vec::new();
        grammar_leaves(tree.root_node(), &mut leaves);
        let lexemes = lex(&text);
        let by_start: std::collections::HashMap<usize, &Lexeme> =
            lexemes.iter().map(|l| (l.start, l)).collect();
        for (start, end, kind) in leaves {
            let lexeme = by_start.get(&start).unwrap_or_else(|| {
                panic!(
                    "{}: no lexeme at byte {start} for {kind:?} {:?}",
                    path.display(),
                    &text[start..end]
                )
            });
            assert_eq!(
                (lexeme.end, lexeme.kind),
                (end, kind),
                "{}: {:?} lexed as {:?}",
                path.display(),
                &text[start..end],
                lexeme.text(&text)
            );
            checked += 1;
        }
    }
    assert!(checked > 10_000, "checked only {checked} leaves");
}

/// The (kind, text) of each lexeme of `text`.
fn lexemes(text: &str) -> Vec<(LexemeKind, &str)> {
    lex(text)
        .into_iter()
        .map(|l| (l.kind, l.text(text)))
        .collect()
}

#[specforge_test(behavior = "lex_spec_text", verify = "a scheme ref ID is one lexeme")]
fn a_scheme_ref_id_is_one_lexeme() {
    assert_eq!(lexemes("gh.issue:42"), [(LexemeKind::RefId, "gh.issue:42")]);
    assert_eq!(
        lexemes("refs [jira.story:ABC-123, b]"),
        [
            (LexemeKind::Ident, "refs"),
            (LexemeKind::Punct('['), "["),
            (LexemeKind::RefId, "jira.story:ABC-123"),
            (LexemeKind::Punct(','), ","),
            (LexemeKind::Ident, "b"),
            (LexemeKind::Punct(']'), "]"),
        ]
    );
    assert_eq!(
        lexemes("a.b"),
        [
            (LexemeKind::Ident, "a"),
            (LexemeKind::Punct('.'), "."),
            (LexemeKind::Ident, "b"),
        ]
    );
    assert_eq!(
        lexemes("a.b: 1"),
        [
            (LexemeKind::Ident, "a"),
            (LexemeKind::Punct('.'), "."),
            (LexemeKind::Ident, "b"),
            (LexemeKind::Punct(':'), ":"),
            (LexemeKind::Number, "1"),
        ],
        "an ID needs at least one character"
    );
}

#[specforge_test(
    behavior = "lex_spec_text",
    verify = "strings and comments are lexemes of their own and hold no others"
)]
fn strings_and_comments_are_lexemes_of_their_own() {
    let text = "behavior a \"a b\" { // a c\n  x [a] \"\"\"a\n\"q\" a\"\"\" y \"esc \\\" a\" z\n}";
    let names: Vec<&str> = lex(text)
        .into_iter()
        .filter(Lexeme::is_name)
        .map(|l| l.text(text))
        .collect();
    assert_eq!(names, ["behavior", "a", "x", "a", "y", "z"]);
    let others: Vec<(LexemeKind, &str)> = lexemes(text)
        .into_iter()
        .filter(|(kind, _)| matches!(kind, LexemeKind::Str | LexemeKind::Comment))
        .collect();
    assert_eq!(
        others,
        [
            (LexemeKind::Str, "\"a b\""),
            (LexemeKind::Comment, "// a c"),
            (LexemeKind::Str, "\"\"\"a\n\"q\" a\"\"\""),
            (LexemeKind::Str, "\"esc \\\" a\""),
        ]
    );
}

#[test]
fn a_word_is_never_split() {
    let text = "x_sesion_limit é→a sesion";
    let names: Vec<&str> = lex(text)
        .into_iter()
        .filter(Lexeme::is_name)
        .map(|l| l.text(text))
        .collect();
    assert_eq!(names, ["x_sesion_limit", "é→a", "sesion"]);
}

#[test]
fn numbers_start_with_a_digit() {
    assert_eq!(
        lexemes("a < 10ms and b > 1.5 x -3"),
        [
            (LexemeKind::Ident, "a"),
            (LexemeKind::Punct('<'), "<"),
            (LexemeKind::Number, "10ms"),
            (LexemeKind::Ident, "and"),
            (LexemeKind::Ident, "b"),
            (LexemeKind::Punct('>'), ">"),
            (LexemeKind::Number, "1.5"),
            (LexemeKind::Ident, "x"),
            (LexemeKind::Punct('-'), "-"),
            (LexemeKind::Number, "3"),
        ]
    );
}

#[test]
fn an_unclosed_string_ends_at_its_lines_end() {
    assert_eq!(
        lexemes("contract \"see\nrefs [a]"),
        [
            (LexemeKind::Ident, "contract"),
            (LexemeKind::Str, "\"see"),
            (LexemeKind::Ident, "refs"),
            (LexemeKind::Punct('['), "["),
            (LexemeKind::Ident, "a"),
            (LexemeKind::Punct(']'), "]"),
        ]
    );
    assert_eq!(
        lexemes("d \"\"\"open\nstill"),
        [
            (LexemeKind::Ident, "d"),
            (LexemeKind::Str, "\"\"\"open\nstill"),
        ],
        "an unclosed triple-quoted string runs to the end"
    );
}

// Pin: flipped by plan 15 T4.
#[test]
fn a_string_spanning_lines_ends_at_its_first_lines_end_today() {
    assert_eq!(
        lexemes("d \"a\nb\" c"),
        [
            (LexemeKind::Ident, "d"),
            (LexemeKind::Str, "\"a"),
            (LexemeKind::Ident, "b"),
            (LexemeKind::Str, "\" c"),
        ]
    );
}
