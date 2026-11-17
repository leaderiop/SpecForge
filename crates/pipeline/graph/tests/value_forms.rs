//! `identifier`: one word from an open set (ADR 0147).

use std::collections::{BTreeSet, HashMap};

use specforge_graph::field_form::{self, FieldForm, FieldForms, Place, Reading, ValueShape};
use specforge_graph::{FieldValue, Written};
use specforge_protocol_types::{Decimal, FieldType};
use specforge_test_macros::test as specforge_test;

#[specforge_test(requirement = "FEATURE-018")]
fn an_identifier_field_takes_one_word() {
    let form = FieldForm::new(FieldType::Identifier, Vec::new());

    assert!(form.admit(ValueShape::Identifier, "cart").is_ok());
    assert!(form.admit(ValueShape::String, "cart").is_ok());
    assert!(form.admit(ValueShape::String, "_a1").is_ok());

    let remedy = "write one word: letters, digits and _, starting with a letter or _";
    for (shape, text) in [
        (ValueShape::String, "api/lsp"),
        (ValueShape::String, "two words"),
        (ValueShape::String, "1abc"),
        (ValueShape::String, ""),
        (ValueShape::Date, "2026-01-01"),
        (ValueShape::Integer, "7"),
    ] {
        let refusal = form.admit(shape, text).unwrap_err();
        assert_eq!(refusal.suggestion.as_deref(), Some(remedy), "{text}");
    }
    let list = form.admit(ValueShape::List, "").unwrap_err();
    assert_eq!(
        list.suggestion.as_deref(),
        Some("give a single value, not a list")
    );
}

fn forms() -> FieldForms {
    let mut declared = HashMap::new();
    for (name, ty) in [
        ("id_", FieldType::Identifier),
        ("ref_", FieldType::Reference),
        ("refs", FieldType::ReferenceList),
        ("text", FieldType::String),
        ("texts", FieldType::StringList),
        ("num", FieldType::Integer),
        ("dec", FieldType::Number),
        ("due", FieldType::Date),
        ("flag", FieldType::Bool),
    ] {
        declared.insert(
            ("k".to_string(), name.to_string()),
            FieldForm::new(ty, vec![]),
        );
    }
    declared.insert(
        ("m".to_string(), "text".to_string()),
        FieldForm::new(FieldType::String, vec![]),
    );
    FieldForms::new(declared, BTreeSet::from(["m".to_string()]), BTreeSet::new())
}

#[specforge_test(requirement = "FEATURE-018")]
fn a_field_type_writes_its_values_in_one_form() {
    let forms = forms();
    let rewrite = |field, written, text| {
        forms
            .rewrite("k", field, written, text, Place::Item)
            .map(|r| r.to)
    };

    for field in ["id_", "ref_", "refs"] {
        assert_eq!(
            rewrite(field, Written::Quoted, "a"),
            Some(Written::Bare),
            "{field}"
        );
        assert_eq!(rewrite(field, Written::Bare, "a"), None, "{field}");
    }
    for field in ["text", "texts"] {
        assert_eq!(
            rewrite(field, Written::Bare, "a"),
            Some(Written::Quoted),
            "{field}"
        );
        assert_eq!(rewrite(field, Written::Quoted, "a"), None, "{field}");
    }
    assert_eq!(
        forms
            .rewrite("k", "num", Written::Quoted, "7", Place::Whole)
            .map(|r| r.to),
        Some(Written::Literal)
    );
}

