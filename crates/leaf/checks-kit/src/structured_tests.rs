//! The structured family's syntax: one tree for JSON, YAML and TOML, JSON pointers, and Markdown heading anchors.

use serde_json::json;

use crate::pointer::{resolve, tokens, written};
use crate::slug::{anchors, heading_anchors, headings, slug};
use crate::tree::{Format, MAX_DEPTH, parse};

/// The document of RFC 6901 §5.
fn rfc_document() -> serde_json::Value {
    json!({
        "foo": ["bar", "baz"], "": 0, "a/b": 1, "c%d": 2, "e^f": 3, "g|h": 4,
        "i\\j": 5, "k\"l": 6, " ": 7, "m~n": 8
    })
}

#[test]
fn pointer_fragments_resolve_as_rfc_6901_section_6_reads_them() {
    let document = rfc_document();
    let at = |fragment: &str| resolve(&document, &tokens(fragment).unwrap()).cloned();
    assert_eq!(at(""), Ok(document.clone()));
    assert_eq!(at("/foo"), Ok(json!(["bar", "baz"])));
    assert_eq!(at("/foo/0"), Ok(json!("bar")));
    assert_eq!(at("/"), Ok(json!(0)));
    assert_eq!(at("/a~1b"), Ok(json!(1)));
    assert_eq!(at("/c%25d"), Ok(json!(2)));
    assert_eq!(at("/e%5Ef"), Ok(json!(3)));
    assert_eq!(at("/g%7Ch"), Ok(json!(4)));
    assert_eq!(at("/i%5Cj"), Ok(json!(5)));
    assert_eq!(at("/k%22l"), Ok(json!(6)));
    assert_eq!(at("/%20"), Ok(json!(7)));
    assert_eq!(at("/m~0n"), Ok(json!(8)));
}

#[test]
fn a_pointer_that_misses_says_how_far_it_reached() {
    let document = rfc_document();
    let missing = tokens("/foo/2").unwrap();
    assert_eq!(resolve(&document, &missing), Err(1));
    assert_eq!(written(&missing, 1), "/foo");
    // An index is digits with no leading zero.
    assert_eq!(resolve(&document, &tokens("/foo/01").unwrap()), Err(1));
    assert_eq!(resolve(&document, &tokens("/nope/x").unwrap()), Err(0));
    assert_eq!(written(&tokens("/a~1b/m~0n").unwrap(), 2), "/a~1b/m~0n");
}

#[test]
fn a_pointer_that_is_malformed_is_invalid() {
    assert!(
        tokens("foo")
            .unwrap_err()
            .reason
            .contains("starts with '/'")
    );
    assert!(tokens("/a~2").unwrap_err().reason.contains("'~0' or '~1'"));
    assert!(tokens("/%FF").unwrap_err().reason.contains("not UTF-8"));
}

#[test]
fn a_yaml_tree_expands_anchors_and_writes_non_string_keys_as_json() {
    let yaml = "a: &x [1, 2]\nb: *x\n1: one\ntrue: yes\nquoted: \"1\"\nnothing: ~\n";
    assert_eq!(
        parse(Format::Yaml, yaml.as_bytes()),
        Ok(json!({
            "a": [1, 2], "b": [1, 2], "1": "one", "true": "yes", "quoted": "1", "nothing": null
        }))
    );
    assert_eq!(parse(Format::Yaml, b""), Ok(json!(null)));
    let undefined = parse(Format::Yaml, b"a: *nope\n").unwrap_err();
    assert!(undefined.message.contains("anchor"), "{undefined}");
}

#[test]
fn a_billion_laughs_stops_at_the_byte_cap() {
    let mut yaml = String::from("a: &a [x, x, x, x, x, x, x, x, x, x]\n");
    for (level, name) in ["b", "c", "d", "e", "f", "g", "h"].iter().enumerate() {
        let previous = ["a", "b", "c", "d", "e", "f", "g"][level];
        let row = vec![format!("*{previous}"); 10].join(", ");
        yaml.push_str(&format!("{name}: &{name} [{row}]\n"));
    }
    let error = parse(Format::Yaml, yaml.as_bytes()).unwrap_err();
    assert!(
        error.message.contains("bytes once its aliases are read"),
        "{error}"
    );
}

#[test]
fn an_alias_of_a_long_string_counts_its_bytes() {
    // 600 KB copied 64 times is 38 MB of strings in 66 nodes.
    let long = "x".repeat(600 * 1024);
    let row = vec!["*s"; 64].join(", ");
    let yaml = format!("s: &s \"{long}\"\nt: [{row}]\n");
    let error = parse(Format::Yaml, yaml.as_bytes()).unwrap_err();
    assert!(
        error.message.contains("bytes once its aliases are read"),
        "{error}"
    );
    let few = format!("s: &s \"{long}\"\nt: [*s, *s]\n");
    assert!(parse(Format::Yaml, few.as_bytes()).is_ok());
}

#[test]
fn a_parse_error_names_its_line_and_column() {
    let json = parse(Format::Json, b"{\n  \"a\": 1,\n  \"b\" 2\n}").unwrap_err();
    assert_eq!((json.line, json.column), (Some(3), Some(7)), "{json}");
    let yaml = parse(Format::Yaml, b"a: [1, 2\nb: 3\n").unwrap_err();
    assert!(yaml.line.is_some(), "{yaml}");
    let toml = parse(Format::Toml, b"a = 1\nb = \n").unwrap_err();
    assert_eq!(toml.line, Some(2), "{toml}");
    let bytes = parse(Format::Json, b"{\"a\": \"\xff\"}").unwrap_err();
    assert_eq!(
        (bytes.message.as_str(), bytes.line, bytes.column),
        ("is not UTF-8", Some(1), Some(8))
    );
}

