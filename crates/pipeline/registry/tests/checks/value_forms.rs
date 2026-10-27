//! W159: a value written in the other form than its field's declared type gives it (ADR 0147),
//! read from `.spec` text, with the span it is written at (ADR 0115).

use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_test_macros::test as spec;

use crate::support::written::{checked, tickets};
use crate::support::{build, coded_in, declare};

/// `note` (a string field) on a kind that takes members, beside `tickets()`.
fn notes() -> ExtensionDeclaration {
    declare("@test/notes", |c| {
        c.kind("Note", |k| {
            k.keyword("note").open_fields(true);
            k.field("text", |f| {
                f.field_type(FieldType::String);
            });
        });
    })
}

/// The W159 messages with their `line:col` of `text`.
fn w159(text: &str) -> Vec<(String, usize, usize)> {
    let diags = checked(&build([tickets(), notes()]), &[("main.spec", text)]);
    coded_in(&diags, "W159")
        .into_iter()
        .map(|d| {
            let at = d.span.as_ref().expect("W159 has a span");
            (d.message.clone(), at.start_line, at.start_col)
        })
        .collect()
}

#[spec(
    behavior = "check_field_value_types",
    verify = "a quoted word on a word field and a bare word on a text field are W159"
)]
fn a_value_in_the_other_form_is_w159_at_the_value() {
    let found = w159(
        "ticket t1 \"T\" {\n  priority \"low\"\n  label \"ab\"\n  summary text\n  \
         watchers [\"t1\", t1]\n  labels [one, \"two\"]\n}\n",
    );
    let messages: Vec<&str> = found.iter().map(|(m, ..)| m.as_str()).collect();
    assert_eq!(
        messages,
        [
            "field 'priority' of ticket 't1' is a word: write it bare, as priority low",
            "field 'label' of ticket 't1' is a word: write it bare, as label ab",
            "field 'summary' of ticket 't1' is text: quote it, as summary \"text\"",
            "an item of field 'watchers' of ticket 't1' is a word: write it bare, as watchers [t1, t1]",
            "an item of field 'labels' of ticket 't1' is text: quote it, as labels [\"one\", \"two\"]",
        ]
    );
    // At the value, quotes included: `  priority "low"` starts at column 12 (1-based).
    assert_eq!((found[0].1, found[0].2), (2, 12), "{found:?}");
}

#[spec(
    behavior = "check_field_value_types",
    verify = "a bare word on a kind that takes members is not W159"
)]
fn a_bare_word_on_a_members_kind_is_not_w159() {
    assert!(w159("note n1 \"N\" {\n  text string\n}\n").is_empty());
}

#[spec(
    behavior = "check_field_value_types",
    verify = "a bare word is read by its field's declared type: a reference its name, an integer its difference, a date its day"
)]
fn a_scalar_reference_holds_a_dashed_or_numeric_name_bare() {
    assert!(
        w159("ticket t1 \"T\" {\n  blocker D-01\n  blocker 10\n  blocker 03/D-01\n}\n").is_empty()
    );
}

#[spec(
    behavior = "check_field_value_types",
    verify = "a value E061 refuses is not W159"
)]
fn a_value_e061_refuses_is_not_w159() {
    assert!(w159("ticket t1 \"T\" {\n  label \"a/b\"\n  priority \"urgent\"\n}\n").is_empty());
}

#[spec(
    behavior = "check_field_value_types",
    verify = "a value a defaults block writes in the other form is W159 once, at the default"
)]
fn a_defaults_value_in_the_other_form_is_w159_once_at_the_default() {
    let found = w159(
        "defaults ticket {\n  priority \"low\"\n  labels   [one]\n}\n\n\
         ticket t1 \"T\" {\n}\n\nticket t2 \"T\" {\n}\n\nticket t3 \"T\" {\n  priority \"high\"\n}\n",
    );
    assert_eq!(
        found,
        [
            (
                "field 'priority' of defaults ticket is a word: write it bare, as priority low"
                    .to_string(),
                2,
                12
            ),
            (
                "an item of field 'labels' of defaults ticket is text: quote it, as labels [\"one\"]"
                    .to_string(),
                3,
                13
            ),
            (
                "field 'priority' of ticket 't3' is a word: write it bare, as priority high"
                    .to_string(),
                13,
                12
            ),
        ]
    );
}

