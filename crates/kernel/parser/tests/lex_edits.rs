//! The lexer's slot reader against the grammar over text a user could type mid-edit (ADR 0285).

use proptest::test_runner::TestRunner;
use specforge_parser::lex::lex;
use specforge_parser::testing::{edited_corpus, property_config};
use specforge_test_macros::test as specforge_test;
use tree_sitter_specforge::kind;

#[specforge_test(
    behavior = "lex_spec_text",
    verify = "the lexer agrees with the grammar on every top-level form an edit of the repository's spec leaves intact"
)]
fn the_lexer_agrees_with_the_grammar_on_every_edited_form() {
    let checked = std::cell::Cell::new(0usize);
    let mut runner = TestRunner::new(property_config());
    let result = runner.run(&edited_corpus(), |case| {
        let text = &case.edited;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_specforge::LANGUAGE.into())
            .unwrap();
        let Some(tree) = parser.parse(text, None) else {
            return Ok(());
        };
        let lexemes = lex(text);
        let by_start: std::collections::HashMap<usize, _> =
            lexemes.iter().map(|l| (l.start, l)).collect();
        let mut cursor = tree.root_node().walk();
        for form in tree.root_node().children(&mut cursor) {
            // A form with an ERROR or MISSING node is the recovery's, not the grammar's reading, and what
            // follows it may pair strings differently: the lexer's string pairing after a broken form is not
            // what this test holds it to.
            // Recovery leaves stray tokens among the forms: anything that is no top-level form ends the walk.
            let is_form = matches!(
                form.kind(),
                kind::COMMENT
                    | kind::USE_IMPORT
                    | kind::PUB_USE_IMPORT
                    | kind::SPEC_BLOCK
                    | kind::REF_INLINE
                    | kind::REF_FULL
                    | kind::DEFINE_BLOCK
                    | kind::UNION_BLOCK
                    | kind::ENTITY_BLOCK
            );
            if !is_form || form.is_error() || form.has_error() || form.is_missing() {
                break;
            }
            let mut leaves = Vec::new();
            crate::lex::grammar_leaves(form, text, &mut leaves);
            // A string running across lines is where the lexer deliberately reads half-typed text its own way
            // (`LexemeKind::Str`): neither it nor what follows it is held to this test, and strings are not.
            if leaves
                .iter()
                .any(|(start, end, kind)| kind.is_str() && text[*start..*end].contains('\n'))
            {
                continue;
            }
            // A non-ASCII byte beside a word is no part of the grammar's word and all of the lexer's (ADR 0038):
            // not what this test holds them to.
            let bytes = text.as_bytes();
            let ascii_around = |start: usize, end: usize| {
                bytes.get(end).is_none_or(u8::is_ascii)
                    && (start == 0 || bytes[start - 1].is_ascii())
            };
            for (start, end, kind) in leaves
                .into_iter()
                .filter(|(start, end, kind)| !kind.is_str() && ascii_around(*start, *end))
            {
                checked.set(checked.get() + 1);
                let found = by_start.get(&start).map(|l| (l.end, l.kind));
                proptest::prop_assert_eq!(
                    found,
                    Some((end, kind)),
                    "{:?} at {} lexed differently, in {:?}",
                    &text[start..end],
                    start,
                    &text[text.floor_char_boundary(start.saturating_sub(100))
                        ..text.ceil_char_boundary((end + 40).min(text.len()))]
                );
            }
        }
        Ok(())
    });
    if let Err(failure) = result {
        panic!("{failure}");
    }
    assert!(
        checked.get() > 10_000,
        "checked only {} leaves",
        checked.get()
    );
}
