//! `FieldForm`: a declared field type's one reading. What the field takes
//! (`admit`), how a written value is stored (`coerce`) and what every output
//! writes (`write`) agree with the schema of that JSON (`json_schema`).

use std::borrow::Cow;

use specforge_common::shape::{self, Definitions};
use specforge_common::{SourceSpan, Sym};
use specforge_graph::field_form::{
    self, BlockMembers, FieldForm, FieldForms, FieldJson, Reading, ValueShape,
};
use specforge_parser::{
    BlockItem, FieldEntry, FieldMap, FieldValue, ListItem, TypeMember, Written, parse_expression,
};
use specforge_protocol_types::FieldType;
use specforge_test_macros::test as specforge_test;

fn span() -> SourceSpan {
    SourceSpan {
        file: Sym::new("t.spec"),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

/// A union member written bare, at the fixed span.
fn member(text: &str) -> TypeMember {
    TypeMember {
        text: text.to_string(),
        span: span(),
        written: Written::Bare,
    }
}

fn item(text: &str, written: Written) -> ListItem {
    ListItem {
        text: text.to_string(),
        span: span(),
        written,
    }
}

fn expression() -> FieldValue {
    FieldValue::Expression(vec![
        parse_expression("a < 1").unwrap(),
        parse_expression("b >= 2ms").unwrap(),
    ])
}

/// One value per `FieldValue` variant, plus the scalars a field commonly
/// gets written as.
fn samples() -> Vec<FieldValue> {
    let mut inner = FieldMap::new();
    inner.push(Sym::new("m"), expression());
    let mut block = FieldMap::new();
    block.push(Sym::new("k"), FieldValue::String("v".into()));
    block.push(Sym::new("n"), FieldValue::Block(inner));
    vec![
        FieldValue::String("low".into()),
        FieldValue::String("7".into()),
        FieldValue::String("true".into()),
        FieldValue::Identifier("high".into()),
        FieldValue::Identifier("true".into()),
        FieldValue::String("2026-10-13".into()),
        FieldValue::Date("2026-10-13".into()),
        FieldValue::Date("2026-02-30".into()),
        FieldValue::Integer(3),
        FieldValue::Boolean(false),
        FieldValue::List(vec![item("s", Written::Quoted), item("t", Written::Quoted)]),
        FieldValue::List(vec![item("x", Written::Bare), item("y", Written::Bare)]),
        FieldValue::List(vec![
            item("1", Written::Literal),
            item("a", Written::Quoted),
        ]),
        FieldValue::List(vec![
            item("true", Written::Literal),
            item("2", Written::Literal),
        ]),
        FieldValue::List(vec![
            item("gh.issue:7", Written::SchemeRef),
            item("a", Written::Bare),
        ]),
        FieldValue::VariantList(vec![member("a"), member("b")]),
        FieldValue::TypeUnion(vec![member("string"), member("string[]")]),
        FieldValue::Block(block),
        expression(),
    ]
}

fn forms() -> Vec<FieldForm> {
    let mut forms: Vec<FieldForm> = FieldType::ALL
        .iter()
        .map(|t| FieldForm::new(*t, Vec::new()))
        .collect();
    forms.push(FieldForm::new(
        FieldType::Enum,
        vec!["low".to_string(), "high".to_string()],
    ));
    forms
}

/// The field text the checks read for a scalar value (E061 compares an
/// enum's text with its declared values); lists have none that matters.
fn text(value: &FieldValue) -> String {
    match value {
        FieldValue::String(s) | FieldValue::Identifier(s) | FieldValue::Date(s) => s.clone(),
        FieldValue::Integer(n) => n.to_string(),
        FieldValue::Boolean(b) => b.to_string(),
        _ => String::new(),
    }
}

/// The document of `form`'s schema: the schema at the root, `FieldJson`
/// defined once under `$defs`.
fn document(form: &FieldForm) -> serde_json::Value {
    let mut defs = Definitions::default();
    let schema = form.json_schema(&mut defs);
    defs.finish(schema)
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "a value the check admits is written as the published schema admits it"
)]
fn a_value_the_check_admits_is_written_as_its_schema_admits() {
    let any_value = shape::build::<FieldJson>();
    let span = span();
    let mut admitted = 0;
    for form in forms() {
        let schema = document(&form);
        for value in samples() {
            let stored = form.coerce(&value, Some(&span)).unwrap_or(value.clone());
            let written: serde_json::Value = field_form::write(&stored).into();
            if form.admit(ValueShape::of(&stored), &text(&stored)).is_ok() {
                admitted += 1;
                assert_eq!(
                    shape::violations(&schema, &written),
                    Vec::<String>::new(),
                    "{:?} stored as {stored:?} on {form:?}",
                    value
                );
            }
            // Admitted or not, every value is written as a field value.
            assert_eq!(
                shape::violations(&any_value, &written),
                Vec::<String>::new(),
                "{value:?}"
            );
        }
    }
    assert!(
        admitted > 40,
        "{admitted} admitted pairs: the sample is thin"
    );
}

#[test]
fn coercion_is_idempotent() {
    let span = span();
    for form in forms() {
        for value in samples() {
            if let Some(once) = form.coerce(&value, Some(&span)) {
                assert!(
                    form.coerce(&once, Some(&span)).is_none(),
                    "{value:?} on {form:?}"
                );
            }
        }
    }
}

#[test]
fn the_written_form_never_holds_a_parse_tree() {
    assert_eq!(
        serde_json::Value::from(field_form::write(&expression())),
        serde_json::json!("a < 1\nb >= 2ms")
    );
    let mut block = FieldMap::new();
    block.push(Sym::new("m"), expression());
    assert_eq!(
        serde_json::Value::from(field_form::write(&FieldValue::Block(block))),
        serde_json::json!({"m": "a < 1\nb >= 2ms"})
    );
}

#[test]
fn a_scalar_on_a_list_field_is_stored_as_a_one_item_list() {
    let span = span();
    let list_form = FieldForm::new(FieldType::ReferenceList, Vec::new());
    let one = |value: FieldValue| match list_form.coerce(&value, Some(&span)) {
        Some(FieldValue::List(items)) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].span, span);
            (items[0].text.clone(), items[0].written)
        }
        other => panic!("expected a one-item list, got {other:?}"),
    };
    assert_eq!(one(FieldValue::Integer(5)), ("5".into(), Written::Literal));
    assert_eq!(
        one(FieldValue::Boolean(true)),
        ("true".into(), Written::Literal)
    );
    assert_eq!(
        one(FieldValue::Date("2026-10-13".into())),
        ("2026-10-13".into(), Written::Literal)
    );
    assert_eq!(
        one(FieldValue::String("a".into())),
        ("a".into(), Written::Quoted)
    );
    // A bare word keeps its span and links like `field [id]`.
    assert_eq!(
        one(FieldValue::Identifier("inv1".into())),
        ("inv1".into(), Written::Bare)
    );
    // A list is stored as written.
    assert_eq!(
        list_form.coerce(
            &FieldValue::List(vec![item("a", Written::Bare)]),
            Some(&span)
        ),
        None
    );
}

