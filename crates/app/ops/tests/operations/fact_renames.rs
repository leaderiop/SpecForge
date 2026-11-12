//! A rename of an entity collected facts name, against what the stores hold beside it (ADR 0346 D7): a name the
//! store holds that names no entity, a name one fact holds for the entity and another for something else, two
//! renames of one name the ledger was merged with, and a second store placed in a file a rename rewrote.

use serde_json::json;
use specforge_project::testing::{TestProject, stored_with};
use specforge_test_macros::test as specforge_test;

use crate::collected::{FACT, KEY, covered_and_placed, project, rename_now, stale_lines};

/// [`project`] whose fact covers `T1` and `T9`, which names no entity, both written on line 1 of `a.test.js`.
fn beside_a_dangling_name() -> TestProject {
    let p = project();
    let source = "// covers T1 T9\n";
    std::fs::write(p.root().join("a.test.js"), source).unwrap();
    let at = |from: usize| json!([{"file": "a.test.js", "start_line": 1, "start_col": from, "end_line": 1, "end_col": from + 2}]);
    stored_with(
        p.root(),
        "node",
        KEY,
        json!([{"kind": "test", "id": FACT, "fields": {"file": "a.test.js", "covers": ["T1", "T9"]},
                "named_at": {"covers": {"T1": at(11), "T9": at(14)}}}]),
        [(
            "a.test.js".to_string(),
            specforge_ops::revision::of_bytes(source.as_bytes()),
        )]
        .into(),
    );
    p
}

/// A rename to the name a fact holds and nothing carries is a capture: the store would hold it twice, once for the
/// renamed entity and once for nothing, and the next rename could not tell them apart.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename to a name a fact holds for no entity is refused as name_captured"
)]
fn a_rename_to_a_name_a_fact_holds_for_no_entity_is_captured() {
    let p = beside_a_dangling_name();

    let refused = rename_now(&p, "T1", "T9").unwrap_err();

    assert!(refused.starts_with("name_captured: "), "{refused}");
    assert!(refused.contains("'T9'"), "{refused}");
    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        "// covers T1 T9\n"
    );
    assert!(stale_lines(&p).is_empty());
}

/// A ledger whose renames already made two held names read as one (written before a capture was refused, or merged):
/// a rename of that name writes a stale line for each, so the serve shifts and checks every place it wrote.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename of a name two held names are read as writes a stale line for each"
)]
fn a_rename_of_a_name_two_held_names_read_as_writes_a_line_for_each() {
    let p = beside_a_dangling_name();
    p.write(
        "truths.spec",
        format!("truth T9 \"One\" {{\n  proof [\"{FACT}\"]\n}}\n"),
    );
    std::fs::write(p.root().join("a.test.js"), "// covers T9 T9\n").unwrap();
    let store = std::fs::read(p.root().join(crate::collected::STORE)).unwrap();
    let ledger = p.root().join("specforge-harness");
    let mut lines =
        specforge_project::harness::Ledger::parse(&std::fs::read_to_string(&ledger).unwrap())
            .ledger;
    lines.insert(specforge_project::harness::Line::stale(
        "node", KEY, &store, "covers", "T1", "T9",
    ));
    lines.insert(specforge_project::harness::Line::moved("T9", "T1"));
    std::fs::write(&ledger, lines.canonical()).unwrap();

    rename_now(&p, "T9", "T7").unwrap();

    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        "// covers T7 T7\n"
    );
    assert_eq!(
        stale_lines(&p),
        [
            ("covers".into(), "T1".into(), "T7".into()),
            ("covers".into(), "T9".into(), "T7".into())
        ]
    );
    assert_eq!(covered_and_placed(&p), (vec!["T7".to_string()], true));
}

/// The extension declaring the collected kind `note`, whose `covers` is text, not a reference.
const NOTES: &str = "@t/notes";

fn notes() -> specforge_extension_sdk::prelude::ContributionsBuilder {
    use specforge_extension_sdk::DeclaredCategory;
    use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta};
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(NOTES, "1.0.0"));
    c.raw_category(
        DeclaredCategory::Entities,
        json!([{"name": "note", "keyword": "note", "writer": "collector", "schemes": ["note"], "fields": [
            {"name": "covers", "field_type": "string"},
        ]}]),
    );
    c
}