#[spec(
    behavior = "check_field_value_types",
    verify = "a value a defaults block writes in the other form is W159 once, at the default"
)]
fn a_defaults_block_below_a_block_is_still_reported_once_at_the_default() {
    let found = w159(
        "ticket t1 \"T\" {\n}\n\nticket t2 \"T\" {\n}\n\ndefaults ticket {\n  priority \"low\"\n}\n",
    );
    assert_eq!(
        found,
        [(
            "field 'priority' of defaults ticket is a word: write it bare, as priority low"
                .to_string(),
            8,
            12
        )]
    );
}

#[spec(
    behavior = "check_field_value_types",
    verify = "a quoted number or boolean, a literal on a text field and a whole value on a list field are W159"
)]
fn a_number_a_boolean_a_literal_or_a_whole_value_on_a_list_field_is_w159() {
    let found = w159(
        "ticket t1 \"T\" {\n  points   \"3\"\n  urgent   \"true\"\n  labels   \"one\"\n  \
         watchers t1\n  summary  7\n}\n",
    );
    let messages: Vec<&str> = found.iter().map(|(m, ..)| m.as_str()).collect();
    assert_eq!(
        messages,
        [
            "field 'points' of ticket 't1' is a number: write it bare, as points 3",
            "field 'urgent' of ticket 't1' is true or false: write it bare, as urgent true",
            "field 'labels' of ticket 't1' is a list: write it in brackets, as labels [\"one\"]",
            "field 'watchers' of ticket 't1' is a list: write it in brackets, as watchers [t1]",
            "field 'summary' of ticket 't1' is text: quote it, as summary \"7\"",
        ]
    );
    assert_eq!((found[0].1, found[0].2), (2, 12), "{found:?}");
}

#[spec(
    behavior = "check_field_value_types",
    verify = "a list item's W159 writes the whole list in its field's form"
)]
fn a_list_items_w159_writes_the_whole_list() {
    let found = w159("ticket t1 \"T\" {\n  watchers [\"t1\", t1]\n}\n");
    let messages: Vec<&str> = found.iter().map(|(m, ..)| m.as_str()).collect();
    assert_eq!(
        messages,
        [
            "an item of field 'watchers' of ticket 't1' is a word: write it bare, as watchers [t1, t1]"
        ]
    );
}

#[spec(
    behavior = "check_field_value_types",
    verify = "a multi-line text on a list field is W159, naming each line as an item"
)]
fn a_multi_line_text_on_a_list_field_is_w159_naming_its_lines() {
    let found = w159("ticket t1 \"T\" {\n  labels \"\"\"\n    one\n    two\n  \"\"\"\n}\n");
    let messages: Vec<&str> = found.iter().map(|(m, ..)| m.as_str()).collect();
    assert_eq!(
        messages,
        [
            "field 'labels' of ticket 't1' is a list written as one text: write each line as its own item, as labels [\"one\", \"two\"]"
        ]
    );
}

#[spec(
    behavior = "check_field_value_types",
    verify = "a quoted calendar day on a date field is W159, written bare"
)]
fn a_quoted_calendar_day_on_a_date_field_is_w159() {
    let found = w159("ticket t1 \"T\" {\n  due \"2026-03-01\"\n}\n");
    let messages: Vec<&str> = found.iter().map(|(m, ..)| m.as_str()).collect();
    assert_eq!(
        messages,
        ["field 'due' of ticket 't1' is a date: write it bare, as due 2026-03-01"]
    );
    assert!(w159("ticket t1 \"T\" {\n  due \"March\"\n}\n").is_empty());
}

#[spec(
    behavior = "check_field_value_types",
    verify = "a quoted number's W159 writes the number it reads to"
)]
fn a_quoted_numbers_w159_writes_the_number_it_reads_to() {
    for given in [" 3", "+3"] {
        let found = w159(&format!("ticket t1 \"T\" {{\n  points \"{given}\"\n}}\n"));
        let messages: Vec<&str> = found.iter().map(|(m, ..)| m.as_str()).collect();
        assert_eq!(
            messages,
            ["field 'points' of ticket 't1' is a number: write it bare, as points 3"]
        );
    }
}
