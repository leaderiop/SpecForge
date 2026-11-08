//! One file parsed with a pack's query: matches, captures, spans, parents, errors and the convention's faults.

use specforge_protocol_types::{SyntaxMatch, SyntaxOutcome, SyntaxRole};
use specforge_source::{CAPTURE_TEXT_CAP, Language, SourceQuery};
use specforge_test_macros::test as specforge_test;

/// A small tags query for Rust: modules, impls, functions (a method inside an impl), structs and calls.
pub const RUST_TAGS: &str = r#"
(mod_item name: (identifier) @name) @definition.module
(impl_item type: (_) @impl.type) @definition.impl
(function_item
  (visibility_modifier)? @visibility
  name: (identifier) @name) @definition.function
(struct_item name: (type_identifier) @name) @definition.struct
(enum_item name: (type_identifier) @name) @definition.enum
(call_expression function: (identifier) @name) @reference.call
"#;

pub fn parsed(language: Language, query: &str, source: &str) -> (Vec<SyntaxMatch>, u32) {
    let query = SourceQuery::compile(language, query).unwrap();
    match query.parse(source.as_bytes()) {
        SyntaxOutcome::Parsed { matches, errors } => (matches, errors),
        other => panic!("{other:?}"),
    }
}

fn capture<'m>(matched: &'m SyntaxMatch, name: &str) -> Option<&'m str> {
    matched
        .captures
        .iter()
        .find(|capture| capture.name == name)
        .map(|capture| capture.text.as_str())
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "a rust file answers every named capture with spans and parents"
)]
fn a_rust_file_answers_every_named_capture_with_spans_and_parents() {
    let source = "mod a {\n    pub struct S;\n    impl S {\n        pub fn m(&self) { helper() }\n    }\n}\nfn helper() {}\n";
    let (matches, errors) = parsed(Language::Rust, RUST_TAGS, source);
    assert_eq!(errors, 0);
    let shown: Vec<(SyntaxRole, &str, Option<&str>)> = matches
        .iter()
        .map(|m| (m.role, m.kind.as_str(), capture(m, "name")))
        .collect();
    assert_eq!(
        shown,
        [
            (SyntaxRole::Definition, "module", Some("a")),
            (SyntaxRole::Definition, "struct", Some("S")),
            (SyntaxRole::Definition, "impl", None),
            (SyntaxRole::Definition, "function", Some("m")),
            (SyntaxRole::Reference, "call", Some("helper")),
            (SyntaxRole::Definition, "function", Some("helper")),
        ]
    );
    // Every named capture, the optional ones included when present.
    assert_eq!(capture(&matches[2], "impl.type"), Some("S"));
    assert_eq!(capture(&matches[3], "visibility"), Some("pub"));
    assert_eq!(capture(&matches[5], "visibility"), None);
    // The innermost definition holding each match.
    let parents: Vec<Option<u32>> = matches.iter().map(|m| m.parent).collect();
    assert_eq!(parents, [None, Some(0), Some(0), Some(2), Some(3), None]);
    // Spans: 0-based bytes, 1-based lines and columns.
    let method = &matches[3].span;
    assert_eq!((method.start.line, method.start.col), (4, 9));
    assert_eq!((method.end.line, method.end.col), (4, 37));
    assert_eq!(method.start.byte as usize, source.find("pub fn m").unwrap());
    let name = &matches[3]
        .captures
        .iter()
        .find(|c| c.name == "name")
        .unwrap();
    assert_eq!((name.span.start.col, name.span.end.col), (16, 17));
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "a broken file answers what the parse recovered and counts its errors"
)]
fn a_broken_file_answers_what_the_parse_recovered_and_counts_its_errors() {
    let source = "pub fn first() -> u8 { 1 }\npub struct Second { a: u8 }\nfn third( { let x = ; }\npub enum Fourth { A, B }\nimpl Second { pub fn fifth(&self) -> u8 { self.a } }\n";
    let (matches, errors) = parsed(Language::Rust, RUST_TAGS, source);
    assert!(errors >= 1, "{errors}");
    let mut names: Vec<&str> = matches
        .iter()
        .filter(|m| m.role == SyntaxRole::Definition)
        .filter_map(|m| capture(m, "name"))
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["Fourth", "Second", "fifth", "first", "third"]);
}

#[test]
fn a_tsx_file_is_parsed_with_the_tsx_grammar() {
    let query = r#"
(function_declaration name: (identifier) @name) @definition.function
(jsx_self_closing_element name: (identifier) @name) @reference.component
"#;
    let source = "export function App() { return <Button label=\"x\" />; }\n";
    let (matches, errors) = parsed(Language::Tsx, query, source);
    assert_eq!(errors, 0);
    let kinds: Vec<(&str, Option<&str>)> = matches
        .iter()
        .map(|m| (m.kind.as_str(), capture(m, "name")))
        .collect();
    assert_eq!(
        kinds,
        [("function", Some("App")), ("component", Some("Button"))]
    );
    // The typescript grammar has no JSX: the same file is broken there.
    let (_, errors) = parsed(Language::TypeScript, "(identifier) @reference.name", source);
    assert!(errors >= 1);
}

#[test]
fn an_anonymous_node_is_captured_too() {
    let query = r#"(export_statement "export" @attr declaration: (function_declaration name: (identifier) @name)) @definition.function"#;
    let (matches, _) = parsed(
        Language::TypeScript,
        query,
        "export function a() {}\nfunction b() {}\n",
    );
    assert_eq!(matches.len(), 1);
    assert_eq!(capture(&matches[0], "attr"), Some("export"));
    assert_eq!(capture(&matches[0], "name"), Some("a"));
}