/// [`project`] with [`NOTES`] loaded, whose store holds `T1` in `covers` twice in `a.test.js`: the test `FACT`'s
/// reference on line 1, and a note's text on line 3.
fn shared() -> (
    TestProject,
    std::sync::Arc<specforge_wasm::testing::InProcessRuntime>,
) {
    use specforge_project::testing::{CODE, Collected, NODE, SPEC};
    let p = project()
        .enabling(&[CODE, NODE, SPEC, NOTES])
        .installing_enabled();
    let source = "// covers T1\ntest('adds', () => {});\n// covers T1\n";
    std::fs::write(p.root().join("a.test.js"), source).unwrap();
    let at = |line: usize| json!([{"file": "a.test.js", "start_line": line, "start_col": 11, "end_line": line, "end_col": 13}]);
    stored_with(
        p.root(),
        "node",
        KEY,
        json!([{"kind": "test", "id": FACT, "fields": {"file": "a.test.js", "covers": ["T1"]},
                "named_at": {"covers": {"T1": at(1)}}},
               {"kind": "note", "id": "note:a.test.js#1", "fields": {"covers": "T1"},
                "named_at": {"covers": {"T1": at(3)}}}]),
        [(
            "a.test.js".to_string(),
            specforge_ops::revision::of_bytes(source.as_bytes()),
        )]
        .into(),
    );
    let runtime = std::sync::Arc::new(
        Collected::new().over(specforge_wasm::testing::InProcessRuntime::new().with(notes)),
    );
    (p, runtime)
}

/// A store renames a name in every fact that holds it: when another fact holds the same name in the same field for
/// something else (here no reference at all), the rename cannot rewrite only the renamed entity's places, and is
/// refused.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename of a name a fact of the same store holds for something else is refused as name_shared"
)]
fn a_rename_of_a_name_another_fact_holds_for_something_else_is_refused() {
    use specforge_ops::authoring::apply::Origin;
    use specforge_ops::authoring::rename;
    use specforge_ops::read::navigate::Navigator;
    use specforge_ops::view::ProjectView;
    let (p, runtime) = shared();
    let compiled = p.compile_in(runtime);
    assert!(
        compiled.graph().node("note:a.test.js#1").is_some(),
        "{:?}",
        compiled.diagnostics()
    );

    let refused = rename::run(
        &Navigator::new(ProjectView::of(&compiled)),
        "T1",
        "T2",
        false,
        &Origin::cli(true),
        specforge_ops::testing::caller(),
    )
    .unwrap_err();

    assert_eq!(
        refused.error.code,
        rename::NAME_SHARED,
        "{}",
        refused.error.message
    );
    assert_eq!(
        refused.error.message,
        "cannot rename 'T1': the fact 'note:a.test.js#1' holds 'T1' in 'covers' too, where it names nothing; \
         a fact store's name is renamed in every fact that holds it"
    );
    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        "// covers T1\ntest('adds', () => {});\n// covers T1\n"
    );
}

/// Two branches each renamed `T1` (to `T2`, to `T3`), and their ledgers merged: the store serves none of its renames,
/// W202 says why, and a rename of a name it holds is refused until the next collect.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename of a name a store holds while the ledger renames one of its names two ways is refused as stale_conflict"
)]
fn a_store_whose_ledger_renames_a_name_two_ways_serves_none_and_refuses_renames() {
    use specforge_project::harness::{Ledger, Line};
    let p = crate::collected::two_named();
    let store = std::fs::read(p.root().join(crate::collected::STORE)).unwrap();
    let ledger = p.root().join("specforge-harness");
    let mut lines = Ledger::parse(&std::fs::read_to_string(&ledger).unwrap()).ledger;
    for now in ["T2", "T3"] {
        lines.insert(Line::stale("node", KEY, &store, "covers", "T1", now));
    }
    std::fs::write(&ledger, lines.canonical()).unwrap();

    let compiled = crate::collected::compiled_quietly(&p);
    let diagnostics = compiled.diagnostics();
    let w202: Vec<&str> = diagnostics
        .iter()
        .filter(|d| d.code == "W202")
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(
        w202,
        [format!(
            "fact store '{}': specforge-harness holds two renames of 'T1' ('T2', 'T3'); none of its renames is \
             served, until `specforge collect`",
            crate::collected::STORE
        )]
    );
    assert!(diagnostics.iter().all(|d| d.code != "W197"));
    // Served as collected: the fact still covers `T1`.
    let covered: Vec<String> = compiled
        .graph()
        .edges_from(FACT)
        .iter()
        .map(|e| e.target.to_string())
        .collect();
    assert!(covered.iter().any(|c| c == "T1"), "{covered:?}");

    let refused = rename_now(&p, "T5", "T6").unwrap_err();

    assert_eq!(
        refused,
        format!(
            "stale_conflict: cannot rename 'T5': specforge-harness holds two renames of 'T1' ('T2', 'T3') in the \
             fact store '{}'",
            crate::collected::STORE
        )
    );
    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        "// covers T1 T5\n"
    );
}

