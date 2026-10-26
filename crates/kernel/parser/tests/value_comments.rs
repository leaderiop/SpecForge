//! Where the comments inside a member's value and after it are written: a list's are the value's, one ending a
//! member's row trails it, a `//` inside a string is none (ADR 0267).

use specforge_common::SourceSpan;
use specforge_parser::{Entity, FieldValue, parse};

fn last(source: &str) -> Entity {
    parse(source, "a.spec").entities.pop().unwrap()
}

fn value(entity: &Entity, key: &str) -> SourceSpan {
    let entry = entity
        .fields
        .entries()
        .iter()
        .find(|e| e.key.as_str() == key);
    entry.unwrap().value_span.clone().unwrap()
}

fn none_anywhere(e: &Entity) -> bool {
    e.body_comments.is_empty() && e.trailing_comments.is_empty() && e.value_comments.is_empty()
}

#[test]
fn a_comment_inside_a_list_is_the_values() {
    let e = last("journey j \"J\" {\n  channels [\n    cli, // primary\n    web\n  ]\n}\n");
    assert_eq!(e.value_comments.len(), 1);
    assert!(e.holds_comment(&value(&e, "channels")));
    assert!(e.body_comments.is_empty() && e.trailing_comments.is_empty());

    let e = last("type A \"A\" {\n  tags [\n    // c\n    \"one\"\n  ]\n}\n");
    assert_eq!(e.value_comments.len(), 1);
    assert!(e.body_comments.is_empty());
}

#[test]
fn a_comment_ending_a_members_row_trails_it() {
    let e = last("type A \"A\" {\n  features [a] // core\n  status x @since \"1\" // t\n}\n");
    for (key, line) in [("features", 2), ("status", 3)] {
        let v = value(&e, key);
        let c = e.trailing_comment(&v).unwrap();
        assert_eq!(c.start_line, line);
        assert!(c.start_col > v.end_col || key == "features");
        assert!(!e.holds_comment(&v));
    }
    let annotated = e.trailing_comment(&value(&e, "status")).unwrap();
    assert!(
        annotated.start_col > 20,
        "past the annotation: {annotated:?}"
    );
}

#[test]
fn a_nested_blocks_comments_are_inside_its_value() {
    let e = last(
        "port p \"P\" {\n  methods {\n    // entry\n    login  \"in\" // primary\n    logout \"out\"\n  }\n}\n",
    );
    let methods = value(&e, "methods");
    assert!(e.holds_comment(&methods));
    assert_eq!(e.comments_above(4).map(|s| s.start_line), Some(3));
    let login = e
        .fields
        .entries()
        .iter()
        .find(|f| f.key.as_str() == "methods")
        .unwrap();
    let FieldValue::Block(inner) = &login.value else {
        panic!("a block field")
    };
    let login_value = inner.entries().first().unwrap().value_span.clone().unwrap();
    assert_eq!(e.trailing_comment(&login_value).unwrap().start_line, 4);
}

#[test]
fn a_comment_between_a_key_and_its_value_is_none_of_its_values() {
    let e = last("journey j \"J\" {\n  persona // who\n    developer\n}\n");
    let v = value(&e, "persona");
    assert!(!e.holds_comment(&v));
    assert!(e.trailing_comment(&v).is_none());
    assert!(none_anywhere(&e));
}

#[test]
fn a_double_slash_inside_a_string_is_no_comment() {
    let e = last("behavior b \"B\" {\n  contract \"a // b\"\n  status x @note \"x // y\"\n}\n");
    assert!(none_anywhere(&e));
}

#[test]
fn the_opening_braces_comment_trails_no_member() {
    let e = last("type A \"A\" { // c\n  a string\n}\n");
    assert_eq!(e.trailing_comments.len(), 1);
    let v = value(&e, "a");
    assert!(e.trailing_comment(&v).is_none());
    assert!(!e.holds_comment(&v));
}
