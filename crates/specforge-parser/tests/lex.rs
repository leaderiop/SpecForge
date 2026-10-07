//! The one lexer of the `.spec` language (`specforge_parser::lex`),
//! checked against tree-sitter's grammar over the repository's spec.

use specforge_parser::lex::{Lexeme, LexemeKind, is_ref_id_prefix, lex, runs_into_text};
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
        kind::STRING | kind::TRIPLE_QUOTED_STRING => LexemeKind::Str { closed: true },
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    spec_files(&root.join("spec"), &mut files);
    assert!(files.len() > 150, "found only {} spec files", files.len());
    for corpus in ["integrations/rust/spec", "examples"] {
        spec_files(&root.join(corpus), &mut files);
    }
    // The forms of the language the corpora do not write; it must parse
    // without an error for the grammar's leaves to be the reference.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lexemes.spec");
    files.push(fixture.clone());
    let mut checked = 0;
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        let (file, tree) = parse_incremental(&text, &path.to_string_lossy(), None);
        let tree = tree.expect("a tree");
        if *path == fixture {
            assert!(file.errors.is_empty(), "the fixture: {:?}", file.errors);
            assert!(!tree.root_node().has_error(), "the fixture has a CST error");
        }
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
        .filter(|(kind, _)| matches!(kind, LexemeKind::Str { .. } | LexemeKind::Comment))
        .collect();
    assert_eq!(
        others,
        [
            (LexemeKind::Str { closed: true }, "\"a b\""),
            (LexemeKind::Comment, "// a c"),
            (LexemeKind::Str { closed: true }, "\"\"\"a\n\"q\" a\"\"\""),
            (LexemeKind::Str { closed: true }, "\"esc \\\" a\""),
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

#[specforge_test(
    behavior = "lex_spec_text",
    verify = "a string spanning lines is one lexeme, and one the grammar would not close ends at its line's end"
)]
fn an_unclosed_string_ends_at_its_lines_end() {
    let unclosed = LexemeKind::Str { closed: false };
    assert_eq!(
        lexemes("contract \"see\nrefs [a]"),
        [
            (LexemeKind::Ident, "contract"),
            (unclosed, "\"see"),
            (LexemeKind::Ident, "refs"),
            (LexemeKind::Punct('['), "["),
            (LexemeKind::Ident, "a"),
            (LexemeKind::Punct(']'), "]"),
        ]
    );
    assert_eq!(
        lexemes("d \"\"\"open\nstill"),
        [(LexemeKind::Ident, "d"), (unclosed, "\"\"\"open\nstill")],
        "an unclosed triple-quoted string runs to the end"
    );
    assert_eq!(
        lexemes("c \"a \\\nb"),
        [
            (LexemeKind::Ident, "c"),
            (unclosed, "\"a \\"),
            (LexemeKind::Ident, "b"),
        ],
        "a backslash before a line break is no escape"
    );
    assert_eq!(
        lexemes("c \"x"),
        [(LexemeKind::Ident, "c"), (unclosed, "\"x")],
        "a quote at the text's end"
    );
}

#[specforge_test(
    behavior = "lex_spec_text",
    verify = "a string spanning lines is one lexeme, and one the grammar would not close ends at its line's end"
)]
fn a_string_spanning_lines_is_one_lexeme() {
    assert_eq!(
        lexemes("d \"a\nb\" c"),
        [
            (LexemeKind::Ident, "d"),
            (LexemeKind::Str { closed: true }, "\"a\nb\""),
            (LexemeKind::Ident, "c"),
        ]
    );
    // The closing quote runs straight into text: the pairing an unclosed
    // quote shifts, so the first quote is the unclosed one.
    assert_eq!(
        lexemes("c \"a\n  b\"x"),
        [
            (LexemeKind::Ident, "c"),
            (LexemeKind::Str { closed: false }, "\"a"),
            (LexemeKind::Ident, "b"),
            (LexemeKind::Str { closed: false }, "\"x"),
        ]
    );
}

#[specforge_test(behavior = "lex_spec_text", verify = "a scheme ref ID is one lexeme")]
fn a_ref_id_cut_short_is_a_ref_id_prefix() {
    for text in [
        "gh",
        "gh.",
        "gh.issue",
        "gh.issue:",
        "gh.issue:42",
        "a.b:c.d:e",
    ] {
        assert!(is_ref_id_prefix(text), "{text:?}");
    }
    for text in ["", "9a", "a.b.c", "gh.issue:4 2", "gh.issue:\"x"] {
        assert!(!is_ref_id_prefix(text), "{text:?}");
    }
}

#[test]
fn what_runs_into_a_closing_quote_is_text() {
    let text = "\"a\"x \"b\" \"c\"]\"d\"/ \"e\"|";
    for (needle, runs) in [
        ("x", true),
        (" \"b", false),
        ("]", false),
        ("/", false),
        ("|", false),
    ] {
        let at = text.find(needle).unwrap();
        assert_eq!(runs_into_text(text, at), runs, "{needle:?}");
    }
    assert!(!runs_into_text(text, text.len()), "the end of the text");
}