#[test]
fn stored_is_the_coerced_value_or_the_value_as_written() {
    let forms = FieldForms::new(
        [("refs", FieldType::StringList), ("rpn", FieldType::Integer)]
            .into_iter()
            .map(|(f, t)| {
                (
                    ("k".to_string(), f.to_string()),
                    FieldForm::new(t, Vec::new()),
                )
            })
            .collect(),
        Default::default(),
        Default::default(),
    );
    let entry = |key: &str, value| FieldEntry {
        key: Sym::new(key),
        value,
        annotations: Vec::new(),
        value_span: Some(span()),
    };
    let one = entry("refs", FieldValue::Identifier("a".into()));
    let stored = forms.stored("k", &one);
    assert_eq!(*stored, FieldValue::List(vec![item("a", Written::Bare)]));
    assert!(matches!(stored, Cow::Owned(_)));
    let number = entry("rpn", FieldValue::String("100".into()));
    assert!(matches!(
        forms.stored("k", &number),
        Cow::Owned(FieldValue::Integer(100))
    ));
    // Already in its form, undeclared, and a stored value passed back in: borrowed.
    let list = entry("refs", FieldValue::List(vec![item("a", Written::Bare)]));
    let undeclared = entry("x", FieldValue::Identifier("a".into()));
    let again = entry("refs", (*stored).clone());
    for e in [&list, &undeclared, &again] {
        assert!(matches!(forms.stored("k", e), Cow::Borrowed(_)), "{e:?}");
    }
}

