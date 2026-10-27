//! A reference value, a list item and a use binding may be a path of names joined by `/`, optionally rooted at the
//! project (ADR 0284, plan K2).

use specforge_parser::lex::{LexemeKind, lex};
use specforge_parser::{FieldValue, Written, parse};
use specforge_test_macros::test as specforge_test;

/// The lexemes of `text` as (kind, text), without whitespace.
fn read(text: &str) -> Vec<(LexemeKind, &str)> {
    lex(text).iter().map(|l| (l.kind, l.text(text))).collect()
}

fn items(source: &str) -> Vec<(String, Written)> {
    let file = parse(source, "t.spec");
    assert!(file.errors.is_empty(), "{source}: {:?}", file.errors);
    let FieldValue::List(items) = &file.entities[0].fields.entries()[0].value else {
        panic!("a list: {source}");
    };
    items.iter().map(|i| (i.text.clone(), i.written)).collect()
}

#[specforge_test(
    behavior = "parse_all_block_types",
    verify = "a reference value, a list item and a use binding may be a path of names joined by '/', rooted or not"
)]
#[ignore = "K2 T4"]
fn a_reference_value_may_be_a_path() {
    for word in ["a/b", "/D-09", "v1.0/03/D-01", "/v1.0/03", "03/D-01"] {
        let file = parse(&format!("thing x \"X\" {{\n  v {word}\n}}\n"), "t.spec");
        assert!(file.errors.is_empty(), "{word}: {:?}", file.errors);
        assert_eq!(
            file.entities[0].fields.entries()[0].value,
            FieldValue::Identifier(word.into()),
            "{word}"
        );
    }
    let read_items = items("thing x \"X\" {\n  v [v1.0/03/D-01, /v1.0/04, 03-02]\n}\n");
    let texts: Vec<&str> = read_items.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(texts, ["v1.0/03/D-01", "/v1.0/04", "03-02"]);
    assert!(read_items.iter().all(|(_, w)| *w == Written::Bare));

    let file = parse("use { 03/D-01, /D-09 as top } from \"./gsd\"\n", "t.spec");
    assert!(file.errors.is_empty(), "{:?}", file.errors);
    let bindings = file.imports[0].bindings.as_ref().unwrap();
    let names: Vec<(&str, Option<&str>)> = bindings
        .iter()
        .map(|b| (b.name.as_str(), b.alias.as_deref()))
        .collect();
    assert_eq!(names, [("03/D-01", None), ("/D-09", Some("top"))]);

    // `//` starts a comment; a space after the root is no path; a scheme ref id keeps its slash.
    let file = parse("thing x \"X\" {\n  v a//c\n}\n", "t.spec");
    assert!(file.errors.is_empty(), "{:?}", file.errors);
    assert_eq!(
        file.entities[0].fields.entries()[0].value,
        FieldValue::Identifier("a".into())
    );
    let file = parse("thing x \"X\" {\n  v / D-09\n}\n", "t.spec");
    assert!(!file.errors.is_empty(), "`/ D-09` is no path");
    let scheme = items("thing x \"X\" {\n  v [gh.pr:o/r]\n}\n");
    assert_eq!(scheme, [("gh.pr:o/r".to_string(), Written::SchemeRef)]);

    // The lexer reads each segment as a name, the slash between them as punctuation.
    let text = "thing x \"X\" {\n  v v1.0/03\n  w /D-09\n}\n";
    let lexemes = read(text);
    let at = lexemes.iter().position(|l| l.1 == "v1.0").expect("v1.0");
    assert_eq!(
        lexemes[at..at + 3],
        [
            (LexemeKind::Ident, "v1.0"),
            (LexemeKind::Punct('/'), "/"),
            (LexemeKind::Ident, "03"),
        ]
    );
    let at = lexemes.iter().position(|l| l.1 == "D-09").expect("D-09");
    assert_eq!(lexemes[at - 1], (LexemeKind::Punct('/'), "/"));
}
