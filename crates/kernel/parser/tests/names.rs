//! What a name, a value word and an expression read as today, before a value is one word (ADR 0283, plan K1).

use specforge_parser::lex::{LexemeKind, lex};
use specforge_parser::testing::corpus;
use specforge_parser::{CmpOp, Expr, FieldValue, Written, parse};
use specforge_test_macros::test as specforge_test;

fn only_entity_fields(source: &str) -> Vec<(String, FieldValue)> {
    let file = parse(source, "t.spec");
    assert!(file.errors.is_empty(), "{source}: {:?}", file.errors);
    file.entities[0]
        .fields
        .entries()
        .iter()
        .map(|e| (e.key.as_str().to_string(), e.value.clone()))
        .collect()
}

#[specforge_test(
    behavior = "parse_all_block_types",
    verify = "a minus in an expression, a union variant and a scheme ref read as before a value is one word"
)]
fn a_minus_a_union_variant_and_a_scheme_ref_read_as_before() {
    let file = parse("type t = 01 | -1 | 2\n", "t.spec");
    assert!(file.errors.is_empty(), "{:?}", file.errors);
    let FieldValue::VariantList(variants) = file.entities[0].fields.get("variants").unwrap() else {
        panic!("a union body keeps its variants");
    };
    let texts: Vec<&str> = variants.iter().map(|v| v.text.as_str()).collect();
    assert_eq!(texts, ["01", "-1", "2"]);

    let fields = only_entity_fields("thing x \"X\" {\n  refs [gh.issue:42]\n}\n");
    let FieldValue::List(items) = &fields[0].1 else {
        panic!("a list");
    };
    assert_eq!(items.len(), 1);
    assert_eq!(
        (items[0].written, items[0].text.as_str()),
        (Written::SchemeRef, "gh.issue:42")
    );

    let fields = only_entity_fields("thing x \"X\" {\n  m expr { a-b < 3 }\n}\n");
    let FieldValue::Expression(exprs) = &fields[0].1 else {
        panic!("an expression group");
    };
    // The group reads `a-b < 3` as two conjuncts, `a` and `-b < 3`, today; a value word never reaches it.
    assert_eq!(exprs.len(), 2, "{exprs:?}");
    assert_eq!(exprs[0].expr, Expr::Var("a".into()));
    let Expr::Cmp(CmpOp::Lt, left, right) = &exprs[1].expr else {
        panic!("{:?}", exprs[1].expr);
    };
    assert!(matches!(&left.expr, Expr::Neg(b) if b.expr == Expr::Var("b".into())));
    assert_eq!(right.expr, Expr::Num(3.0, String::new()));

    let fields = only_entity_fields("thing x \"X\" {\n  m expr { a < 1.5s }\n}\n");
    let FieldValue::Expression(exprs) = &fields[0].1 else {
        panic!("an expression group");
    };
    let Expr::Cmp(_, _, right) = &exprs[0].expr else {
        panic!("{:?}", exprs[0].expr);
    };
    assert_eq!(right.expr, Expr::Num(1.5, "s".into()));

    let text = "thing t \"T\" {\n  m expr { a-b }\n}\n";
    let words: Vec<(LexemeKind, &str)> = lex(text).iter().map(|l| (l.kind, l.text(text))).collect();
    assert_eq!(
        words[7..10],
        [
            (LexemeKind::Ident, "a"),
            (LexemeKind::Punct('-'), "-"),
            (LexemeKind::Ident, "b"),
        ]
    );

    let fields =
        only_entity_fields("thing x \"X\" {\n  a 42\n  b -5\n  c true\n  d 2026-10-26\n}\n");
    assert_eq!(fields[0].1, FieldValue::Integer(42));
    assert_eq!(fields[1].1, FieldValue::Integer(-5));
    assert_eq!(fields[2].1, FieldValue::Boolean(true));
    assert_eq!(fields[3].1, FieldValue::Date("2026-10-26".into()));
}

/// The text a scalar value is written as, when the parse stores one of the four bare variants.
fn written(value: &FieldValue) -> Option<String> {
    Some(match value {
        FieldValue::Integer(n) => n.to_string(),
        FieldValue::Boolean(b) => b.to_string(),
        FieldValue::Date(text) | FieldValue::Identifier(text) => text.clone(),
        _ => return None,
    })
}