/// A block of `members` (name, form) over a kind `k` field `f` of `form`.
fn typed(form: FieldForm) -> FieldForms {
    FieldForms::new(
        [(("k".to_string(), "f".to_string()), form)].into(),
        Default::default(),
        Default::default(),
    )
}

fn members() -> BlockMembers {
    BlockMembers::new(vec![
        (
            "phase".to_string(),
            FieldForm::new(FieldType::Reference, Vec::new()).requiring(true),
        ),
        (
            "score".to_string(),
            FieldForm::new(FieldType::Integer, Vec::new()).bounded(Some(0), Some(100)),
        ),
        (
            "kind".to_string(),
            FieldForm::new(FieldType::EnumList, vec!["cli".into(), "api".into()]),
        ),
    ])
}

fn map(entries: &[(&str, FieldValue)]) -> FieldMap {
    let mut map = FieldMap::new();
    for (key, value) in entries {
        map.push_entry(FieldEntry {
            key: Sym::new(key),
            value: value.clone(),
            annotations: Vec::new(),
            value_span: Some(span()),
        });
    }
    map
}

fn bare(text: &str) -> FieldValue {
    FieldValue::Identifier(text.into())
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "a typed block is stored with each member coerced by its form, and a single block on a block list is its one item"
)]
fn a_typed_block_is_stored_member_by_member() {
    let span = span();
    let block = FieldForm::new(FieldType::Block, Vec::new()).with_members(members());
    let given = map(&[
        ("phase", FieldValue::Integer(2)),
        ("score", FieldValue::Identifier("03-02".into())),
        ("kind", bare("cli")),
    ]);
    let Some(FieldValue::Block(stored)) =
        block.coerce(&FieldValue::Block(given.clone()), Some(&span))
    else {
        panic!("a coerced block");
    };
    assert_eq!(stored.get("phase"), Some(&bare("2")));
    assert_eq!(stored.get("score"), Some(&FieldValue::Integer(1)));
    assert!(matches!(stored.get("kind"), Some(FieldValue::List(items)) if items.len() == 1));
    assert!(
        block
            .coerce(&FieldValue::Block(stored), Some(&span))
            .is_none(),
        "idempotent"
    );

    let list = FieldForm::new(FieldType::BlockList, Vec::new()).with_members(members());
    let Some(FieldValue::Blocks(items)) = list.coerce(&FieldValue::Block(given), Some(&span))
    else {
        panic!("one item");
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].fields.get("score"), Some(&FieldValue::Integer(1)));
    assert_eq!(
        list.coerce(&FieldValue::List(Vec::new()), Some(&span)),
        Some(FieldValue::Blocks(Vec::new()))
    );
    assert!(
        list.coerce(&FieldValue::Blocks(items), Some(&span))
            .is_none()
    );
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "an integer outside its declared minimum or maximum is E061 naming the bounds"
)]
fn bounds_name_the_range_in_the_remedy() {
    let both = FieldForm::new(FieldType::Integer, Vec::new()).bounded(Some(1), Some(5));
    assert!(both.admit(ValueShape::Integer, "5").is_ok());
    let low = both.admit(ValueShape::Integer, "0").unwrap_err();
    assert_eq!(
        low.suggestion.as_deref(),
        Some("write a whole number from 1 to 5")
    );
    let min = FieldForm::new(FieldType::Integer, Vec::new()).bounded(Some(1), None);
    assert_eq!(
        min.admit(ValueShape::Integer, "0")
            .unwrap_err()
            .suggestion
            .as_deref(),
        Some("write a whole number of at least 1")
    );
    let max = FieldForm::new(FieldType::Integer, Vec::new()).bounded(None, Some(4));
    assert_eq!(
        max.admit(ValueShape::Integer, "9")
            .unwrap_err()
            .suggestion
            .as_deref(),
        Some("write a whole number of at most 4")
    );
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "an enum list takes the declared words, each once, and a scalar is a one-item list"
)]
fn an_enum_list_refuses_an_unknown_and_a_repeated_word() {
    let form = FieldForm::new(FieldType::EnumList, vec!["threshold".into(), "jump".into()]);
    let words = |w: &[&str]| w.iter().map(|w| w.to_string()).collect::<Vec<_>>();
    assert!(form.refuse_item(&words(&["jump", "threshold"])).is_none());
    let given = words(&["thresold"]);
    let (item, refusal) = form.refuse_item(&given).unwrap();
    assert_eq!(item, "thresold");
    assert_eq!(
        refusal.suggestion.as_deref(),
        Some("did you mean 'threshold'?")
    );
    let given = words(&["jump", "jump"]);
    let (item, refusal) = form.refuse_item(&given).unwrap();
    assert_eq!(item, "jump");
    assert_eq!(
        refusal.suggestion.as_deref(),
        Some("'jump' is written twice: each value once")
    );
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "a block list takes a list of blocks and a typed block writes an object closed to its members"
)]
fn typed_forms_write_what_their_schema_admits() {
    let span = span();
    let item = |score: i64| BlockItem {
        fields: map(&[("phase", bare("02")), ("score", FieldValue::Integer(score))]),
        span: span.clone(),
    };
    let list = FieldForm::new(FieldType::BlockList, Vec::new()).with_members(members());
    let block = FieldForm::new(FieldType::Block, Vec::new()).with_members(members());
    let kinds = FieldForm::new(FieldType::EnumList, vec!["cli".into(), "api".into()]);
    let bounded = FieldForm::new(FieldType::Integer, Vec::new()).bounded(Some(0), Some(100));
    let cases: [(&FieldForm, FieldValue, bool); 6] = [
        (&list, FieldValue::Blocks(vec![item(5), item(7)]), true),
        (&block, FieldValue::Block(item(5).fields), true),
        (
            &kinds,
            FieldValue::List(vec![ListItem {
                text: "cli".into(),
                span: span.clone(),
                written: Written::Bare,
            }]),
            true,
        ),
        (&bounded, FieldValue::Integer(100), true),
        (&bounded, FieldValue::Integer(101), false),
        (
            &list,
            FieldValue::Blocks(vec![BlockItem {
                fields: map(&[("score", FieldValue::Integer(1))]),
                span: span.clone(),
            }]),
            false,
        ),
    ];
    for (form, value, conforms) in cases {
        let schema = document(form);
        let written: serde_json::Value = field_form::write(&value).into();
        assert_eq!(
            shape::violations(&schema, &written).is_empty(),
            conforms,
            "{value:?}: {:?}",
            shape::violations(&schema, &written)
        );
    }
    assert!(list.admit(ValueShape::Blocks, "").is_ok());
    assert_eq!(
        list.admit(ValueShape::List, "")
            .unwrap_err()
            .suggestion
            .as_deref(),
        Some("write each item as a block: { name value }")
    );
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "the walker yields every typed member of a stored value by its member path, and reading_at reads a declared member path"
)]
fn the_walker_names_each_typed_member_by_its_path() {
    let list = FieldForm::new(FieldType::BlockList, Vec::new()).with_members(members());
    let forms = typed(list);
    let span = span();
    let entry = FieldEntry {
        key: Sym::new("f"),
        value: FieldValue::Blocks(vec![
            BlockItem {
                fields: map(&[("phase", bare("02")), ("other", bare("x"))]),
                span: span.clone(),
            },
            BlockItem {
                fields: map(&[("score", FieldValue::Integer(3))]),
                span: span.clone(),
            },
        ]),
        annotations: Vec::new(),
        value_span: Some(span),
    };
    let paths: Vec<String> = forms
        .walk("k", &entry)
        .map(|m| m.path.to_string())
        .collect();
    assert_eq!(
        paths,
        ["f", "f[0].phase", "f[1].score"],
        "an undeclared key is not yielded"
    );
    let declared: Vec<String> = forms.walk("k", &entry).map(|m| m.path.declared()).collect();
    assert_eq!(declared, ["f", "f.phase", "f.score"]);
    assert!(matches!(forms.reading_at("k", "f.score"),
        Reading::Declared(form) if form.bounds() == (Some(0), Some(100))));
    assert!(matches!(
        forms.reading_at("k", "f.nope"),
        Reading::Undeclared
    ));
    assert!(matches!(forms.reading_at("k", "f"), Reading::Declared(_)));
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "a member of a typed block is respelled by its own form, in a block and in each item of a block list"
)]
fn a_member_is_respelled_by_its_form() {
    let list = FieldForm::new(FieldType::BlockList, Vec::new()).with_members(members());
    let forms = typed(list);
    let quoted = FieldValue::String("2".into());
    let entry = FieldEntry {
        key: Sym::new("f"),
        value: FieldValue::Blocks(vec![BlockItem {
            fields: map(&[("phase", quoted), ("score", FieldValue::Integer(3))]),
            span: span(),
        }]),
        annotations: Vec::new(),
        value_span: Some(span()),
    };
    let rewrites = forms.rewrites("k", &entry, &span());
    assert_eq!(rewrites.len(), 1, "{rewrites:?}");
    assert_eq!(
        (rewrites[0].text.as_str(), rewrites[0].value.as_str()),
        ("2", "2")
    );
}

