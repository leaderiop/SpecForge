//! What a name, a value word and an expression read as today, before a value is one word (ADR 0285, plan K1).

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

    let text = "x expr { a-b }";
    let words: Vec<(LexemeKind, &str)> = lex(text).iter().map(|l| (l.kind, l.text(text))).collect();
    assert_eq!(
        words[3..],
        [
            (LexemeKind::Ident, "a"),
            (LexemeKind::Punct('-'), "-"),
            (LexemeKind::Ident, "b"),
            (LexemeKind::Punct('}'), "}"),
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
