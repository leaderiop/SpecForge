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

    let (file, _) = served(KEY, &text, &FieldForms::default()).unwrap();

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
    let sources = ["src/a.rs", "src/b.rs"].map(|f| (f.to_string(), "sha256:aa".to_string()));
    let text = String::from_utf8(
        Store::new("node", "@t/node", "abc", vec![fact, bare], sources.into()).bytes(),
    )
    .unwrap();

    let (file, _) = served(KEY, &text, &declared()).unwrap();

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

#[specforge_test_macros::test(
    behavior = "build_in_memory_graph",
    verify = "a served store gives the digest it recorded of each source file its facts are placed in"
)]
fn a_store_records_the_digest_of_each_located_file() {
    let fact = Fact::new("test", "js:a#b").at(place("src/a.rs", 12, 1, 20));
    let sources = BTreeMap::from([("src/a.rs".to_string(), "sha256:aa".to_string())]);
    let text = String::from_utf8(
        Store::new("node", "@t/node", "abc", vec![fact], sources.clone()).bytes(),
    )
    .unwrap();

    let (_, served_sources) = served(KEY, &text, &declared()).unwrap();

    let digests: BTreeMap<String, String> = served_sources
        .into_iter()
        .map(|(file, placed)| (file, placed.digest))
        .collect();
    assert_eq!(digests, sources);
}

/// A store whose fact is placed in a file it records no digest of: no collect writes one, another writer can. Nothing
/// could judge whether that file is the text its places are in, so it is not served.
#[specforge_test_macros::test(
    behavior = "build_in_memory_graph",
    verify = "a fact store placing a fact in a file it records no digest of is W192 and not served"
)]
fn a_store_placing_a_fact_in_a_file_it_records_no_digest_of_is_w192() {
    let fact = Fact::new("test", "js:a#b")
        .at(place("src/a.rs", 12, 1, 20))
        .name("names", "T1", place("src/b.rs", 1, 4, 6));
    let sources = BTreeMap::from([("src/a.rs".to_string(), "sha256:aa".to_string())]);
    let text = String::from_utf8(Store::new("node", "@t/node", "abc", vec![fact], sources).bytes())
        .unwrap();

    let w192 = store(KEY, &text).unwrap_err();

    assert_eq!(w192.code, "W192");
    assert_eq!(
        w192.message,
        format!(
            "fact store '{KEY}' cannot be read: its fact 'js:a#b' is placed in src/b.rs, of which it records no digest"
        )
    );
    assert_eq!(
        w192.suggestion.as_deref(),
        Some("run `specforge collect` to write it again")
    );
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

    let (file, _) = served(KEY, &text, &declared()).unwrap();

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

    let (file, _) = served(KEY, &text, &FieldForms::default()).unwrap();

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

#[specforge_test_macros::test(
    behavior = "build_in_memory_graph",
    verify = "a stale store's places are shifted exactly by what the renames wrote, and only the text they left holds"
)]
fn a_store_served_through_renames_shifts_its_places_by_what_they_wrote() {
    let collected = "// covers T1 T5 and T1\ntest('adds', () => {});\n";
    let fact = Fact::new("test", "js:a.test.js#adds")
        .at(place("a.test.js", 1, 1, 23))
        .at(place("a.test.js", 2, 1, 24))
        .name("covers", "T1", place("a.test.js", 1, 11, 13))
        .name("covers", "T5", place("a.test.js", 1, 14, 16))
        .name("covers", "T1", place("a.test.js", 1, 21, 23));
    let sources = BTreeMap::from([(
        "a.test.js".to_string(),
        format!(
            "sha256:{}",
            specforge_package::Sha256::of(collected.as_bytes())
        ),
    )]);
    let bytes =
        String::from_utf8(Store::new("node", "@t/node", "abc", vec![fact], sources).bytes())
            .unwrap();
    let renames: Renames = [(
        "covers".to_string(),
        [("T1".to_string(), "Longer".to_string())].into(),
    )]
    .into();
    let text = through(bytes, renames);

    let served = store(KEY, &text).unwrap();

    let fact = &served.store.facts[0];
    let spans = |places: &[FactLocation]| -> Vec<(usize, usize, usize)> {
        places
            .iter()
            .map(|p| (p.start_line, p.start_col, p.end_col))
            .collect()
    };
    // Four bytes more at each `T1`: what follows on its line moves by them, the next line not at all.
    assert_eq!(spans(&fact.at), [(1, 1, 31), (2, 1, 24)]);
    assert_eq!(
        spans(&fact.named_at["covers"]["Longer"]),
        [(1, 11, 17), (1, 25, 31)]
    );
    assert_eq!(spans(&fact.named_at["covers"]["T5"]), [(1, 18, 20)]);
    assert_eq!(fact.fields["covers"], serde_json::json!(["Longer", "T5"]));
    assert_eq!(served.collected("covers", "Longer"), ["T1"]);
    assert_eq!(served.collected("covers", "T5"), ["T5"]);
    // The text the rename left holds; the text collected, or one edited by hand since, does not.
    let placed = &served.placed["a.test.js"];
    let left = collected.replace("T1", "Longer");
    assert!(placed.holds(left.as_bytes()));
    assert!(!placed.holds(collected.as_bytes()));
    assert!(!placed.holds(format!("{left}// by hand\n").as_bytes()));
    assert!(!placed.holds(left.replacen("Longer", "Longer2", 1).as_bytes()));
}