/// The forms a setting can take: K3's types a project config can hold (ADR 0288).
fn setting_forms() -> Vec<FieldForm> {
    let members = |required: bool| {
        BlockMembers::new(vec![
            (
                "model".to_string(),
                FieldForm::new(FieldType::String, Vec::new()).requiring(required),
            ),
            (
                "level".to_string(),
                FieldForm::new(FieldType::Integer, Vec::new()).bounded(Some(1), Some(3)),
            ),
        ])
    };
    let words = vec!["low".to_string(), "high".to_string()];
    vec![
        FieldForm::new(FieldType::String, Vec::new()),
        FieldForm::new(FieldType::Integer, Vec::new()),
        FieldForm::new(FieldType::Integer, Vec::new()).bounded(Some(1), Some(3)),
        FieldForm::new(FieldType::Bool, Vec::new()),
        FieldForm::new(FieldType::Enum, words.clone()),
        FieldForm::new(FieldType::StringList, Vec::new()),
        FieldForm::new(FieldType::EnumList, words),
        FieldForm::new(FieldType::Block, Vec::new()),
        FieldForm::new(FieldType::Block, Vec::new()).with_members(members(true)),
        FieldForm::new(FieldType::Block, Vec::new()).with_members(members(false)),
        FieldForm::new(FieldType::BlockList, Vec::new()).with_members(members(true)),
    ]
}