#[specforge_test(requirement = "FEATURE-018")]
fn a_value_is_rewritten_at_its_place_to_its_types_one_form() {
    let forms = forms();
    let at = |field, written, text, place| {
        forms
            .rewrite("k", field, written, text, place)
            .map(|r| (r.to, r.listed))
    };
    let respell = |to, listed| Some((to, listed));
    use Place::{Item, Whole};
    assert_eq!(
        at("num", Written::Quoted, "100", Whole),
        respell(Written::Literal, false)
    );
    assert_eq!(at("num", Written::Literal, "100", Whole), None);
    // E061 reports a text that is no number.
    assert_eq!(at("num", Written::Quoted, "abc", Whole), None);
    assert_eq!(at("num", Written::Bare, "abc", Whole), None);
    assert_eq!(
        at("due", Written::Quoted, "2026-03-01", Whole),
        respell(Written::Literal, false)
    );
    assert_eq!(at("due", Written::Literal, "2026-03-01", Whole), None);
    assert_eq!(at("due", Written::Literal, "2026-02-30", Whole), None);
    assert_eq!(
        at("flag", Written::Quoted, "true", Whole),
        respell(Written::Literal, false)
    );
    assert_eq!(
        at("text", Written::Literal, "100", Whole),
        respell(Written::Quoted, false)
    );
    assert_eq!(
        at("text", Written::Literal, "2026-01-01", Whole),
        respell(Written::Quoted, false)
    );
    assert_eq!(
        at("refs", Written::Bare, "f1", Whole),
        respell(Written::Bare, true)
    );
    assert_eq!(
        at("refs", Written::Quoted, "f1", Whole),
        respell(Written::Bare, true)
    );
    assert_eq!(at("refs", Written::Bare, "f1", Item), None);
    assert_eq!(
        at("texts", Written::Quoted, "gh.issue:1", Whole),
        respell(Written::Quoted, true)
    );
    assert_eq!(at("texts", Written::Quoted, "gh.issue:1", Item), None);
}

#[specforge_test(requirement = "FEATURE-018")]
fn rewrite_leaves_what_it_cannot_write() {
    let forms = forms();
    // Text with a space, or true/false, cannot be a bare word (a path can: ADR 0284).
    assert_eq!(
        forms.rewrite("k", "ref_", Written::Quoted, "api / lsp", Place::Item),
        None
    );
    assert_eq!(
        forms.rewrite("k", "id_", Written::Quoted, "true", Place::Item),
        None
    );
    // A scheme ref id is a reference list item's bare form (ADR 0153); a scalar reference takes a word only,
    // and an identifier too, as a scheme ref id does not parse bare outside a list.
    let item = |field, written, text| {
        forms
            .rewrite("k", field, written, text, Place::Item)
            .map(|r| r.to)
    };
    assert_eq!(
        item("refs", Written::Quoted, "gh.issue:1"),
        Some(Written::Bare)
    );
    assert_eq!(item("ref_", Written::Quoted, "gh.issue:1"), None);
    assert_eq!(item("id_", Written::Quoted, "gh.issue:1"), None);
    // A scheme ref on a string list is in form; a literal on a text field is quoted.
    assert_eq!(item("texts", Written::SchemeRef, "gh.issue:1"), None);
    assert_eq!(
        item("text", Written::Literal, "2026-01-01"),
        Some(Written::Quoted)
    );
    // A bare word on a members kind may be a member's type.
    assert_eq!(
        forms.rewrite("m", "text", Written::Bare, "string", Place::Item),
        None
    );
    // An undeclared field or kind has no form.
    assert_eq!(
        forms.rewrite("k", "nope", Written::Bare, "a", Place::Item),
        None
    );
    assert_eq!(
        forms.rewrite("zzz", "text", Written::Bare, "a", Place::Item),
        None
    );
}

#[specforge_test(requirement = "FEATURE-018")]
fn a_date_field_takes_one_calendar_day() {
    let form = FieldForm::new(FieldType::Date, Vec::new());
    for day in ["2026-03-01", "2024-02-29"] {
        assert!(form.admit(ValueShape::Date, day).is_ok(), "{day}");
    }
    for day in ["2026-02-29", "2026-13-01", "2026-04-31", "0000-00-00"] {
        let refusal = form.admit(ValueShape::Date, day).unwrap_err();
        assert_eq!(
            refusal.suggestion.as_deref(),
            Some("there is no such day: write a calendar day, YYYY-MM-DD"),
            "{day}"
        );
    }
    let refusal = form.admit(ValueShape::String, "March 2026").unwrap_err();
    assert_eq!(
        refusal.suggestion.as_deref(),
        Some("write a calendar day, YYYY-MM-DD")
    );
    let list = form.admit(ValueShape::List, "").unwrap_err();
    assert_eq!(
        list.suggestion.as_deref(),
        Some("give a single value, not a list")
    );

    assert_eq!(
        form.coerce(&FieldValue::String("2026-03-01".into()), None),
        Some(FieldValue::Date("2026-03-01".into()))
    );
    assert_eq!(form.coerce(&FieldValue::String("March".into()), None), None);
    assert_eq!(
        form.coerce(&FieldValue::Date("2026-03-01".into()), None),
        None
    );
    let mut defs = Default::default();
    assert_eq!(
        form.json_schema(&mut defs),
        serde_json::json!({"type": "string", "format": "date"})
    );
}

