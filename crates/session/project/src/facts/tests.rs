//! The fact store: its bytes, its admission and the entities it serves.

use super::*;

#[test]
fn a_store_is_written_canonically_whatever_order_its_facts_came_in() {
    let a = Fact::new("test", "js:b#2").field("z", 1).field("a", "x");
    let b = Fact::new("test", "js:a#1").title("one");
    let one = Store::new(
        "node",
        "@t/node",
        "k",
        vec![a.clone(), b.clone()],
        BTreeMap::new(),
    );
    let two = Store::new("node", "@t/node", "k", vec![b, a], BTreeMap::new());

    assert_eq!(one.bytes(), two.bytes());
    let text = String::from_utf8(one.bytes()).unwrap();
    assert!(
        text.ends_with("}\n") && text.contains("\n  \"store\": 2,"),
        "{text}"
    );
    assert!(text.find("js:a#1").unwrap() < text.find("js:b#2").unwrap());
    assert!(text.find("\"a\": \"x\"").unwrap() < text.find("\"z\": 1").unwrap());
    assert_eq!(Store::path("node", "k"), ".specforge/facts/k/node.json");
}

use specforge_common::Exclude;
use specforge_registry::MemoryDisk;
use std::collections::HashMap;

const KEY: &str = ".specforge/facts/abc/node.json";

fn root() -> SourceRoot {
    SourceRoot::new("/p".into(), "/p/spec".into(), Exclude::default())
}

fn two() -> Vec<u8> {
    let facts = vec![
        Fact::new("test", "js:a.test.js#adds")
            .title("adds")
            .field("file", "a.test.js"),
        Fact::new("test", "js:a.test.js#subtracts"),
    ];
    Store::new("node", "@t/node", "abc", facts, BTreeMap::new()).bytes()
}

/// The ledger recording `bytes` as node's store under `abc`.
fn recording(bytes: &[u8]) -> Vec<u8> {
    let mut ledger = Ledger::default();
    ledger.insert(Line::facts("node", "abc", bytes));
    ledger.canonical().into_bytes()
}

fn disk(store: &[u8], ledger: Option<Vec<u8>>) -> MemoryDisk {
    let disk = MemoryDisk::new().with_file(format!("/p/{KEY}"), store);
    match ledger {
        Some(ledger) => disk.with_file("/p/specforge-harness", &ledger),
        None => disk,
    }
}

fn diagnostic(read: Read) -> Diagnostic {
    match read {
        Read::Unreadable(diagnostic) => diagnostic,
        Read::Text(_) => panic!("served"),
        Read::Gone => panic!("gone"),
    }
}

#[specforge_test_macros::test(
    behavior = "build_in_memory_graph",
    verify = "a fact store specforge-harness records is served as read-only entities; one it does not is W191, one that does not read W192"
)]
fn a_recorded_fact_store_is_served_as_one_spec_file() {
    let bytes = two();
    let Read::Text(text) = read(&disk(&bytes, Some(recording(&bytes))), &root(), KEY, None) else {
        panic!("a recorded store is served");
    };

    let file = served(KEY, &text, &FieldForms::default()).unwrap();

    assert_eq!(file.path.as_str(), KEY);
    let ids: Vec<&str> = file.entities.iter().map(|e| e.id.raw.as_str()).collect();
    assert_eq!(ids, ["js:a.test.js#adds", "js:a.test.js#subtracts"]);
    let adds = &file.entities[0];
    assert_eq!(
        (adds.kind.raw.as_str(), adds.title.as_deref()),
        ("test", Some("adds"))
    );
    assert_eq!((adds.span.start_line, adds.name_span.is_none()), (1, true));
    assert!(file.errors.is_empty());
}

#[test]
fn an_unrecorded_fact_store_is_w191_and_not_served() {
    let w191 = diagnostic(read(&disk(&two(), None), &root(), KEY, None));

    assert_eq!(w191.code, "W191");
    assert_eq!(
        w191.message,
        "fact store '.specforge/facts/abc/node.json' is not the one specforge-harness records for collector \
         'node' (collected on another commit or branch, or changed since); its facts are not served"
    );
    assert_eq!(w191.suggestion.as_deref(), Some("run `specforge collect`"));
}

