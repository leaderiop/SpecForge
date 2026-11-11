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
use specforge_test_macros::test as specforge_test;
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

fn nested() -> TestProject {
    project(
        NESTED,
        "// covers ws/m1\n",
        fact("js:a.test.js#adds", &[("ws/m1", 11)]),
        fact("js:a.test.js#five", &[("m1", 14)]),
    )
}

fn source(p: &TestProject) -> String {
    std::fs::read_to_string(p.root().join("a.test.js")).unwrap()
}

/// `node` reads `ws/m1` whole, `unit` the `m1` at its end: both name the milestone `ws/m1`, and a rename of it writes
/// one text, the outer place, the inner reading its name inside it; a second rename composes.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename of what two facts name one inside the other writes one text, and renames of it compose"
)]
fn a_rename_of_a_name_two_facts_hold_one_inside_the_other_writes_both() {
    let p = nested();
    assert!(both_hold(&p));

    rename_now(&p, "ws/m1", "m2").unwrap();

    assert_eq!(source(&p), "// covers ws/m2\n");
    let line = |held: &str, now: &str| ("covers".to_string(), held.to_string(), now.to_string());
    assert_eq!(stale_lines(&p), [line("ws/m1", "ws/m2"), line("m1", "m2")]);
    assert!(both_hold(&p));

    rename_now(&p, "ws/m2", "m3x").unwrap();

    assert_eq!(source(&p), "// covers ws/m3x\n");
    assert_eq!(
        stale_lines(&p),
        [line("ws/m1", "ws/m3x"), line("m1", "m3x")]
    );
    assert!(both_hold(&p));
}

/// The workstream renamed: `ws/m1` writes `wz/m1`, and the `m1` nested at its end, which names the milestone that
/// moved with it, writes itself.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename of what two facts name one inside the other writes one text, and renames of it compose"
)]
fn a_rename_of_a_scope_writes_a_nested_name_its_suffix_keeps() {
    let p = nested();

    rename_now(&p, "ws", "wz").unwrap();

    assert_eq!(source(&p), "// covers wz/m1\n");
    assert!(both_hold(&p));
}

/// `node` reads `ws/m1`, naming the milestone of `ws`; `unit` the `m1` at its end, naming the milestone `m1` of no
/// workstream. A rename of that one writes inside `node`'s name, which would read `ws/m2`: refused.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename that would write over a name another fact holds for something else is refused as name_shared"
)]
fn a_rename_writing_inside_another_facts_name_is_refused() {
    let specs = format!("{NESTED}\nmilestone m1 \"Alone\" {{}}\n");
    let p = project(
        &specs,
        "// covers ws/m1\n",
        fact("js:a.test.js#adds", &[("ws/m1", 11)]),
        fact("js:a.test.js#five", &[("m1", 14)]),
    );

    let refused = rename_now(&p, "m1", "m2").unwrap_err();

    assert!(refused.starts_with("name_shared: "), "{refused}");
    assert!(refused.contains("'ws/m1'"), "{refused}");
    assert_eq!(source(&p), "// covers ws/m1\n");
    assert!(stale_lines(&p).is_empty());
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

/// A rename writing at a place another fact holds a name in a field it does not rename would change what that fact
/// reads: refused, nothing written.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename that would write over a name another fact holds for something else is refused as name_shared"
)]
fn a_rename_writing_where_another_fact_holds_a_name_for_something_else_is_refused() {
    let p = sharing_a_place();

    let refused = rename_now(&p, "T1", "T2").unwrap_err();

    // Today: a gap of spaces inside the sentence.
    assert_eq!(
        refused,
        "name_shared: cannot rename 'T1': it writes over 'T1', which the fact 'js:a.test.js#five' holds in 'note' at \
         a.test.js:1:11              and the rename does not rename; a rename writes only names of what it moves"
    );
    assert_eq!(source(&p), "// covers T1\n");
    assert!(stale_lines(&p).is_empty());
}

/// `node` reads `ws/m1`, `unit` the `m1/p3` that begins inside it: renaming the milestone writes both, and the two
/// places cross, so no one text writes them.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename of names whose places cross one another is refused as places_cross"
)]
fn a_rename_writing_places_that_cross_is_refused() {
    let specs = format!("{NESTED}\nphase p3 \"P\" {{\n  milestone m1\n}}\n");
    let p = project(
        &specs,
        "// covers ws/m1/p3\n",
        fact("js:a.test.js#adds", &[("ws/m1", 11)]),
        fact("js:a.test.js#five", &[("m1/p3", 14)]),
    );

    let refused = rename_now(&p, "ws/m1", "m2").unwrap_err();

    // Today: a gap of spaces inside the sentence.
    assert_eq!(
        refused,
        "places_cross: cannot rename 'ws/m1': two places of a name it writes in a.test.js cross one another, so no \
         one text writes              both"
    );
    assert_eq!(source(&p), "// covers ws/m1/p3\n");
}