#[specforge_test(
    behavior = "parse_all_block_types",
    verify = "every bare scalar value of the repository's spec keeps its written text"
)]
fn every_bare_value_in_the_corpus_keeps_its_written_text() {
    let mut checked = 0usize;
    for (path, source) in corpus() {
        let lines: Vec<&str> = source.lines().collect();
        let file = parse(source, &path.to_string_lossy());
        for entity in &file.entities {
            for entry in entity.fields.entries() {
                let (Some(text), Some(span)) = (written(&entry.value), &entry.value_span) else {
                    continue;
                };
                if span.start_line != span.end_line {
                    continue;
                }
                let line = lines[span.start_line - 1];
                // Spans are 1-based lines and columns.
                assert_eq!(
                    line.get(span.start_col - 1..span.end_col - 1),
                    Some(text.as_str()),
                    "{}:{}: stored {text:?}, written {line:?}",
                    path.display(),
                    span.start_line
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 100, "only {checked} bare values were checked");
}

// -- K1 pins: written for the grammar K1 T3 changes; ignored until then ----------------------------

const GSD_NAMES: [&str; 9] = [
    "AUTH-01", "03-02", "03.1", "999.1", "v1.0", "D-01", "G-03-1", "03", "10",
];

#[specforge_test(
    behavior = "parse_all_block_types",
    verify = "an entity header names an entity with a word holding a dash or a dot, or all digits"
)]
fn a_gsd_name_names_an_entity() {
    for name in GSD_NAMES {
        let file = parse(&format!("thing {name} \"T\" {{\n}}\n"), "t.spec");
        assert!(file.errors.is_empty(), "{name}: {:?}", file.errors);
        assert_eq!(file.entities.len(), 1, "{name}");
        assert_eq!(file.entities[0].id.raw.as_str(), name);
    }
}

#[specforge_test(
    behavior = "parse_all_block_types",
    verify = "a field value or list item is one word, tagged by shape and kept as written"
)]
fn a_bare_value_or_list_item_is_one_word() {
    for (word, tag) in [
        ("03-02", "id"),
        ("2-1", "id"),
        ("007", "id"),
        ("1.5", "id"),
        ("99999999999999999999", "id"),
        ("AUTH-01", "id"),
        ("-5", "int"),
        ("42", "int"),
        ("true", "bool"),
        ("2026-10-26", "date"),
    ] {
        let file = parse(&format!("thing x \"X\" {{\n  v {word}\n}}\n"), "t.spec");
        assert!(file.errors.is_empty(), "{word}: {:?}", file.errors);
        let value = &file.entities[0].fields.entries()[0].value;
        match tag {
            "id" => assert_eq!(value, &FieldValue::Identifier(word.into()), "{word}"),
            "int" => assert_eq!(value, &FieldValue::Integer(word.parse().unwrap()), "{word}"),
            "bool" => assert_eq!(value, &FieldValue::Boolean(true)),
            _ => assert_eq!(value, &FieldValue::Date(word.into()), "{word}"),
        }
    }
    let file = parse(
        "thing x \"X\" {\n  v [AUTH-01, v1.0, 2026-10-26, 03, 10, -5]\n}\n",
        "t.spec",
    );
    assert!(file.errors.is_empty(), "{:?}", file.errors);
    let FieldValue::List(items) = &file.entities[0].fields.entries()[0].value else {
        panic!("a list");
    };
    let read: Vec<(&str, Written)> = items.iter().map(|i| (i.text.as_str(), i.written)).collect();
    assert_eq!(
        read,
        [
            ("AUTH-01", Written::Bare),
            ("v1.0", Written::Bare),
            ("2026-10-26", Written::Literal),
            ("03", Written::Literal),
            ("10", Written::Literal),
            ("-5", Written::Literal),
        ]
    );
}

#[specforge_test(
    behavior = "parse_use_imports",
    verify = "a selective use binding may import a name holding a dash or a dot"
)]
fn a_use_binding_may_import_a_gsd_name() {
    let file = parse(
        "use { AUTH-01, v1.0 as release } from \"./gsd\"\n",
        "t.spec",
    );
    assert!(file.errors.is_empty(), "{:?}", file.errors);
    let bindings = file.imports[0].bindings.as_ref().unwrap();
    let read: Vec<(&str, Option<&str>)> = bindings
        .iter()
        .map(|b| (b.name.as_str(), b.alias.as_deref()))
        .collect();
    assert_eq!(read, [("AUTH-01", None), ("v1.0", Some("release"))]);
}

#[specforge_test(
    behavior = "lex_spec_text",
    verify = "a word is one lexeme in a value, list item, header name or binding slot and nowhere else"
)]
fn a_word_is_one_lexeme_in_a_value_or_name_slot_only() {
    let text = "thing AUTH-01 \"x\" {\n  a 03-02\n  b [v1.0, 2-1]\n  c 2026-10-26\n  d 007\n  m expr { a-b }\n}\nuse { D-01 } from \"x\"\n";
    let read: Vec<(LexemeKind, &str)> = lex(text).iter().map(|l| (l.kind, l.text(text))).collect();
    let has = |kind, word| assert!(read.contains(&(kind, word)), "{word}: {read:?}");
    has(LexemeKind::Ident, "AUTH-01");
    has(LexemeKind::Ident, "03-02");
    has(LexemeKind::Ident, "v1.0");
    has(LexemeKind::Ident, "2-1");
    has(LexemeKind::Number, "2026-10-26");
    has(LexemeKind::Number, "007");
    has(LexemeKind::Ident, "D-01");
    assert!(!read.iter().any(|(_, word)| *word == "a-b"), "{read:?}");
}

#[specforge_test(
    behavior = "parse_all_block_types",
    verify = "a union's name stays an identifier: a dash in it is a syntax error"
)]
fn a_union_name_is_one_word() {
    let file = parse("type a-b = x\n", "t.spec");
    assert!(!file.errors.is_empty());
    let file = parse("type ab = x\n", "t.spec");
    assert!(file.errors.is_empty(), "{:?}", file.errors);
}
