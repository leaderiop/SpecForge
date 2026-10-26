//! A `//` line inside a string is part of the string: no typed edit takes it for the comment of the member below
//! (ADR 0248).

use super::*;

/// A list below a triple-quoted string whose last line starts with `//`.
const TRIPLE: &str = "event ev_one \"E1\" {\n}\n\nbehavior bh_one \"B\" {\n  category command\n  contract \"\"\"\n    Example:\n    // the caller writes this\"\"\"\n  produces [ev_one]\n}\n";
/// The same below a `"…"` string written over two lines.
const QUOTED: &str = "event ev_one \"E1\" {\n}\n\nbehavior bh_one \"B\" {\n  category command\n  contract \"Example:\n// the caller writes this\"\n  produces [ev_one]\n}\n";
/// A verify statement below the triple-quoted string.
const STATEMENT: &str = "behavior bh_one \"B\" {\n  category command\n  contract \"\"\"\n    Example:\n    // the caller writes this\"\"\"\n  verify \"returns the token\"\n}\n";
/// A verify statement whose string's last line starts with `//`, above the first field.
const FIRST: &str = "behavior bh_two \"B2\" {\n  verify \"Example:\n// the caller writes this\"\n  contract \"\"\"\n    text\"\"\"\n}\n";

const PRODUCES: &str = "  produces [ev_one]\n";

#[specforge_test_macros::test(
    behavior = "edit_entity_field",
    verify = "unset keeps a // line inside the string above the field"
)]
fn unset_keeps_a_comment_line_inside_a_string_above_the_field() {
    for source in [TRIPLE, QUOTED] {
        let c = project(&[("hello.spec", source)]);
        let text = after(&c, "hello.spec", &[unset("bh_one", "produces")]);
        assert_eq!(text, source.replace(PRODUCES, ""));
        assert!(is_canonical(&c, &text));
    }
}

#[specforge_test_macros::test(
    behavior = "edit_entity_link",
    verify = "unlink of the last item keeps a // line inside the string above the list"
)]
fn unlink_keeps_a_comment_line_inside_a_string_above_the_list() {
    let c = project(&[("hello.spec", TRIPLE)]);
    let text = after(&c, "hello.spec", &[unlink("bh_one", "produces", "ev_one")]);
    assert_eq!(text, TRIPLE.replace(PRODUCES, ""));
    assert!(is_canonical(&c, &text));
}

#[specforge_test_macros::test(
    behavior = "edit_entity_obligation",
    verify = "remove-verify keeps a // line inside the string above the statement"
)]
fn remove_verify_keeps_a_comment_line_inside_a_string_above_the_statement() {
    let c = project(&[("hello.spec", STATEMENT)]);
    let edit = Edit::RemoveVerify {
        entity_id: "bh_one".into(),
        kind: None,
        text: "returns the token".into(),
    };
    let text = after(&c, "hello.spec", &[edit]);
    assert_eq!(
        text,
        STATEMENT.replace("  verify \"returns the token\"\n", "")
    );
    assert!(is_canonical(&c, &text));
}

#[specforge_test_macros::test(
    behavior = "delete_entity",
    verify = "delete with unlink_references keeps a // line inside a string above a reference"
)]
fn delete_keeps_a_comment_line_inside_a_string_above_a_reference() {
    let c = project(&[("hello.spec", TRIPLE)]);
    let edit = Edit::Delete {
        entity_id: "ev_one".into(),
        unlink_references: true,
    };
    let text = after(&c, "hello.spec", &[edit]);
    assert!(!text.contains("ev_one"), "{text}");
    assert!(
        text.contains("    // the caller writes this\"\"\"\n}"),
        "{text}"
    );
}

#[specforge_test_macros::test(
    behavior = "edit_entity_field",
    verify = "set of an absent field never writes inside the string above the first field"
)]
fn a_new_member_never_goes_inside_a_string_above_the_first_field() {
    let c = project(&[("hello.spec", FIRST)]);
    let text = after(&c, "hello.spec", &[set("bh_two", "category", one("query"))]);
    assert_eq!(
        text,
        "behavior bh_two \"B2\" {\n  verify \"Example:\n// the caller writes this\"\n  category query\n  contract \"\"\"\n    text\"\"\"\n}\n"
    );
}