#[specforge_test(requirement = "FEATURE-018")]
fn a_respelled_value_carries_its_spelling() {
    let forms = forms();
    let spelling = |field, written, text| {
        forms
            .rewrite("k", field, written, text, Place::Whole)
            .map(|r| (r.spelling, r.listed))
    };
    for text in [" 7", "+7", "007"] {
        assert_eq!(
            spelling("num", Written::Quoted, text),
            Some(("7".to_string(), false)),
            "{text:?}"
        );
    }
    assert_eq!(
        spelling("text", Written::Literal, "100"),
        Some(("\"100\"".to_string(), false))
    );
    assert_eq!(
        spelling("refs", Written::Quoted, "f1"),
        Some(("f1".to_string(), true))
    );
}

#[specforge_test(requirement = "FEATURE-018")]
fn a_list_on_a_number_a_boolean_or_a_date_field_keeps_its_items() {
    let forms = forms();
    for (field, text) in [("due", "2026-03-01"), ("num", "+3"), ("flag", "true")] {
        assert_eq!(
            forms.rewrite("k", field, Written::Quoted, text, Place::Item),
            None,
            "{field}"
        );
    }
}

// -- K1: a declared type reads a bare word ------------------------------------------------------

/// What the build stores for `value` on a field of `form`, or E061's remedy: the coerced value, then `admit`.
fn read(form: &FieldForm, value: FieldValue) -> Result<FieldValue, Option<String>> {
    let stored = form.coerce(&value, None).unwrap_or(value);
    let text = match &stored {
        FieldValue::Identifier(t) | FieldValue::String(t) | FieldValue::Date(t) => t.clone(),
        FieldValue::Integer(n) => n.to_string(),
        FieldValue::Number(d) => d.to_string(),
        FieldValue::Boolean(b) => b.to_string(),
        other => panic!("{other:?}"),
    };
    form.admit(ValueShape::of(&stored), &text)
        .map(|()| stored)
        .map_err(|refusal| refusal.suggestion)
}

fn word(text: &str) -> FieldValue {
    FieldValue::Identifier(text.into())
}

