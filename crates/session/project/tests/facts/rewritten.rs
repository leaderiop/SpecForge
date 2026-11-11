//! What renames wrote in a source file, derived across every stale store (ADR 0346 D7): a store placed in the file,
//! holding none of the renamed names, is served shifted by it; and since what it serves depends on the other stores'
//! bytes, a session reading one store again reads them all again.

use std::fs;
use std::sync::Arc;

use serde_json::json;
use specforge_project::ProjectSession;
use specforge_project::harness::{Ledger, Line};
use specforge_project::testing::{Collected, TestProject, revision_of, stored_with};
use specforge_test::prelude::*;

use super::{KEY, collected};

const NODE: &str = ".specforge/facts/0123456789abcdef0123456789abcdef01234567/node.json";
const UNIT: &str = ".specforge/facts/0123456789abcdef0123456789abcdef01234567/unit.json";

/// `node`'s fact covers `T1` and `unit`'s covers `T5`, both on line 1 of `a.test.js` as collected; the ledger marks
/// `node`'s store stale, `T1` renamed to `Longer`, and the file reads as that rename left it.
fn renamed() -> TestProject {
    let p = collected();
    let collected = "// covers T1 T5\n";
    let at = |from: usize| json!([{"file": "a.test.js", "start_line": 1, "start_col": from, "end_line": 1, "end_col": from + 2}]);
    let sources: std::collections::BTreeMap<String, String> =
        [("a.test.js".to_string(), revision_of(collected.as_bytes()))].into();
    stored_with(
        p.root(),
        "node",
        KEY,
        json!([{"kind": "test", "id": "js:a.test.js#adds", "fields": {"covers": ["T1"]},
                "named_at": {"covers": {"T1": at(11)}}}]),
        sources.clone(),
    );
    stored_with(
        p.root(),
        "unit",
        KEY,
        json!([{"kind": "test", "id": "js:a.test.js#five", "fields": {"covers": ["T5"]},
                "named_at": {"covers": {"T5": at(14)}}}]),
        sources,
    );
    let node = fs::read(p.root().join(NODE)).unwrap();
    let path = p.root().join("specforge-harness");
    let mut ledger = Ledger::parse(&fs::read_to_string(&path).unwrap()).ledger;
    ledger.insert(Line::stale("node", KEY, &node, "covers", "T1", "Longer"));
    fs::write(&path, ledger.canonical()).unwrap();
    fs::write(p.root().join("a.test.js"), "// covers Longer T5\n").unwrap();
    p
}

/// Where `unit`'s `T5` is served, and whether `unit`'s store holds `a.test.js` as it is on disk.
fn unit_placed(session: &ProjectSession, p: &TestProject) -> (usize, bool) {
    let project = session.project();
    let fact = project.graph().node("js:a.test.js#five").unwrap();
    let Some(specforge_parser::FieldValue::List(items)) =
        fact.fields.entry("covers").map(|e| &e.value)
    else {
        panic!("{:?}", fact.fields);
    };
    let text = fs::read(p.root().join("a.test.js")).unwrap();
    (
        items[0].span.start_col,
        project.fact_sources()["a.test.js"][UNIT].holds(&text),
    )
}

#[specforge_test(
    behavior = "build_in_memory_graph",
    verify = "a store placed in a file another store's renames rewrote is served shifted by them, and read again with it"
)]
fn a_store_in_a_file_another_stores_rename_rewrote_is_served_shifted_and_read_again_with_it() {
    let p = renamed();
    let mut session = p.session_in(Arc::new(Collected::new().runtime()));

    assert_eq!(unit_placed(&session, &p), (18, true));

    // `node`'s store written again with other bytes, no ledger line for them: its stale line marks nothing, so the
    // file holds no rename any store derives, and `unit`, its own file unchanged, is read again with it.
    let node = fs::read_to_string(p.root().join(NODE)).unwrap();
    fs::write(p.root().join(NODE), format!("{node} ")).unwrap();
    let root = fs::canonicalize(p.root()).unwrap();
    let changes = session.changes([root.join(NODE).as_path()]);
    assert_eq!(changes.sources(), [NODE]);
    assert!(session.apply(&changes).is_some());

    assert_eq!(unit_placed(&session, &p), (14, false));
}