/// The served text of a store of one fact covering `names` at their places in `file`, collected as `collected`,
/// read through `renames` (field `covers`: held → now).
fn served_through(
    file: &str,
    collected: &str,
    names: &[(&str, Vec<FactLocation>)],
    renames: &[(&str, &str)],
) -> String {
    let mut fact = Fact::new("test", "js:a.test.js#adds");
    for (name, places) in names {
        for at in places {
            fact = fact.name("covers", name, at.clone());
        }
    }
    let fact = fact.field(
        "covers",
        serde_json::Value::Array(names.iter().map(|(n, _)| (*n).into()).collect()),
    );
    let sources = BTreeMap::from([(
        file.to_string(),
        format!(
            "sha256:{}",
            specforge_package::Sha256::of(collected.as_bytes())
        ),
    )]);
    let bytes =
        String::from_utf8(Store::new("node", "@t/node", "abc", vec![fact], sources).bytes())
            .unwrap();
    let mut by_field: Renames = Renames::new();
    for (held, now) in renames {
        by_field
            .entry("covers".to_string())
            .or_default()
            .insert(held.to_string(), now.to_string());
    }
    through(bytes, by_field)
}

/// The served text of the store whose bytes are `bytes`, the ledger renaming its names by `renames` and no other store
/// stale: what it writes in its files derived from it alone.
fn through(bytes: String, renames: Renames) -> String {
    let store: Store = serde_json::from_str(&bytes).unwrap();
    let mut rewrites = Rewrites::new();
    let mut torn = BTreeSet::new();
    stale::rewritten_by(&store, &renames, &mut rewrites, &mut torn);
    let rewrites = stale::settled(rewrites, &torn);
    let carried = stale::Carried {
        renames,
        ..Default::default()
    };
    stale::served(bytes, &carried.rewriting(&rewrites))
}

/// A store holding `C` (for no entity) and `A`, read through `A → C`: both read as `C`, so `C` is collected from
/// both, and renamed on with a line for each, the text the renames left holds.
#[test]
fn every_held_name_read_as_one_is_collected_from() {
    let collected = "// covers A C\n";
    let names = [
        ("A", vec![place("a.test.js", 1, 11, 12)]),
        ("C", vec![place("a.test.js", 1, 13, 14)]),
    ];
    let text = served_through("a.test.js", collected, &names, &[("A", "C")]);
    let served = store(KEY, &text).unwrap();

    assert_eq!(served.collected("covers", "C"), ["A", "C"]);

    let text = served_through("a.test.js", collected, &names, &[("A", "D"), ("C", "D")]);
    let served = store(KEY, &text).unwrap();
    assert!(served.placed["a.test.js"].holds(b"// covers D D\n"));
    assert_eq!(served.collected("covers", "D"), ["A", "C"]);
}

#[specforge_test_macros::test(
    behavior = "build_in_memory_graph",
    verify = "a store whose stale lines rename one name two ways serves none of its renames, W202"
)]
fn a_store_whose_ledger_renames_a_name_two_ways_serves_none_of_its_renames() {
    let collected = "// covers T1 T5\n";
    let fact = Fact::new("test", "js:a.test.js#adds")
        .name("covers", "T1", place("a.test.js", 1, 11, 13))
        .name("covers", "T5", place("a.test.js", 1, 14, 16));
    let sources = BTreeMap::from([(
        "a.test.js".to_string(),
        format!(
            "sha256:{}",
            specforge_package::Sha256::of(collected.as_bytes())
        ),
    )]);
    let bytes = Store::new("node", "@t/node", "abc", vec![fact], sources).bytes();
    let mut ledger = Ledger::default();
    ledger.insert(Line::facts("node", "abc", &bytes));
    // Two merged branches: each renamed `T1`; one also renamed `T5`.
    for (held, now) in [("T1", "Two"), ("T1", "Three"), ("T5", "T6")] {
        ledger.insert(Line::stale("node", "abc", &bytes, "covers", held, now));
    }
    let disk = disk(&bytes, Some(ledger.canonical().into_bytes()));
    let Read::Text(text) = read(&disk, &root(), KEY, None) else {
        panic!("a recorded store is served");
    };

    let served = store(KEY, &text).unwrap();

    let w202 = served.stale.as_ref().expect("W202");
    assert_eq!(w202.code, "W202");
    assert_eq!(
        w202.message,
        format!(
            "fact store '{KEY}': specforge-harness holds two renames of 'T1' ('Three', 'Two'); none of its renames \
             is served, until `specforge collect`"
        )
    );
    assert!(served.renames.is_empty());
    let fact = &served.store.facts[0];
    assert_eq!(fact.fields["covers"], serde_json::json!(["T1", "T5"]));
    assert_eq!(fact.named_at["covers"]["T5"][0].start_col, 14);
    assert!(served.placed["a.test.js"].holds(collected.as_bytes()));
}

