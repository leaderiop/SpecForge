//! The path grammar, the walker, the printer and the shapes.

use super::*;

/// A picked string with an escape sequence reaches a pipe exactly and a terminal escaped, newlines kept.
#[test]
fn a_picked_string_is_escaped_before_a_terminal_only() {
    use crate::terminal::{Facts, Terminal};
    let document = serde_json::json!({"text": "red \u{1b}[31mX\nnext"});
    let pick = Pick::parse(".text").unwrap();
    let printed = |facts: Facts| {
        let (terminal, capture) = Terminal::captured("", facts);
        print_json(&terminal, Some(&pick), &Shape::default(), &document);
        capture.stdout()
    };
    assert_eq!(printed(Facts::PIPED), "red \u{1b}[31mX\nnext\n");
    let on_terminal = printed(Facts {
        out_terminal: true,
        ..Facts::PIPED
    });
    assert!(!on_terminal.contains('\u{1b}'), "{on_terminal:?}");
    assert!(on_terminal.contains("\nnext"), "{on_terminal:?}");
}
use serde_json::json;

fn picked(path: &str, document: &Value) -> String {
    Shape::default().print(Pick::parse(path).unwrap().texts(document, None))
}

#[test]
fn the_grammar_has_four_steps() {
    assert_eq!(
        Pick::parse(".nodes[kind=plan].fields.goal[0][]").unwrap().0,
        [
            Step::Key("nodes".into()),
            Step::Field(vec!["kind".into()], "plan".into()),
            Step::Key("fields".into()),
            Step::Key("goal".into()),
            Step::Index(0),
            Step::Each,
        ]
    );
}

#[test]
fn a_malformed_path_names_its_byte() {
    for (path, at) in [
        ("", "byte 0"),
        ("nodes", "byte 0"),
        (".a..b", "byte 2"),
        (".a[", "byte 2"),
        (".a[x]", "byte 2"),
        (".a[-1]", "byte 2"),
        (".a[kind=]", "byte 2"),
        (".a[=plan]", "byte 2"),
        (".a[a b=x]", "byte 2"),
        (".a.", "byte 2"),
        (".a[0]b", "byte 5"),
        (".a[+1]", "byte 2"),
        (".a[01]", "byte 2"),
        (".a[00]", "byte 2"),
        (".a b", "byte 2"),
        (".a\tb", "byte 2"),
        (".a]", "byte 2"),
        (".kind=plan", "byte 5"),
    ] {
        let error = Pick::parse(path).unwrap_err();
        assert!(error.starts_with(&format!("at {at}:")), "{path}: {error}");
    }
}

#[test]
fn a_dot_alone_is_the_whole_answer_on_one_line() {
    let document = json!({"activeHooks": [{"capId": "security"}], "skippedHooks": []});
    assert_eq!(
        picked(".", &document),
        "{\"activeHooks\":[{\"capId\":\"security\"}],\"skippedHooks\":[]}\n"
    );
    assert_eq!(picked(".", &json!("s")), "s\n");
}

#[test]
fn an_index_is_digits_without_a_leading_zero() {
    assert_eq!(Pick::parse(".a[0]").unwrap().0[1], Step::Index(0));
    assert_eq!(Pick::parse(".a[10]").unwrap().0[1], Step::Index(10));
}