#[specforge_test(requirement = "FEATURE-018")]
fn a_declared_type_reads_a_bare_word() {
    let reference = FieldForm::new(FieldType::Reference, Vec::new());
    assert_eq!(read(&reference, word("03")), Ok(word("03")));
    assert_eq!(read(&reference, word("2-1")), Ok(word("2-1")));
    assert_eq!(read(&reference, FieldValue::Integer(10)), Ok(word("10")));
    assert_eq!(
        read(&reference, FieldValue::Date("2026-10-26".into())),
        Ok(word("2026-10-26"))
    );
    assert_eq!(
        read(&reference, FieldValue::Boolean(true)),
        Ok(word("true"))
    );
    assert_eq!(
        read(&reference, FieldValue::Integer(-5)),
        Err(Some("a name never starts with '-'".to_string()))
    );

    for text in ["-a", "-0", "-007", "-5-3"] {
        assert_eq!(
            read(&reference, word(text)),
            Err(Some("a name never starts with '-'".to_string())),
            "{text}"
        );
    }

    let integer = FieldForm::new(FieldType::Integer, Vec::new());
    for (text, value) in [("2-1", 1), ("03-02", 1), ("007", 7)] {
        assert_eq!(
            read(&integer, word(text)),
            Ok(FieldValue::Integer(value)),
            "{text}"
        );
    }
    assert_eq!(
        read(&integer, FieldValue::Integer(-5)),
        Ok(FieldValue::Integer(-5))
    );
    assert_eq!(
        read(&integer, FieldValue::Date("2026-10-26".into())),
        Ok(FieldValue::Integer(1990))
    );
    for (text, remedy) in [
        ("1.5", "an integer has no fraction"),
        ("AUTH-01", "'AUTH' is no number"),
        (
            "99999999999999999999",
            "'99999999999999999999' is outside -9223372036854775808..9223372036854775807",
        ),
    ] {
        let refused = read(&integer, word(text)).unwrap_err().unwrap_or_default();
        assert!(refused.contains(remedy), "{text}: {refused}");
    }
    // A quoted difference is read as a bare one, and respelled bare once (W159); a bare one is in form.
    assert_eq!(
        read(&integer, FieldValue::String("2-1".into())),
        Ok(FieldValue::Integer(1))
    );
    let forms = forms();
    assert_eq!(
        forms
            .rewrite("k", "num", Written::Quoted, "2-1", Place::Whole)
            .map(|r| (r.to, r.spelling)),
        Some((Written::Literal, "2-1".to_string()))
    );
    assert_eq!(
        forms.rewrite("k", "num", Written::Bare, "2-1", Place::Whole),
        None
    );

    let date = FieldForm::new(FieldType::Date, Vec::new());
    let refused = read(&date, word("03-02")).unwrap_err().unwrap_or_default();
    assert!(refused.contains("a calendar day"), "{refused}");

    let identifier = FieldForm::new(FieldType::Identifier, Vec::new());
    assert!(read(&identifier, word("a-b")).is_err());
}

#[specforge_test(requirement = "FEATURE-018")]
fn a_number_field_reads_a_decimal_word() {
    let decimal = |text: &str| FieldValue::Number(Decimal::parse(text).unwrap());
    let number = FieldForm::new(FieldType::Number, Vec::new());
    for (written, value) in [
        ("0.75", "0.75"),
        ("-0.25", "-0.25"),
        ("007.5", "7.5"),
        ("1.50", "1.5"),
        ("1.5e3", "1500"),
        ("2E-4", "0.0002"),
        ("100", "100"),
    ] {
        assert_eq!(
            read(&number, word(written)),
            Ok(decimal(value)),
            "{written}"
        );
    }
    assert_eq!(read(&number, FieldValue::Integer(-5)), Ok(decimal("-5")));
    assert_eq!(
        read(&number, FieldValue::String(" 1.50 ".into())),
        Ok(decimal("1.5"))
    );
    for (written, remedy) in [
        ("abc", "'abc' is no decimal number"),
        ("1.2.3", "'1.2.3' is no decimal number"),
        ("2-1", "'2-1' is no decimal number"),
        (
            "0.1234567890123456",
            "a number keeps at most 15 significant digits",
        ),
        ("1e308", "'1e308' is outside the range a number holds"),
    ] {
        assert_eq!(
            read(&number, word(written)),
            Err(Some(remedy.to_string())),
            "{written}"
        );
    }
    assert_eq!(
        read(&number, FieldValue::Date("2026-10-26".into())),
        Err(Some("'2026-10-26' is no decimal number".to_string()))
    );
    assert_eq!(
        read(&number, FieldValue::Integer(1234567890123456)),
        Err(Some(
            "a number keeps at most 15 significant digits".to_string()
        ))
    );
    assert_eq!(
        number
            .admit(ValueShape::List, "a, b")
            .unwrap_err()
            .suggestion
            .as_deref(),
        Some("give a single value, not a list")
    );
    // An integer field refuses a decimal: a fraction is a number's.
    let integer = FieldForm::new(FieldType::Integer, Vec::new());
    assert_eq!(
        integer
            .admit(ValueShape::Number, "1.5")
            .unwrap_err()
            .suggestion
            .as_deref(),
        Some("an integer has no fraction")
    );
}

