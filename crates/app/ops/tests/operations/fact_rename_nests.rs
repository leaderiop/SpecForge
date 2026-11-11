//! Renames where facts hold names one inside another, or at a place another fact holds a name for something else
//! (ADR 0346 D7): `ws/m1` read whole by `node`'s collector and its `m1` by `unit`'s, and a name `unit` holds in a field
//! no rename renames at the place `node`'s name is written.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta};
use specforge_ops::authoring::apply::Origin;
use specforge_ops::authoring::rename;
use specforge_ops::read::navigate::Navigator;
use specforge_ops::testing::caller;
use specforge_ops::view::ProjectView;
use specforge_project::CompiledProject;
use specforge_project::testing::{
    COLLECTED_EXTENSIONS, Collected, SCOPED_EXTENSION, TestProject, scoped_kinds, stored,
};
use specforge_wasm::testing::InProcessRuntime;

use crate::collected::{KEY, stale_lines};

const NODE_STORE: &str = ".specforge/facts/0123456789abcdef0123456789abcdef01234567/node.json";
const UNIT_STORE: &str = ".specforge/facts/0123456789abcdef0123456789abcdef01234567/unit.json";

fn scoped() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(SCOPED_EXTENSION, "1.0.0"));
    scoped_kinds(&mut c);
    c
}

fn compiled(p: &TestProject) -> CompiledProject {
    p.compile_in(Arc::new(
        Collected::new().over(InProcessRuntime::new().with(scoped)),
    ))
}

/// A project declaring `specs` beside the truth kind and the GSD-shaped kinds, `a.test.js` reading `source`, and
/// `node`'s and `unit`'s stores holding the facts given.
fn project(specs: &str, source: &str, node: Value, unit: Value) -> TestProject {
    let mut enabled = COLLECTED_EXTENSIONS.to_vec();
    enabled.push(SCOPED_EXTENSION);
    let p = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&enabled)
        .installing_enabled()
        .file("main.spec", specs);
    std::fs::write(p.root().join("a.test.js"), source).unwrap();
    stored(p.root(), "node", KEY, node);
    stored(p.root(), "unit", KEY, unit);
    p
}

/// One fact `id` covering `names`, each written at its column of line 1 of `a.test.js`.
fn fact(id: &str, names: &[(&str, usize)]) -> Value {
    let covers: Vec<&str> = names.iter().map(|(name, _)| *name).collect();
    let mut named = serde_json::Map::new();
    for (name, from) in names {
        named.insert(
            name.to_string(),
            json!([{"file": "a.test.js", "start_line": 1, "start_col": from, "end_line": 1, "end_col": from + name.len()}]),
        );
    }
    json!([{"kind": "test", "id": id, "fields": {"file": "a.test.js", "covers": covers},
            "named_at": {"covers": named}}])
}

fn rename_now(p: &TestProject, from: &str, to: &str) -> Result<(), String> {
    let compiled = compiled(p);
    rename::run(
        &Navigator::new(ProjectView::of(&compiled)),
        from,
        to,
        false,
        &Origin::cli(true),
        caller(),
    )
    .map(drop)
    .map_err(|refused| format!("{}: {}", refused.error.code, refused.error.message))
}

/// Whether both stores serve their places in `a.test.js` as it is on disk.
fn both_hold(p: &TestProject) -> bool {
    let compiled = compiled(p);
    let text = std::fs::read(p.root().join("a.test.js")).unwrap();
    let sources = compiled.fact_sources();
    let placed = &sources["a.test.js"];
    [NODE_STORE, UNIT_STORE]
        .iter()
        .all(|store| placed.get(store).is_some_and(|placed| placed.holds(&text)))
}

const NESTED: &str = "workstream ws \"W\" {}\n\nmilestone m1 \"M\" {\n  workstream ws\n}\n";

/// `node` reads `ws/m1` whole, `unit` the `m1` at its end: both name the milestone `ws/m1`, and a rename of it writes
/// both.
#[test]
fn a_rename_of_a_name_two_facts_hold_one_inside_the_other_is_source_moved() {
    let p = project(
        NESTED,
        "// covers ws/m1\n",
        fact("js:a.test.js#adds", &[("ws/m1", 11)]),
        fact("js:a.test.js#five", &[("m1", 14)]),
    );
    assert!(both_hold(&p));

    let refused = rename_now(&p, "ws/m1", "m2").unwrap_err();

    assert!(refused.starts_with("source_moved: "), "{refused}");
}

const TWO: &str = "truth T1 \"One\" {}\n";

/// `node` covers `T1` at columns 11..13; `unit` holds the same `T1` in `note`, a field no rename renames.
fn sharing_a_place() -> TestProject {
    let place = json!([{"file": "a.test.js", "start_line": 1, "start_col": 11, "end_line": 1, "end_col": 13}]);
    project(
        TWO,
        "// covers T1\n",
        fact("js:a.test.js#adds", &[("T1", 11)]),
        json!([{"kind": "test", "id": "js:a.test.js#five", "fields": {"file": "a.test.js", "note": "T1"},
                "named_at": {"note": {"T1": place}}}]),
    )
}

/// A rename writing at a place another fact holds a name in a field it does not rename changes what that fact reads.
#[test]
fn a_rename_writing_where_another_fact_holds_a_name_for_something_else_applies() {
    let p = sharing_a_place();

    rename_now(&p, "T1", "T2").unwrap();

    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        "// covers T2\n"
    );
    assert_eq!(
        stale_lines(&p),
        [("covers".into(), "T1".into(), "T2".into())]
    );
}