#[test]
fn a_fact_store_whose_digest_differs_is_w191() {
    let other = Store::new("node", "@t/node", "abc", Vec::new(), BTreeMap::new()).bytes();

    let w191 = diagnostic(read(
        &disk(&two(), Some(recording(&other))),
        &root(),
        KEY,
        None,
    ));

    assert_eq!(w191.code, "W191");
}

#[test]
fn an_unreadable_fact_store_is_w192() {
    // A recorded store is read as it is, and parsed once, when it is served: no form of it is dropped silently.
    let not_json = served(KEY, "{ not json", &FieldForms::default()).unwrap_err();
    let newer =
        r#"{"store": 3, "collector": "node", "extension": "@t/node", "key": "abc", "facts": []}"#;
    let newer = served(KEY, newer, &FieldForms::default()).unwrap_err();
    let Read::Text(_) = read(
        &disk(b"{ not json", Some(recording(b"{ not json"))),
        &root(),
        KEY,
        None,
    ) else {
        panic!("a recorded store reads as its text");
    };

    for w192 in [&not_json, &newer] {
        assert_eq!(w192.code, "W192");
        assert!(
            w192.message
                .starts_with("fact store '.specforge/facts/abc/node.json' cannot be read: "),
            "{}",
            w192.message
        );
    }
    assert!(
        newer
            .message
            .ends_with("it is store version 3, and this specforge reads version 2")
    );
    assert!(matches!(
        read(&MemoryDisk::new(), &root(), KEY, None),
        Read::Gone
    ));
}

#[test]
fn a_version_1_store_is_w192() {
    let older =
        r#"{"store": 1, "collector": "node", "extension": "@t/node", "key": "abc", "facts": []}"#;

    let w192 = served(KEY, older, &FieldForms::default()).unwrap_err();

    assert_eq!(w192.code, "W192");
    assert!(
        w192.message
            .ends_with("it is store version 1, and this specforge reads version 2"),
        "{}",
        w192.message
    );
    assert_eq!(
        w192.suggestion.as_deref(),
        Some("run `specforge collect` to write it again")
    );
}

fn place(file: &str, line: usize, from: usize, to: usize) -> FactLocation {
    FactLocation {
        file: file.into(),
        start_line: line,
        start_col: from,
        end_line: line,
        end_col: to,
    }
}

#[specforge_test_macros::test(
    behavior = "build_in_memory_graph",
    verify = "a served fact is located where its collector says it is written, and each name it holds where that name is written"
)]
fn a_served_fact_is_located_where_it_is_written_and_its_names_where_they_are() {
    let fact = Fact::new("test", "js:a#b")
        .at(place("src/a.rs", 12, 1, 20))
        .at(place("src/b.rs", 3, 1, 20))
        .name("names", "v1/TR1", place("src/a.rs", 11, 18, 24))
        .field("status", "passed");
    let bare = Fact::new("test", "js:a#c").field("names", serde_json::json!(["TR2"]));
    let text = String::from_utf8(
        Store::new("node", "@t/node", "abc", vec![fact, bare], BTreeMap::new()).bytes(),
    )
    .unwrap();

    let file = served(KEY, &text, &declared()).unwrap();

    let (located, unlocated) = (&file.entities[0], &file.entities[1]);
    assert_eq!(
        located.span.file.as_str(),
        KEY,
        "a fact keeps its store span"
    );
    let places: Vec<(&str, usize)> = located
        .located
        .iter()
        .map(|s| (s.file.as_str(), s.start_line))
        .collect();
    assert_eq!(places, [("src/a.rs", 12), ("src/b.rs", 3)]);
    let Some(FieldValue::List(names)) = located.fields.get("names") else {
        panic!("names is a list");
    };
    let at = &names[0].span;
    assert_eq!(
        (at.file.as_str(), at.start_line, at.start_col, at.end_col),
        ("src/a.rs", 11, 18, 24)
    );
    let status = located.fields.entries().iter().find(|e| e.key == "status");
    let status = status.and_then(|e| e.value_span.as_ref()).unwrap();
    assert_eq!(
        (status.file.as_str(), status.start_line),
        ("src/a.rs", 12),
        "a value with no name place is placed at the fact"
    );
    let Some(FieldValue::List(names)) = unlocated.fields.get("names") else {
        panic!("names is a list");
    };
    assert_eq!(
        (names[0].span.file.as_str(), names[0].span.start_line),
        (KEY, 1)
    );
    assert!(unlocated.located.is_empty());
}