#[specforge_test(requirement = "FEATURE-018")]
fn a_decimal_word_on_a_text_or_reference_field_keeps_its_text() {
    let tagged = |text: &str| FieldValue::Number(Decimal::parse(text).unwrap());
    for field_type in [FieldType::String, FieldType::Identifier] {
        let form = FieldForm::new(field_type, Vec::new());
        assert_eq!(
            form.coerce(&tagged("1.5"), None),
            Some(FieldValue::String("1.5".into())),
            "{field_type:?}"
        );
    }
    let text = FieldForm::new(FieldType::String, Vec::new());
    assert_eq!(
        read(&text, tagged("-0.25")),
        Ok(FieldValue::String("-0.25".into()))
    );
    let enumerated = FieldForm::new(FieldType::Enum, vec!["1.5".into(), "2".into()]);
    assert_eq!(
        read(&enumerated, tagged("1.5")),
        Ok(FieldValue::String("1.5".into()))
    );
    let reference = FieldForm::new(FieldType::Reference, Vec::new());
    assert_eq!(read(&reference, tagged("3.1")), Ok(word("3.1")));
    assert_eq!(
        read(&reference, tagged("-0.5")),
        Err(Some("a name never starts with '-'".to_string()))
    );
    let number = FieldForm::new(FieldType::Number, Vec::new());
    assert_eq!(read(&number, tagged("1.5")), Ok(tagged("1.5")));
    let integer = FieldForm::new(FieldType::Integer, Vec::new());
    assert_eq!(
        read(&integer, tagged("1.5")),
        Err(Some("an integer has no fraction".to_string()))
    );
    // On a list field a tagged decimal is one literal item, as an integer is.
    let list = FieldForm::new(FieldType::ReferenceList, Vec::new());
    let span = specforge_common::SourceSpan {
        file: specforge_common::Sym::new("a.spec"),
        start_line: 1,
        start_col: 1,
        end_line: 1,
        end_col: 4,
    };
    match list.coerce(&tagged("3.1"), Some(&span)) {
        Some(FieldValue::List(items)) => {
            assert_eq!(items.len(), 1);
            assert_eq!(
                (items[0].text.as_str(), items[0].written),
                ("3.1", Written::Literal)
            );
        }
        other => panic!("{other:?}"),
    }
}

#[specforge_test(requirement = "FEATURE-018")]
fn a_quoted_number_is_w159_and_respelled_bare() {
    let forms = forms();
    let respell = |written, text: &str, place| {
        forms
            .rewrite("k", "dec", written, text, place)
            .map(|r| (r.to, r.spelling))
    };
    let literal = |text: &str| Some((Written::Literal, text.to_string()));
    assert_eq!(
        respell(Written::Quoted, "1.50", Place::Whole),
        literal("1.50")
    );
    assert_eq!(
        respell(Written::Quoted, "1.5e3", Place::Whole),
        literal("1.5e3")
    );
    assert_eq!(
        respell(Written::Quoted, "+1.5", Place::Whole),
        literal("1.5")
    );
    assert_eq!(
        respell(Written::Quoted, "1e+3", Place::Whole),
        literal("1000")
    );
    assert_eq!(
        respell(Written::Quoted, " 0.25 ", Place::Whole),
        literal("0.25")
    );
    // A bare decimal is in its form; a text that is no decimal is E061's; a list's items keep their text.
    assert_eq!(respell(Written::Bare, "1.50", Place::Whole), None);
    assert_eq!(respell(Written::Literal, "1.5", Place::Whole), None);
    assert_eq!(respell(Written::Quoted, "abc", Place::Whole), None);
    assert_eq!(respell(Written::Quoted, "1.5", Place::Item), None);
    let number = forms.reading("k", "dec");
    let Reading::Declared(form) = number else {
        panic!("a declared number");
    };
    assert_eq!(field_form::spell::value(form, "+1.5"), "1.5");
    assert_eq!(field_form::spell::value(form, "007.5"), "007.5");
}