#[test]
fn a_toml_datetime_is_its_string_and_a_bom_is_dropped() {
    let toml = "\u{feff}when = 1979-05-27T07:32:00Z\n[table]\nday = 1979-05-27\n";
    assert_eq!(
        parse(Format::Toml, toml.as_bytes()),
        Ok(json!({ "when": "1979-05-27T07:32:00Z", "table": { "day": "1979-05-27" } }))
    );
    assert_eq!(
        parse(Format::Json, "\u{feff}[1]".as_bytes()),
        Ok(json!([1]))
    );
}

#[test]
fn a_tree_deeper_than_the_cap_does_not_parse() {
    let deep = format!("{}{}", "[".repeat(MAX_DEPTH + 1), "]".repeat(MAX_DEPTH + 1));
    assert!(parse(Format::Json, deep.as_bytes()).is_err());
    let yaml = format!("{}{}", "[".repeat(MAX_DEPTH + 1), "]".repeat(MAX_DEPTH + 1));
    let error = parse(Format::Yaml, yaml.as_bytes()).unwrap_err();
    assert!(error.message.contains("nests deeper"), "{error}");
}

#[test]
fn a_format_is_read_from_the_extension() {
    assert_eq!(Format::of_path("a/b.YML"), Some(Format::Yaml));
    assert_eq!(Format::of_path("Cargo.toml"), Some(Format::Toml));
    assert_eq!(Format::of_path("README"), None);
}

#[test]
fn a_heading_slug_is_githubs() {
    assert_eq!(slug("Hello, World!"), "hello-world");
    assert_eq!(slug("Café ünïcode"), "café-ünïcode");
    assert_eq!(slug("[Install](x.md) guide"), "install-guide");
    assert_eq!(slug("`a_b` c"), "a_b-c");
    assert_eq!(slug("_emphasis_ and snake_case"), "emphasis-and-snake_case");
    assert_eq!(slug("<em>tag</em> - dash"), "tag---dash");
}

#[test]
fn duplicate_headings_are_numbered_and_an_explicit_id_wins() {
    let md = "# A\n## A\n### A\n## Custom {#custom}\n";
    assert_eq!(heading_anchors(md), ["a", "a-1", "a-2", "custom"]);
}

#[test]
fn only_a_heading_outside_code_and_html_blocks_names_an_anchor() {
    let md = "\
# Top
```
# fenced
```
~~~~
# tilde
~~~~
    # indented
<div>
# in html
</div>

Setext
------
> # Quoted
- # Listed
1. ## Numbered
#hashtag
***
";
    assert_eq!(
        headings(md),
        ["Top", "Setext", "Quoted", "Listed", "Numbered"]
    );
}

#[test]
fn a_files_anchors_hold_its_html_ids_and_names() {
    let md = "# Title\n\n<a name=\"legacy\"></a>\n\nText <span id=\"inline\">x</span>\n";
    let found: Vec<String> = anchors(md).into_iter().collect();
    assert_eq!(found, ["inline", "legacy", "title"]);
}

#[test]
fn atx_and_setext_headings_follow_the_commonmark_examples() {
    let heading = |md: &str| headings(md);
    // CommonMark 0.31, §4.2 and §4.3.
    assert_eq!(heading("#5 bolt\n\n#hashtag\n"), Vec::<String>::new());
    assert_eq!(heading("####### foo\n"), Vec::<String>::new());
    assert_eq!(heading("\\## foo\n"), Vec::<String>::new());
    assert_eq!(heading("#\tfoo\n"), ["foo"]);
    assert_eq!(heading("   ### foo\n"), ["foo"]);
    assert_eq!(heading("    # foo\n"), Vec::<String>::new());
    assert_eq!(heading("## foo ##\n"), ["foo"]);
    assert_eq!(heading("# foo#\n"), ["foo#"]);
    assert_eq!(heading("### foo ### b\n"), ["foo ### b"]);
    assert_eq!(heading("#\n### ###\n"), ["", ""]);
    assert_eq!(heading("Foo *bar*\n=========\n"), ["Foo *bar*"]);
    assert_eq!(heading("Foo\nbar\n---\n"), ["Foo bar"]);
    assert_eq!(heading("Foo\n    ---\n"), Vec::<String>::new());
    assert_eq!(heading("---\nFoo\n---\n"), ["Foo"]);
    // An underline outside a block quote or a list item is a thematic break.
    assert_eq!(heading("> foo\n-----\n"), Vec::<String>::new());
    assert_eq!(heading("- foo\n-----\n"), Vec::<String>::new());
    assert_eq!(heading("> foo\n> ---\n"), ["foo"]);
}

#[test]
fn a_heading_of_unclosed_markup_is_read_in_linear_time() {
    for unit in ["<", "]("] {
        let markdown = format!("# {}\n", unit.repeat((1 << 20) / unit.len()));
        let started = std::time::Instant::now();
        let found = anchors(&markdown);
        assert_eq!(found.len(), 1);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "`{unit}` took {:?}",
            started.elapsed()
        );
    }
}