#[test]
fn a_number_prints_as_serde_json_writes_an_f64() {
    // not `arbitrary_precision`: a number is an i64, a u64 or an f64, and an f64 keeps its `.0`
    let document: Value = serde_json::from_str(r#"{"a": 1.0, "b": -0, "c": 1e3, "d": 7}"#).unwrap();
    assert_eq!(picked(".a", &document), "1.0\n");
    assert_eq!(picked(".b", &document), "-0.0\n");
    assert_eq!(picked(".c", &document), "1000.0\n");
    assert_eq!(picked(".d", &document), "7\n");
}

#[test]
fn each_value_prints_by_its_type() {
    let document =
        json!({"s": "bare", "n": 1.5, "b": false, "o": {"a": [1, null]}, "z": null, "l": "a\nb"});
    assert_eq!(picked(".s", &document), "bare\n");
    assert_eq!(picked(".n", &document), "1.5\n");
    assert_eq!(picked(".b", &document), "false\n");
    assert_eq!(picked(".o", &document), "{\"a\":[1,null]}\n");
    assert_eq!(picked(".o.a", &document), "[1,null]\n");
    assert_eq!(picked(".z", &document), "");
    assert_eq!(picked(".l", &document), "a\nb\n");
}

#[test]
fn a_missing_value_prints_nothing() {
    let document = json!({"a": [], "s": "x"});
    for path in [
        ".b",
        ".a[0]",
        ".a[]",
        ".s.k",
        ".s[0]",
        ".s[]",
        "[0]",
        ".a[kind=plan]",
    ] {
        assert_eq!(picked(path, &document), "", "{path}");
    }
}

#[test]
fn every_item_and_the_items_of_a_kind() {
    let document = json!({"nodes": [
        {"id": "p", "kind": "phase"}, {"id": "a", "kind": "plan"}, {"id": "b", "kind": "plan"}, 7,
    ]});
    assert_eq!(picked(".nodes[].id", &document), "p\na\nb\n");
    assert_eq!(picked(".nodes[kind=plan].id", &document), "a\nb\n");
    assert_eq!(picked(".nodes[kind=task]", &document), "");
    assert_eq!(picked(".nodes[3]", &document), "7\n");
    assert_eq!(picked(".nodes[]", &document).lines().count(), 4);
}

#[test]
fn an_item_is_selected_by_any_string_field() {
    let settings = json!({"extensions": [{"settings": [
        {"name": "workflow.auto_advance", "value": true}, {"name": "workflow.other", "value": "x"},
        {"name": "workflow.unset"},
    ]}]});
    let path = ".extensions[].settings[name=workflow.auto_advance].value";
    assert_eq!(picked(path, &settings), "true\n");
    assert_eq!(
        picked(
            ".extensions[].settings[name=workflow.unset].value",
            &settings
        ),
        ""
    );
    let diagnostics = json!([{"code": "gsd/W205"}, {"code": "E003"}, {"code": 205}]);
    assert_eq!(picked(".[code=gsd/W205].code", &diagnostics), "gsd/W205\n");
    assert_eq!(picked("[code=E003].code", &diagnostics), "E003\n");
    // a number is compared as JSON, and the value may hold '=' and '.'
    assert_eq!(picked(".[code=205].code", &diagnostics), "205\n");
    assert_eq!(
        picked(".[a=b=c.d]", &json!([{"a": "b=c.d"}])),
        "{\"a\":\"b=c.d\"}\n"
    );
}

#[test]
fn a_dotted_key_selects_by_a_nested_field() {
    let nodes = json!({"nodes": [
        {"id": "t1", "fields": {"result": "issue"}}, {"id": "t2", "fields": {"result": "pass"}},
        {"id": "t3", "fields": {}}, {"id": "t4"}, {"id": "t5", "fields": "issue"},
    ]});
    assert_eq!(picked(".nodes[fields.result=issue].id", &nodes), "t1\n");
    assert_eq!(picked(".nodes[fields.result=pass].id", &nodes), "t2\n");
    assert_eq!(picked(".nodes[fields.missing=x].id", &nodes), "");
    assert_eq!(
        Pick::parse(".nodes[fields.result=issue]").unwrap().0[1],
        Step::Field(vec!["fields".into(), "result".into()], "issue".into())
    );
    for path in [".a[.x=1]", ".a[x.=1]", ".a[x..y=1]"] {
        assert!(
            Pick::parse(path).unwrap_err().starts_with("at byte 2:"),
            "{path}"
        );
    }
}

#[test]
fn a_scalar_that_is_not_a_string_is_compared_as_json() {
    let nodes = json!({"nodes": [
        {"id": "a", "fields": {"backlog": true, "wave": 2}},
        {"id": "b", "fields": {"backlog": false, "wave": 1}},
        {"id": "c", "fields": {"backlog": "true", "wave": "2"}},
        {"id": "d", "fields": {"backlog": [true], "wave": {"n": 2}}},
        {"id": "e", "fields": {"backlog": null}},
    ]});
    assert_eq!(picked(".nodes[fields.backlog=true].id", &nodes), "a\nc\n");
    assert_eq!(picked(".nodes[fields.backlog=false].id", &nodes), "b\n");
    assert_eq!(picked(".nodes[fields.wave=2].id", &nodes), "a\nc\n");
    assert_eq!(picked(".nodes[fields.backlog=null].id", &nodes), "e\n");
    // an object or an array is never selected
    assert_eq!(picked(".nodes[fields.wave={\"n\":2}].id", &nodes), "");
}

#[test]
fn fields_print_one_row_per_item_and_an_empty_cell_for_a_missing_value() {
    let nodes = json!({"nodes": [
        {"id": "c1", "fields": {"hash": "abc", "subject": "feat: a", "n": 2}},
        {"id": "c2", "fields": {"subject": "tab\there\nnext \\ end"}},
        {"id": "c3", "fields": {"hash": null, "files": ["a", "b"]}},
    ]});
    let fields = Fields::parse("fields.hash,fields.subject,id,fields.n,fields.files").unwrap();
    let pick = Pick::parse(".nodes[]").unwrap();
    assert_eq!(
        pick.printed(&nodes.to_string(), Some(&fields), &Shape::default())
            .unwrap(),
        "abc\u{1f}feat: a\u{1f}c1\u{1f}2\u{1f}\n\u{1f}tab\there\\nnext \\\\ end\u{1f}c2\u{1f}\u{1f}\n\u{1f}\u{1f}c3\u{1f}\u{1f}[\"a\",\"b\"]\n"
    );
    // a unit separator in a cell is written out, so it cannot split the row
    assert_eq!(escaped("a\u{1f}b"), "a\\x1fb");
    // a value that is null still holds its row; an item missing from the path holds none
    let one = Fields::parse("id").unwrap();
    assert_eq!(
        Pick::parse(".a[]")
            .unwrap()
            .texts(&json!({"a": [null, 1]}), Some(&one)),
        ["", ""]
    );
    assert!(
        Pick::parse(".nodes[kind=x]")
            .unwrap()
            .texts(&nodes, Some(&one))
            .is_empty()
    );
    for bad in ["", "a,", ",a", "a..b", "a b", "a[0]", "a=b"] {
        assert!(Fields::parse(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn an_answer_that_is_not_json_is_refused_and_an_empty_one_prints_nothing() {
    let pick = Pick::parse(".a").unwrap();
    let plain = Shape::default();
    assert_eq!(pick.printed("{\"a\": 1}\n", None, &plain), Ok("1\n".into()));
    assert_eq!(pick.printed("", None, &plain), Ok(String::new()));
    assert!(
        pick.printed("done", None, &plain)
            .unwrap_err()
            .contains("not JSON")
    );
}

fn shaped(shape: &Shape, path: &str, document: &Value, fields: Option<&str>) -> String {
    let fields = fields.map(|keys| Fields::parse(keys).unwrap());
    Pick::parse(path)
        .unwrap()
        .printed(&document.to_string(), fields.as_ref(), shape)
        .unwrap()
}

#[test]
fn a_count_is_the_number_of_values_a_pick_prints() {
    let count = Shape {
        count: true,
        ..Shape::default()
    };
    let document = json!({"a": [1, null, "x\ny", {"b": 2}], "z": null});
    assert_eq!(shaped(&count, ".a[]", &document, None), "3\n");
    assert_eq!(
        picked(".a[]", &document).lines().count(),
        4,
        "a newline in a string is still one value"
    );
    assert_eq!(shaped(&count, ".z", &document, None), "0\n");
    assert_eq!(shaped(&count, ".missing[]", &document, None), "0\n");
    assert_eq!(shaped(&count, ".", &document, None), "1\n");
}

#[test]
fn first_and_last_keep_the_ends_in_document_order() {
    let document = json!({"a": ["p", "q", "r"], "n": [{"id": "x"}, null, {"id": "z"}]});
    let first = |n| Shape {
        first: Some(n),
        ..Shape::default()
    };
    let last = |n| Shape {
        last: Some(n),
        ..Shape::default()
    };
    assert_eq!(shaped(&first(2), ".a[]", &document, None), "p\nq\n");
    assert_eq!(shaped(&last(2), ".a[]", &document, None), "q\nr\n");
    assert_eq!(shaped(&first(0), ".a[]", &document, None), "");
    assert_eq!(shaped(&last(0), ".a[]", &document, None), "");
    assert_eq!(shaped(&first(9), ".a[]", &document, None), "p\nq\nr\n");
    assert_eq!(shaped(&last(9), ".a[]", &document, None), "p\nq\nr\n");
    // a `--fields` row is one text, null included
    assert_eq!(shaped(&last(2), ".n[]", &document, Some("id")), "\nz\n");
    assert_eq!(shaped(&first(1), ".n[]", &document, Some("id")), "x\n");
}

#[test]
fn join_puts_the_kept_values_on_one_line() {
    let document = json!({"a": ["p", "q", "r"], "none": []});
    let join = |separator: &str, first: Option<usize>| Shape {
        join: Some(separator.to_string()),
        first,
        ..Shape::default()
    };
    assert_eq!(shaped(&join(",", None), ".a[]", &document, None), "p,q,r\n");
    assert_eq!(
        shaped(&join(" ⟶ ", None), ".a[]", &document, None),
        "p ⟶ q ⟶ r\n"
    );
    assert_eq!(
        shaped(&join("\n    ", None), ".a[]", &document, None),
        "p\n    q\n    r\n"
    );
    assert_eq!(
        shaped(&join("\\n", None), ".a[]", &document, None),
        "p\\nq\\nr\n",
        "no escape is read"
    );
    assert_eq!(shaped(&join(",", None), ".none[]", &document, None), "");
    assert_eq!(
        shaped(&join(", ", Some(2)), ".a[]", &document, None),
        "p, q\n"
    );
    let last = Shape {
        last: Some(1),
        ..join(",", None)
    };
    assert_eq!(shaped(&last, ".a[]", &document, None), "r\n");
}

#[test]
fn an_empty_answer_prints_no_shape() {
    let pick = Pick::parse(".a[]").unwrap();
    for shape in [
        Shape {
            count: true,
            ..Shape::default()
        },
        Shape {
            join: Some(",".into()),
            ..Shape::default()
        },
        Shape {
            first: Some(1),
            ..Shape::default()
        },
    ] {
        assert_eq!(
            pick.printed("", None, &shape),
            Ok(String::new()),
            "{shape:?}"
        );
        assert_eq!(
            pick.printed("  \n", None, &shape),
            Ok(String::new()),
            "{shape:?}"
        );
    }
}

#[test]
fn the_shape_names_are_the_ids_the_args_derive() {
    use clap::Args as _;
    let derived = Shape::augment_args(clap::Command::new(""));
    let ids: Vec<&str> = derived
        .get_arguments()
        .map(|arg| arg.get_id().as_str())
        .collect();
    assert_eq!(ids, SHAPES);
    for arg in derived.get_arguments() {
        assert_eq!(arg.get_long(), Some(arg.get_id().as_str()));
    }
}