/// JSON values of every shape, scalars first, then the lists and objects built from them.
fn json_corpus() -> Vec<serde_json::Value> {
    use serde_json::json;
    let scalars = vec![
        json!(null),
        json!(true),
        json!(false),
        json!(0),
        json!(2),
        json!(-1),
        json!(4),
        json!(2.5),
        json!(""),
        json!("low"),
        json!("high"),
        json!("other"),
    ];
    let mut corpus = scalars.clone();
    corpus.push(json!([]));
    corpus.push(json!({}));
    for a in &scalars {
        corpus.push(json!([a]));
        corpus.push(json!({"model": a}));
        corpus.push(json!({"level": a}));
        corpus.push(json!({"model": "m", "level": a}));
        corpus.push(json!([{"model": a}]));
        for b in &scalars {
            corpus.push(json!([a, b]));
        }
    }
    corpus.push(json!({"model": "m", "extra": 1}));
    corpus.push(json!({"extra": "x"}));
    corpus.push(json!([{"model": "m"}, {"level": 2}]));
    corpus.push(json!(["low", "low"]));
    corpus.push(json!(["low", "high"]));
    corpus
}

#[specforge_test(
    behavior = "load_extension_settings",
    verify = "json admitted by a form conforms to the form's published schema, and json the schema refuses is refused"
)]
fn json_admitted_by_a_form_conforms_to_its_schema() {
    let mut agreed = 0;
    for form in setting_forms() {
        let schema = document(&form);
        for value in json_corpus() {
            let admitted = form.admit_json(&value).is_ok();
            let conforms = shape::violations(&schema, &value).is_empty();
            assert_eq!(
                admitted, conforms,
                "{value} on {form:?}: admit_json {admitted}, schema {conforms}"
            );
            agreed += usize::from(admitted);
        }
    }
    assert!(agreed > 60, "{agreed} admitted pairs: the corpus is thin");
}
