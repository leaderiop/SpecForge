//! A value is written in its field's form (ADR 0147), by the declared types of `FieldForms`.

use super::*;
use specforge_graph::FieldForm;
use specforge_protocol_types::FieldType;
use specforge_test_macros::test as specforge_test;
use std::collections::{BTreeSet, HashMap};

fn forms() -> FieldForms {
    let mut declared = HashMap::new();
    for (kind, field, ty) in [
        ("behavior", "category", FieldType::Identifier),
        ("behavior", "contract", FieldType::String),
        ("behavior", "points", FieldType::Integer),
        ("behavior", "urgent", FieldType::Bool),
        ("behavior", "due", FieldType::Date),
        ("behavior", "types", FieldType::ReferenceList),
        ("behavior", "tags", FieldType::StringList),
        ("type", "description", FieldType::String),
    ] {
        declared.insert(
            (kind.to_string(), field.to_string()),
            FieldForm::new(ty, vec![]),
        );
    }
    FieldForms::new(
        declared,
        BTreeSet::from(["type".to_string()]),
        BTreeSet::new(),
    )
}

fn fmt_forms(source: &str) -> String {
    format_source(source, &FormatConfig::default(), &forms()).formatted
}

#[specforge_test(
    behavior = "apply_format_rules",
    verify = "a value is written in its field's form, read from the project's declarations"
)]
fn a_value_is_written_in_its_fields_form() {
    assert_eq!(
        fmt_forms("behavior b \"B\" {\n  category \"cart\"\n  contract plain\n}\n"),
        "behavior b \"B\" {\n  category cart\n  contract \"plain\"\n}\n"
    );
    assert_eq!(
        fmt_forms("behavior b \"B\" {\n  types [\"a\", b]\n  tags [x, \"y\"]\n}\n"),
        "behavior b \"B\" {\n  types [a, b]\n  tags  [\"x\", \"y\"]\n}\n"
    );
}

#[specforge_test(
    behavior = "apply_format_rules",
    verify = "a value is written in its field's form, read from the project's declarations"
)]
fn what_cannot_be_written_in_the_form_is_left() {
    let text = "behavior b \"B\" {\n  category \"api/lsp\"\n  contract \"\"\"\n    x\n  \"\"\"\n}\n\n\
                type T \"T\" {\n  description string\n}\n";
    assert_eq!(fmt_forms(text), text);
    let nested = "behavior b \"B\" {\n  inner {\n    category \"x\"\n  }\n}\n";
    assert_eq!(fmt_forms(nested), nested);
    let truth = "behavior b \"B\" {\n  category \"true\"\n}\n";
    assert_eq!(fmt_forms(truth), truth);
}

#[specforge_test(
    behavior = "apply_format_rules",
    verify = "a value is written in its field's form, read from the project's declarations"
)]
fn a_quoted_scheme_ref_in_a_reference_list_loses_its_quotes() {
    assert_eq!(
        fmt_forms("behavior b \"B\" {\n  types [\"gh.issue:1\", b]\n}\n"),
        "behavior b \"B\" {\n  types [gh.issue:1, b]\n}\n"
    );
}

#[specforge_test(
    behavior = "apply_format_rules",
    verify = "without the project's declarations no value is rewritten"
)]
fn with_no_forms_nothing_is_rewritten() {
    let text = "behavior b \"B\" {\n  category \"cart\"\n  contract plain\n}\n";
    assert_eq!(fmt(text), text);
}

#[specforge_test(
    behavior = "apply_format_rules",
    verify = "a value is written in its field's form, read from the project's declarations"
)]
fn the_rewritten_text_is_a_fixed_point() {
    let once = fmt_forms("behavior b \"B\" {\n  category \"cart\"\n  types [\"a\"]\n}\n");
    assert_eq!(fmt_forms(&once), once);
}

#[specforge_test(
    behavior = "apply_format_rules",
    verify = "a value is written in its field's form, read from the project's declarations"
)]
fn a_triple_quoted_word_is_written_bare_as_a_quoted_one_is() {
    assert_eq!(
        fmt_forms("behavior b \"B\" {\n  category \"\"\"cart\"\"\"\n}\n"),
        "behavior b \"B\" {\n  category cart\n}\n"
    );
}

#[specforge_test(
    behavior = "apply_format_rules",
    verify = "a defaults block's values are written in the forms of the kind it names"
)]
fn a_defaults_blocks_values_are_written_in_the_forms_of_the_kind_it_names() {
    assert_eq!(
        fmt_forms(
            "defaults behavior {\n  category \"cart\"\n  types    [\"a\", b]\n  tags     [x]\n}\n"
        ),
        "defaults behavior {\n  category cart\n  types    [a, b]\n  tags     [\"x\"]\n}\n"
    );
    let once = fmt_forms("defaults behavior {\n  category \"cart\"\n}\n");
    assert_eq!(fmt_forms(&once), once);
}

/// D5 through the named kind: `type` takes members, so a bare word stays.
#[specforge_test(
    behavior = "apply_format_rules",
    verify = "a defaults block's values are written in the forms of the kind it names"
)]
fn a_defaults_block_of_a_kind_that_takes_members_keeps_a_bare_word() {
    let text = "defaults type {\n  description string\n}\n";
    assert_eq!(fmt_forms(text), text);
}

/// A kind with no forms (E077 in the graph) rewrites nothing.
#[specforge_test(
    behavior = "apply_format_rules",
    verify = "a defaults block's values are written in the forms of the kind it names"
)]
fn a_defaults_block_naming_an_undeclared_kind_rewrites_nothing() {
    let text = "defaults widget {\n  category \"cart\"\n}\n";
    assert_eq!(fmt_forms(text), text);
}

#[specforge_test(
    behavior = "apply_format_rules",
    verify = "a number, a boolean, a literal and a whole value on a list field are written in their field's form"
)]
fn a_number_a_boolean_a_literal_and_a_whole_value_on_a_list_are_written_in_their_form() {
    let format = fmt_forms;
    let once = format(
        "behavior b \"B\" {\n  points \"3\"\n  urgent \"true\"\n  tags \"x\"\n  types a\n  contract 7\n  due \"2026-03-01\"\n}\n",
    );
    assert_eq!(
        once,
        "behavior b \"B\" {\n  points   3\n  urgent   true\n  tags     [\"x\"]\n  types    [a]\n  contract \"7\"\n  due      2026-03-01\n}\n"
    );
    assert_eq!(format(&once), once, "a fixed point");
}

#[specforge_test(
    behavior = "apply_format_rules",
    verify = "a multi-line text on a list field is left as written"
)]
fn a_multi_line_text_on_a_list_field_is_left_as_written() {
    let text = "behavior b \"B\" {\n  tags \"\"\"\n    one\n    two\n  \"\"\"\n}\n";
    assert_eq!(fmt_forms(text), text);
}