#[test]
fn a_span_is_read_from_the_line_starts_once_found() {
    let text = b"ab\ncd\nef";
    let lines = line_starts(text);
    assert_eq!(lines, [0, 3, 6]);
    let at =
        |line: usize, from: usize, to: usize| byte_range(text, &lines, &place("f", line, from, to));
    assert_eq!(at(1, 1, 3), Some(0..2));
    assert_eq!(at(2, 2, 3), Some(4..5));
    assert_eq!(
        at(3, 1, 3),
        Some(6..8),
        "the last line ends at the text's end"
    );
    assert_eq!(at(1, 1, 4), None, "a column past its line's end");
    assert_eq!(at(4, 1, 1), None, "a line past the text");
    assert_eq!(at(0, 1, 1), None);
}

/// The byte offset of 1-based `line` and byte column `col` in `text`.
fn byte_at(text: &str, line: usize, col: usize) -> usize {
    text.split_inclusive('\n')
        .take(line - 1)
        .map(str::len)
        .sum::<usize>()
        + col
        - 1
}

#[specforge_test_macros::test(
    behavior = "build_in_memory_graph",
    verify = "a stale store's places are shifted exactly by what the renames wrote, and only the text they left holds"
)]
fn places_are_byte_exact_across_multi_byte_names_and_prefixes() {
    // `é` before the names and `Ť` in the name written are two bytes each; `T5 → T1` takes the name `T1 → Ťwo` freed.
    let collected = "x\n// é covers T1 T5 T1 end\n";
    let (t1a, t5, t1b) = (
        place("a.test.js", 2, 14, 16),
        place("a.test.js", 2, 17, 19),
        place("a.test.js", 2, 20, 22),
    );
    let mut text = served_through(
        "a.test.js",
        collected,
        &[("T1", vec![t1a, t1b]), ("T5", vec![t5])],
        &[("T1", "Ťwo"), ("T5", "T1")],
    );
    // The fact's own place, the whole comment line, rides along.
    text = text.replacen(
        "\"facts\": [\n    {",
        "\"facts\": [\n    {\n      \"at\": [{\"file\": \"a.test.js\", \"start_line\": 2, \"start_col\": 1, \"end_line\": 2, \"end_col\": 26}],",
        1,
    );
    let served = store(KEY, &text).unwrap();
    let after = "x\n// é covers Ťwo T1 Ťwo end\n";

    let fact = &served.store.facts[0];
    for (name, places) in &fact.named_at["covers"] {
        for at in places {
            let read = &after[byte_at(after, at.start_line, at.start_col)
                ..byte_at(after, at.end_line, at.end_col)];
            assert_eq!(read, name);
        }
    }
    assert_eq!(fact.fields["covers"], serde_json::json!(["Ťwo", "T1"]));
    let whole = &fact.at[0];
    assert_eq!(
        &after[byte_at(after, 2, whole.start_col)..byte_at(after, 2, whole.end_col)],
        "// é covers Ťwo T1 Ťwo end"
    );
    let placed = &served.placed["a.test.js"];
    assert!(placed.holds(after.as_bytes()));
    assert!(!placed.holds(collected.as_bytes()));
    assert!(
        !placed.holds(after.replace("end", "END").as_bytes()),
        "a hand edit of the same length"
    );
    let reverted = after.replacen("Ťwo", "T1", 1);
    assert!(
        !placed.holds(reverted.as_bytes()),
        "one place typed back by hand"
    );
}

#[test]
fn a_crlf_files_places_are_judged_as_bytes() {
    let collected = "x\r\n// covers T1 end\r\n";
    let text = served_through(
        "a.test.js",
        collected,
        &[("T1", vec![place("a.test.js", 2, 11, 13)])],
        &[("T1", "Longer")],
    );
    let served = store(KEY, &text).unwrap();

    assert!(served.placed["a.test.js"].holds(collected.replace("T1", "Longer").as_bytes()));
    // Renamed back: no rename is served, and the text collected holds.
    let back = served_through(
        "a.test.js",
        collected,
        &[("T1", vec![place("a.test.js", 2, 11, 13)])],
        &[],
    );
    assert!(store(KEY, &back).unwrap().placed["a.test.js"].holds(collected.as_bytes()));
}

