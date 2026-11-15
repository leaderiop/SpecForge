//! The invariant kind, end to end (`spec/governance.md`): an invariant authored over the taskboard twin, its proof and
//! items read through `@code/core`'s facts.

use crate::harness::{Project, taskboard};
use crate::judged::TWIN_TESTS;

/// One invariant over the taskboard twin, in force from phase 01 (executed): what `addTask` keeps, the test that
/// proves it and the code item it holds over.
pub(super) const TWIN_INVARIANT: &str = r#"invariant ids_are_sequential "Task ids are sequential" {
  statement   "A new task takes the next id"
  holds_from  v1.0/01
  proof       ["js:taskboard/test/tasks.test.js#a task is added with the next id"]
  items       ["js:taskboard/src/tasks.js::addTask"]
  criticality high
}
"#;

/// Writes `text` as the project's `spec/invariants.spec`.
pub(super) fn author(p: &Project, text: &str) {
    p.write("spec/invariants.spec", text);
}

/// Collects the twin's three tests and its source items, so a proof or an item reference resolves or is code/E101.
pub(super) fn collect_facts(p: &Project) {
    for run in [
        p.node_tests(&TWIN_TESTS),
        p.collect(&["--runner", "javascript-source"]),
    ] {
        assert_eq!(run.code, 0, "{}", run.all());
    }
}

/// `(code, message)` of every diagnostic `specforge check` reports on the entity `id`, in report order.
pub(super) fn on(p: &Project, id: &str) -> Vec<(String, String)> {
    let run = p.run(&["check", "--format", "json"]);
    let found: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
    found
        .as_array()
        .expect("a list")
        .iter()
        .filter(|d| d["data"]["entity"] == id)
        .map(|d| {
            let text = |key: &str| d[key].as_str().unwrap_or_default().to_string();
            (text("code"), text("message"))
        })
        .collect()
}

#[test]
fn an_invariant_is_authored_with_its_proof_and_items() {
    let p = taskboard();
    author(&p, TWIN_INVARIANT);
    p.check_exact(&[]);
    collect_facts(&p);
    assert_eq!(on(&p, "ids_are_sequential"), Vec::new());
    let fields = p.fields_of("ids_are_sequential");
    assert_eq!(
        fields["statement"], "A new task takes the next id",
        "{fields}"
    );
    assert_eq!(fields["holds_from"], "v1.0/01", "{fields}");
    assert_eq!(
        fields["proof"],
        serde_json::json!(["js:taskboard/test/tasks.test.js#a task is added with the next id"]),
        "{fields}"
    );
    assert_eq!(
        fields["items"],
        serde_json::json!(["js:taskboard/src/tasks.js::addTask"]),
        "{fields}"
    );
    assert_eq!(fields["criticality"], "high", "{fields}");
}

#[test]
fn an_in_force_invariants_missing_test_is_code_e101() {
    let p = taskboard();
    author(
        &p,
        &TWIN_INVARIANT.replace("#a task is added with the next id", "#no such test"),
    );
    collect_facts(&p);
    let found = on(&p, "ids_are_sequential");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "code/E101", "{found:#?}");
    assert!(found[0].1.contains("#no such test"), "{found:#?}");
}

