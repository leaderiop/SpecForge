//! Where a member's comment lines are written: the whole-line comments of a block's body directly above it, by the
//! doc comment's rule; a `//` inside a string is none (ADR 0248).

use specforge_parser::parse;

/// The first and last line of the comment run above 1-based `line` in the file's last block.
fn above(source: &str, line: usize) -> Option<(usize, usize)> {
    parse(source, "a.spec")
        .entities
        .last()
        .unwrap()
        .comments_above(line)
        .map(|s| (s.start_line, s.end_line))
}

fn comments(source: &str) -> usize {
    parse(source, "a.spec")
        .entities
        .last()
        .unwrap()
        .body_comments
        .len()
}

#[test]
fn a_members_comments_are_the_comment_lines_directly_above_it() {
    let s = "type A \"A\" {\n  a string\n  // one\n  // two\n  b string\n}\n";
    assert_eq!(above(s, 5), Some((3, 4)));
    assert_eq!(above(s, 2), None);
}

#[test]
fn a_comment_line_inside_a_string_is_no_comment() {
    for s in [
        "behavior b \"B\" {\n  contract \"\"\"\n    Example:\n    // the caller writes this\"\"\"\n  produces [e]\n}\n",
        "behavior b \"B\" {\n  contract \"Example:\n// the caller writes this\"\n  produces [e]\n}\n",
    ] {
        assert_eq!(above(s, if s.contains("\"\"\"") { 5 } else { 4 }), None);
        assert_eq!(comments(s), 0);
    }
}

#[test]
fn a_comment_ending_a_line_trails_it() {
    let s = "type A \"A\" { // brace\n  a string // after a\n  b string\n}\n";
    assert_eq!(above(s, 2), None);
    assert_eq!(above(s, 3), None);
    assert_eq!(comments(s), 0);
}

#[test]
fn a_comment_after_a_blank_line_is_none() {
    let s = "type A \"A\" {\n  // note\n\n  b string\n}\n";
    assert_eq!(above(s, 4), None);
    assert_eq!(comments(s), 1);
}

#[test]
fn a_nested_entrys_and_a_statements_comments_are_read() {
    let s = "type A \"A\" {\n  steps {\n    // above a\n    a \"x\"\n  }\n  // above verify\n  verify \"v\"\n}\n";
    assert_eq!(above(s, 4), Some((3, 3)));
    assert_eq!(above(s, 7), Some((6, 6)));
}

#[test]
fn the_header_row_and_the_doc_comment_are_no_members() {
    let s = "// doc\ntype A \"A\" { // c\n  a string\n}\n";
    let file = parse(s, "a.spec");
    assert_eq!(file.entities[0].doc_span.as_ref().unwrap().start_line, 1);
    assert_eq!(above(s, 3), None);
}