/// The served store of one fact holding `names` (field, name, place) in `a.test.js` collected as `collected`, its
/// place `at`, read through `renames` (field, held, now).
fn one_fact_through(
    collected: &str,
    at: FactLocation,
    names: &[(&str, &str, FactLocation)],
    renames: &[(&str, &str, &str)],
) -> String {
    let mut fact = Fact::new("test", "js:a.test.js#adds").at(at);
    for (field, name, place) in names {
        fact = fact.name(field, name, place.clone());
    }
    let sources = BTreeMap::from([(
        "a.test.js".to_string(),
        format!(
            "sha256:{}",
            specforge_package::Sha256::of(collected.as_bytes())
        ),
    )]);
    let bytes =
        String::from_utf8(Store::new("node", "@t/node", "abc", vec![fact], sources).bytes())
            .unwrap();
    let mut by_field = Renames::new();
    for (field, held, now) in renames {
        by_field
            .entry(field.to_string())
            .or_default()
            .insert(held.to_string(), now.to_string());
    }
    through(bytes, by_field)
}

/// The text `place` covers in `text`.
fn read_at<'t>(text: &'t str, place: &FactLocation) -> &'t str {
    &text[byte_at(text, place.start_line, place.start_col)
        ..byte_at(text, place.end_line, place.end_col)]
}

#[specforge_test_macros::test(
    behavior = "build_in_memory_graph",
    verify = "names a store holds one inside another, both renamed, are served exactly where the renames left them"
)]
fn names_held_one_inside_another_are_served_exactly() {
    // `v1/T1` held in `covers`, its `T1` in `names`: one rename wrote `v1/Two` there.
    let collected = "// covers v1/T1 end\n";
    let text = one_fact_through(
        collected,
        place("a.test.js", 1, 1, 20),
        &[
            ("covers", "v1/T1", place("a.test.js", 1, 11, 16)),
            ("names", "T1", place("a.test.js", 1, 14, 16)),
        ],
        &[("covers", "v1/T1", "v1/Two"), ("names", "T1", "Two")],
    );
    let served = store(KEY, &text).unwrap();
    let after = "// covers v1/Two end\n";

    let fact = &served.store.facts[0];
    assert_eq!(
        read_at(after, &fact.named_at["covers"]["v1/Two"][0]),
        "v1/Two"
    );
    assert_eq!(read_at(after, &fact.named_at["names"]["Two"][0]), "Two");
    assert_eq!(read_at(after, &fact.at[0]), "// covers v1/Two end");
    let placed = &served.placed["a.test.js"];
    assert_eq!(
        placed.respelled.len(),
        1,
        "the outer place is what is written"
    );
    assert!(placed.holds(after.as_bytes()));
    assert!(!placed.holds(collected.as_bytes()));
    assert!(!placed.holds(b"// covers v1/Two end!\n"));
}

#[specforge_test_macros::test(
    behavior = "build_in_memory_graph",
    verify = "a name inside a name written is served where it stands while the write keeps it, and its file is not the text the places are in when the write changed it"
)]
fn a_name_inside_a_name_written_is_kept_only_where_the_write_keeps_it() {
    let collected = "// covers a/T1 x\n";
    let whole = ("covers", "a/T1", place("a.test.js", 1, 11, 15));
    let renamed = [("covers", "a/T1", "a/Longer")];
    let after = "// covers a/Longer x\n";

    // `a`, held in `scope` and renamed by nothing, is the part of `a/T1` the write keeps.
    let kept = one_fact_through(
        collected,
        place("a.test.js", 1, 1, 17),
        &[whole.clone(), ("scope", "a", place("a.test.js", 1, 11, 12))],
        &renamed,
    );
    let served = store(KEY, &kept).unwrap();
    let fact = &served.store.facts[0];
    assert_eq!(read_at(after, &fact.named_at["scope"]["a"][0]), "a");
    assert!(served.placed["a.test.js"].holds(after.as_bytes()));

    // `T1`, held in `leaf` and renamed by nothing, is what the write changed: no exact place, served as collected.
    let torn = one_fact_through(
        collected,
        place("a.test.js", 1, 1, 17),
        &[whole, ("leaf", "T1", place("a.test.js", 1, 13, 15))],
        &renamed,
    );
    let served = store(KEY, &torn).unwrap();
    let fact = &served.store.facts[0];
    assert_eq!(read_at(collected, &fact.named_at["leaf"]["T1"][0]), "T1");
    let placed = &served.placed["a.test.js"];
    assert!(!placed.holds(after.as_bytes()));
    assert!(placed.holds(collected.as_bytes()));
}