/// `test`'s `status` declared an enum and its `names` a reference list.
fn declared() -> FieldForms {
    use specforge_protocol_types::FieldType;
    let field = |name: &str| ("test".to_string(), name.to_string());
    let status = FieldForm::new(FieldType::Enum, vec!["passed".into(), "failed".into()]);
    let names = FieldForm::new(FieldType::ReferenceList, Vec::new());
    FieldForms::new(
        HashMap::from([(field("status"), status), (field("names"), names)]),
        BTreeSet::new(),
        BTreeSet::from(["test".to_string()]),
    )
}

#[specforge_test_macros::test(
    behavior = "build_in_memory_graph",
    verify = "a fact's value is read by its field's form, as a spec file writes it: a word field's word is bare"
)]
fn a_facts_strings_are_read_by_their_fields_form() {
    let fact = Fact::new("test", "js:a#b")
        .field("status", "passed")
        .field("file", "a.test.js")
        .field("names", serde_json::json!(["v1/TR1", "a b"]));
    let text = String::from_utf8(
        Store::new("node", "@t/node", "abc", vec![fact], BTreeMap::new()).bytes(),
    )
    .unwrap();

    let file = served(KEY, &text, &declared()).unwrap();

    let fields = &file.entities[0].fields;
    assert_eq!(
        fields.get("status"),
        Some(&FieldValue::Identifier("passed".into()))
    );
    assert_eq!(
        fields.get("file"),
        Some(&FieldValue::String("a.test.js".into())),
        "an undeclared field's string stays text"
    );
    let Some(FieldValue::List(names)) = fields.get("names") else {
        panic!("an array of strings is a list");
    };
    let names: Vec<(&str, Written)> = names.iter().map(|n| (n.text.as_str(), n.written)).collect();
    assert_eq!(names, [("v1/TR1", Written::Bare), ("a b", Written::Quoted)]);
}

#[test]
fn a_facts_field_values_become_field_values_by_their_form() {
    let fact = Fact::new("test", "js:a#b")
        .field("file", "a.test.js")
        .field("runs", 3)
        .field("flaky", true)
        .field("tags", serde_json::json!(["slow", "js:a#c"]))
        .field("meta", serde_json::json!({"x": 1}))
        .field("ratio", 0.5);
    let text = String::from_utf8(
        Store::new("node", "@t/node", "abc", vec![fact], BTreeMap::new()).bytes(),
    )
    .unwrap();

    let file = served(KEY, &text, &FieldForms::default()).unwrap();

    let fields = &file.entities[0].fields;
    assert_eq!(
        fields.get("file"),
        Some(&FieldValue::String("a.test.js".into()))
    );
    assert_eq!(fields.get("runs"), Some(&FieldValue::Integer(3)));
    assert_eq!(fields.get("flaky"), Some(&FieldValue::Boolean(true)));
    let Some(FieldValue::List(tags)) = fields.get("tags") else {
        panic!("an array of strings is a list");
    };
    let tags: Vec<(&str, Written)> = tags.iter().map(|t| (t.text.as_str(), t.written)).collect();
    assert_eq!(
        tags,
        [("slow", Written::Quoted), ("js:a#c", Written::Quoted)]
    );
    assert_eq!((fields.get("meta"), fields.get("ratio")), (None, None));
}
