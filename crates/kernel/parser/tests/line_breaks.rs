//! A line break has one reader (ADR 0266): a LF, with a CR directly before it part of it.

use specforge_parser::lex::{LexemeKind, lex};
use specforge_parser::{FieldValue, parse};
use specforge_test_macros::test as specforge_test;

/// Block text (whole lines, `\n`) with each `␍` standing for a raw CR.
fn raw(text: &str) -> String {
    text.replace('␍', "\r")
}

/// `body` as a `feature` block in an LF file and in a CRLF one.
fn both_layouts(header: &str, body: &str) -> [String; 2] {
    let lf = raw(&format!("{header} {{\n{body}}}\n"));
    let crlf = lf.replace("\r\n", "\n").replace('\n', "\r\n");
    [lf, crlf]
}

#[specforge_test(
    behavior = "parse_all_block_types",
    verify = "a raw CR LF inside a quoted text reads as LF, as inside a triple-quoted one"
)]
fn a_raw_cr_lf_inside_a_quoted_text_reads_as_lf() {
    let field = |value: &str| both_layouts("feature f1 \"F1\"", &format!("  problem {value}\n"));
    for source in field("\"a␍\nb\"") {
        let file = parse(&source, "t.spec");
        assert!(file.errors.is_empty(), "{:?}", file.errors);
        let Some(FieldValue::String(text)) = file.entities[0].fields.get("problem") else {
            panic!("no string");
        };
        assert_eq!(text, "a\nb", "a field value in {source:?}");
    }
    for source in field("[\"a␍\nb\"]") {
        let file = parse(&source, "t.spec");
        let Some(FieldValue::List(items)) = file.entities[0].fields.get("problem") else {
            panic!("no list");
        };
        assert_eq!(items[0].text, "a\nb", "a list item in {source:?}");
    }
    for source in both_layouts("feature f1 \"F␍\none\"", "  problem \"p\"\n") {
        let file = parse(&source, "t.spec");
        assert_eq!(
            file.entities[0].title.as_deref(),
            Some("F\none"),
            "{source:?}"
        );
    }
    for source in both_layouts("feature f1 \"F1\"", "  problem \"p\" @doc \"a␍\nb\"\n") {
        let file = parse(&source, "t.spec");
        let note = &file.entities[0].fields.entries()[0].annotations[0];
        assert_eq!(note.value.as_deref(), Some("a\nb"), "{source:?}");
    }
    for source in both_layouts("feature f1 \"F1\"", "  verify unit \"a␍\nb\"\n") {
        let file = parse(&source, "t.spec");
        assert_eq!(
            file.entities[0].obligations[0].description, "a\nb",
            "{source:?}"
        );
    }
}

#[specforge_test(
    behavior = "parse_all_block_types",
    verify = "an escaped CR and a lone CR inside a quoted text stay a CR"
)]
fn an_escaped_cr_and_a_lone_cr_inside_a_quoted_text_stay_a_cr() {
    let read = |source: &str| match parse(source, "t.spec").entities[0].fields.get("problem") {
        Some(FieldValue::String(text)) => text.clone(),
        other => panic!("{other:?}"),
    };
    for source in both_layouts("feature f1 \"F1\"", "  problem \"a\\r\\nb\"\n") {
        assert_eq!(read(&source), "a\r\nb", "{source:?}");
    }
    for source in both_layouts("feature f1 \"F1\"", "  problem \"a␍b\"\n") {
        assert_eq!(read(&source), "a\rb", "{source:?}");
    }
}

#[specforge_test(
    behavior = "recover_from_syntax_errors",
    verify = "a backslash before a line break leaves a string unclosed, as the grammar reads it"
)]
fn a_backslash_before_a_cr_lf_leaves_the_string_unclosed() {
    let source = "behavior a \"A\" {\r\n  contract \"line one \\\r\n  line two\"\r\n}\r\n\r\nbehavior b \"B\" {\r\n  contract \"fine\"\r\n}\r\n";
    let result = parse(source, "test.spec");
    assert!(
        result.entities.iter().any(|e| e.id.raw == "b"),
        "entities: {:?}, errors: {:?}",
        result.entities,
        result.errors
    );
    assert_eq!(result.errors.len(), 1, "errors: {:?}", result.errors);
    let span = &result.errors[0].span;
    assert_eq!((span.start_line, span.start_col), (2, 12));

    let text = "c \"a \\\r\nb";
    let lexemes: Vec<_> = lex(text)
        .into_iter()
        .map(|l| (l.kind, l.text(text)))
        .collect();
    assert_eq!(
        lexemes,
        [
            (LexemeKind::Ident, "c"),
            (LexemeKind::Str { closed: false }, "\"a \\\r"),
            (LexemeKind::Ident, "b"),
        ]
    );
}
