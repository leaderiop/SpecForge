//! The invariant kind, end to end (`spec/governance.md`): an invariant authored over the taskboard twin, its proof and
//! items read through `@code/core`'s facts, and the prompt that judges whether it holds.

use crate::harness::{Project, taskboard};
use crate::judged::TWIN_TESTS;
use crate::prompts_support::{payload, prompt, render_three_hosts, text_of};

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

/// The status of an entity, as `specforge query` answers it.
pub(super) fn status(p: &Project, id: &str) -> String {
    p.fields_of(id)["status"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn a_pending_invariants_proof_is_not_checked() {
    let p = taskboard();
    let missing = TWIN_INVARIANT.replace("#a task is added with the next id", "#no such test");
    author(&p, &missing.replace("v1.0/01", "v1.0/02"));
    collect_facts(&p);
    let phase = status(&p, "v1.0/02");
    assert!(
        !["executed", "needs_review", "complete"].contains(&phase.as_str()),
        "phase 02 is not complete: {phase}"
    );
    assert_eq!(status(&p, "ids_are_sequential"), "pending");
    assert_eq!(on(&p, "ids_are_sequential"), Vec::new());
    let phase = status(&p, "v1.0/01");
    assert!(
        ["executed", "needs_review", "complete"].contains(&phase.as_str()),
        "phase 01 is complete: {phase}"
    );
    author(&p, &missing);
    assert_eq!(status(&p, "ids_are_sequential"), "unverified");
    let found = on(&p, "ids_are_sequential");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "code/E101", "{found:#?}");
}

/// The twin's invariant in force and a second one pending until phase 02 completes.
const WITH_A_PENDING_ONE: &str = r#"invariant ids_stay_unique "Task ids stay unique" {
  statement  "No two tasks share an id"
  holds_from v1.0/02
}
"#;

/// `verify_invariant` is our own text, rendered for each host with no shell-ids line and no attribution; its tail
/// names the invariant given, else every invariant in force that does not hold, else says none awaits a verdict.
#[test]
fn the_verify_invariant_prompt_lists_the_invariants_awaiting_a_verdict() {
    let p = taskboard();
    author(&p, &format!("{TWIN_INVARIANT}\n{WITH_A_PENDING_ONE}"));
    assert_eq!(status(&p, "ids_stay_unique"), "pending");
    let heading = "## The invariants to judge";
    for (host, rendered) in
        render_three_hosts(&p, "verify_invariant", &[], "(no arguments)", heading)
    {
        let text = text_of(&rendered);
        for word in [
            "specforge gsd record-verdict",
            "--inputs gsd/I536",
            "- `ids_are_sequential`",
        ] {
            assert!(text.contains(word), "{host}: no '{word}'");
        }
        for word in ["`ids_stay_unique`", "In the shell blocks above"] {
            assert!(!text.contains(word), "{host}: '{word}'");
        }
        assert!(
            rendered["payload"]
                .get("attribution")
                .is_none_or(|a| a.is_null()),
            "{host}: {}",
            rendered["payload"]
        );
    }
    let given = ["--invariant", "ids_stay_unique"];
    let text = text_of(&payload(&prompt(
        &p,
        "verify_invariant",
        &given,
        "claude-code",
    )));
    assert!(text.contains("- `ids_stay_unique`"), "{text}");
    assert!(!text.contains("`ids_are_sequential`"), "{text}");
    let plain = taskboard();
    let text = text_of(&payload(&prompt(
        &plain,
        "verify_invariant",
        &[],
        "claude-code",
    )));
    assert!(text.contains("No invariant awaits a verdict."), "{text}");
    assert!(!text.contains(heading), "{text}");
}