#[test]
fn text_predicates_filter_matches() {
    let source = "fn alpha() {}\nfn beta() {}\nfn gamma() {}\nfn test_delta() {}\n";
    let names = |predicate: &str| -> Vec<String> {
        let query =
            format!("((function_item name: (identifier) @name) @definition.function {predicate})");
        parsed(Language::Rust, &query, source)
            .0
            .iter()
            .filter_map(|m| capture(m, "name").map(str::to_string))
            .collect()
    };
    assert_eq!(names("(#eq? @name \"beta\")"), ["beta"]);
    assert_eq!(names("(#not-eq? @name \"beta\")").len(), 3);
    assert_eq!(names("(#match? @name \"^test_\")"), ["test_delta"]);
    assert_eq!(
        names("(#any-of? @name \"alpha\" \"gamma\")"),
        ["alpha", "gamma"]
    );
    assert_eq!(
        names("(#not-any-of? @name \"alpha\" \"gamma\")"),
        ["beta", "test_delta"]
    );
}

#[test]
fn a_capture_starting_with_an_underscore_is_not_answered() {
    let query = "(function_item name: (identifier) @_name body: (_) @body) @definition.function";
    let (matches, _) = parsed(Language::Rust, query, "fn a() { 1 }\n");
    let names: Vec<&str> = matches[0]
        .captures
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, ["body"]);
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "a query without exactly one role capture or with a general predicate is a fault"
)]
fn a_query_without_one_role_capture_is_a_fault() {
    let fault = |query: &str| SourceQuery::compile(Language::Rust, query).unwrap_err();
    let none = fault("(function_item name: (identifier) @name)");
    assert_eq!((none.row, none.column), (1, 1));
    assert!(none.message.contains("no role capture"), "{none:?}");
    let two = fault(
        "(struct_item) @definition.struct\n  (function_item name: (identifier) @reference.call) @definition.function",
    );
    assert_eq!((two.row, two.column), (2, 3));
    assert!(two.message.contains("2 role captures"), "{two:?}");
    let optional = fault("(function_item (visibility_modifier)? @definition.x)");
    assert!(optional.message.contains("exactly once"), "{optional:?}");
    let no_kind = fault("(function_item) @definition.");
    assert!(no_kind.message.contains("names no kind"), "{no_kind:?}");
}

#[test]
fn a_general_predicate_is_a_fault() {
    for predicate in [
        "(#set! kind \"x\")",
        "(#strip! @name \"x\")",
        "(#select-adjacent! @name @name)",
        "(#is? local)",
        "(#mine? @name)",
    ] {
        let query = format!("((function_item name: (identifier) @name) @definition.f {predicate})");
        let fault = SourceQuery::compile(Language::Rust, &query).unwrap_err();
        assert!(
            fault.message.contains("is not a text predicate"),
            "{predicate}: {fault:?}"
        );
    }
}

#[test]
fn a_tree_sitter_query_error_is_a_fault_with_its_row_and_column() {
    let fault = SourceQuery::compile(
        Language::Rust,
        "(function_item) @definition.f\n  (no_such_node) @definition.g",
    )
    .unwrap_err();
    assert_eq!((fault.row, fault.column), (2, 4));
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "a capture's text is cut at 4096 bytes on a character boundary"
)]
fn a_capture_text_is_cut_at_4096_bytes_on_a_char_boundary() {
    // A string of 3-byte characters whose 4096th byte falls inside one.
    let body = "é".to_string() + &"€".repeat(2000);
    let source = format!("fn a() {{ \"{body}\" }}\n");
    let (whole, _) = parsed(
        Language::Rust,
        "(string_literal) @definition.text @whole",
        &source,
    );
    assert_eq!(whole.len(), 1);
    let text = &whole[0].captures[0];
    assert!(text.cut);
    assert!(text.text.len() <= CAPTURE_TEXT_CAP && text.text.len() > CAPTURE_TEXT_CAP - 3);
    assert!(text.text.starts_with("\"é€"));
}

#[test]
fn columns_count_characters_not_bytes() {
    let source = "// é€ comment\nfn a() {}  fn b() {}\nconst S: &str = \"€€\"; fn c() {}\n";
    let (matches, _) = parsed(
        Language::Rust,
        "(function_item name: (identifier) @name) @definition.function",
        source,
    );
    let c = &matches[2];
    assert_eq!(c.span.start.line, 3);
    // `const S: &str = "€€"; ` is 22 characters and 26 bytes before `fn c`.
    assert_eq!((c.span.start.col, c.span.start.byte_col), (23, 27));
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "the grammar versions are the locked ones"
)]
fn the_grammar_versions_are_the_locked_ones() {
    let lock =
        std::fs::read_to_string(specforge_repo_root::repo_root().join("Cargo.lock")).unwrap();
    let locked = |name: &str| -> String {
        let at = lock.find(&format!("name = \"{name}\"\n")).unwrap();
        let version = &lock[at..].lines().nth(1).unwrap();
        version
            .strip_prefix("version = \"")
            .and_then(|v| v.strip_suffix('"'))
            .unwrap()
            .to_string()
    };
    for language in Language::ALL {
        let (crate_name, rest) = language.grammar().split_once('@').unwrap();
        let version = rest.split('/').next().unwrap();
        assert_eq!(version, locked(crate_name), "{language:?}");
        assert_eq!(Language::named(language.name()), Some(language));
        // Each grammar loads into this runtime and parses.
        let query = SourceQuery::compile(language, "(_) @definition.any").unwrap();
        assert!(matches!(query.parse(b"x"), SyntaxOutcome::Parsed { .. }));
    }
    assert_eq!(Language::named("javascript"), None);
}