/// `unit`'s store, filed under [`KEY`] beside `node`'s.
const UNIT_STORE: &str = ".specforge/facts/0123456789abcdef0123456789abcdef01234567/unit.json";

/// [`project`] with a second truth `T5`, and two collectors' stores placed in one line of `a.test.js`: `node`'s fact
/// covers `T1` (columns 11..13), `unit`'s covers `T5` (14..16).
fn two_stores() -> TestProject {
    let p = project();
    p.write(
        "truths.spec",
        format!("truth T1 \"One\" {{\n  proof [\"{FACT}\"]\n}}\n\ntruth T5 \"Five\" {{}}\n"),
    );
    let source = "// covers T1 T5\n";
    std::fs::write(p.root().join("a.test.js"), source).unwrap();
    let at = |from: usize| json!([{"file": "a.test.js", "start_line": 1, "start_col": from, "end_line": 1, "end_col": from + 2}]);
    let sources: std::collections::BTreeMap<String, String> = [(
        "a.test.js".to_string(),
        specforge_ops::revision::of_bytes(source.as_bytes()),
    )]
    .into();
    stored_with(
        p.root(),
        "node",
        KEY,
        json!([{"kind": "test", "id": FACT, "fields": {"file": "a.test.js", "covers": ["T1"]},
                "named_at": {"covers": {"T1": at(11)}}}]),
        sources.clone(),
    );
    stored_with(
        p.root(),
        "unit",
        KEY,
        json!([{"kind": "test", "id": "js:a.test.js#five", "fields": {"file": "a.test.js", "covers": ["T5"]},
                "at": [{"file": "a.test.js", "start_line": 1, "start_col": 1, "end_line": 1, "end_col": 17}],
                "named_at": {"covers": {"T5": at(14)}}}]),
        sources,
    );
    p
}

/// Whether every store placed in `a.test.js` serves its places in the file as it is on disk, and where `unit`'s
/// fact's name `name` is served.
fn every_store_holds(p: &TestProject, name: &str) -> (bool, Option<(usize, usize)>) {
    let compiled = crate::collected::compiled_quietly(p);
    let text = std::fs::read(p.root().join("a.test.js")).unwrap();
    let sources = compiled.fact_sources();
    let placed = &sources["a.test.js"];
    assert!(
        placed.contains_key(UNIT_STORE),
        "both stores are placed in it"
    );
    let holds = placed.values().all(|placed| placed.holds(&text));
    let fact = compiled.graph().node("js:a.test.js#five").unwrap();
    let at = match fact.fields.entry("covers").map(|e| &e.value) {
        Some(specforge_parser::FieldValue::List(items)) => items
            .iter()
            .find(|i| i.text == name)
            .map(|i| (i.span.start_col, i.span.end_col)),
        _ => None,
    };
    (holds, at)
}

/// A store placed in a file a rename rewrote, holding none of the renamed names, is served shifted by what the rename
/// wrote there: its places are exact in the text the rename left, and a rename of its own name applies.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "every store placed in a file a rename rewrote is served shifted by it, and a rename of its own name applies"
)]
fn a_second_store_in_a_file_a_rename_rewrote_is_served_shifted() {
    let p = two_stores();
    rename_now(&p, "T1", "Longer").unwrap();

    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        "// covers Longer T5\n"
    );
    // `unit`'s `T5` is served where it now stands, four bytes on, and its store holds the file.
    assert_eq!(every_store_holds(&p, "T5"), (true, Some((18, 20))));

    rename_now(&p, "T5", "T6").unwrap();

    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        "// covers Longer T6\n"
    );
    assert_eq!(every_store_holds(&p, "T6"), (true, Some((18, 20))));
    let lines = stale_lines(&p);
    assert_eq!(
        lines,
        [
            ("covers".into(), "T1".into(), "Longer".into()),
            ("covers".into(), "T5".into(), "T6".into())
        ]
    );
    let ledger = std::fs::read_to_string(p.root().join("specforge-harness")).unwrap();
    assert!(ledger.contains("unit stale:"), "{ledger}");
}
