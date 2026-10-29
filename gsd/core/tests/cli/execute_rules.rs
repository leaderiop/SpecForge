//! The rules of execution, end to end (`spec/rules.md`): `gsd/E401`, `E402`, `E403` and `E405` on the `execute`
//! project. Each case edits a fresh copy and asserts the whole list of codes.

use serde_json::json;

use crate::harness::{Project, project};

const SUMMARY: &str = "v1.0/01/01-01/01-01-SUMMARY";
const T0: &str = "2026-10-29T09:00:00Z";
const T1: &str = "2026-10-29T09:30:00Z";
const REQUIREMENTS: &str = "requirements_completed [TASK-01]";
const FILES: &str = "files_modified [\"src/tasks.js\", \"test/tasks.test.js\"]";
const STATUS: &str = "status                 complete";

fn repository() -> Project {
    let p = project("execute");
    p.commit();
    p
}

/// `(code, message)` of the report, as the diagnostics name them.
fn report(p: &Project) -> Vec<(String, String)> {
    p.report()
}

fn codes(p: &Project) -> Vec<String> {
    p.codes()
}

#[test]
fn each_execute_code_fires_on_its_fixture() {
    // E401: the summary lists a requirement its plan does not name, and misses the one it does.
    let p = project("execute");
    p.edit(
        "spec/plans.spec",
        REQUIREMENTS,
        "requirements_completed [TASK-02]",
    );
    let found = report(&p);
    assert_eq!(codes(&p), ["gsd/E401"], "{found:?}");
    assert!(
        found[0].1.contains(SUMMARY)
            && found[0].1.contains("missing TASK-01")
            && found[0].1.contains("not in the plan: TASK-02"),
        "{found:?}"
    );
    // The same set in another order is clean.
    let p = project("execute");
    p.edit(
        "spec/plans.spec",
        REQUIREMENTS,
        "requirements_completed [TASK-01]",
    );
    assert_eq!(codes(&p), Vec::<String>::new());

    // E402: halted says nothing why; with a reason it is clean.
    let p = project("execute");
    p.edit("spec/plans.spec", STATUS, "status                 halted");
    assert_eq!(codes(&p), ["gsd/E402"]);
    p.edit(
        "spec/plans.spec",
        "status                 halted",
        "status                 halted\n  halted_because         \"the store design was wrong\"",
    );
    assert_eq!(codes(&p), Vec::<String>::new());
}

fn tdd(p: &Project) {
    p.edit(
        "spec/plans.spec",
        "task T1 \"Store module\" {\n  plan   v1.0/01/01-01\n  type   auto",
        "task T1 \"Store module\" {\n  plan   v1.0/01/01-01\n  type   tdd",
    );
}

#[test]
fn e403_needs_a_red_commit_first_and_tdd_mode_on() {
    let p = repository();
    tdd(&p);
    p.settings(json!({ "workflow.tdd_mode": true }));
    assert_eq!(
        codes(&p),
        Vec::<String>::new(),
        "a tdd task nobody committed"
    );
    p.git_commit("feat(01-01): Store module", &[("src/tasks.js", "a\n")], T0);
    // With the gate on the host refuses to record the GREEN commit (the edit would add the error): record it first.
    p.settings(json!({ "workflow.tdd_mode": true }));
    let refused = p.collect(&["--runner", "gsd-git"]);
    assert_ne!(refused.code, 0, "{}", refused.all());
    assert!(refused.all().contains("gsd/E403"), "{}", refused.all());
    p.settings(json!({ "workflow.tdd_mode": false }));
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(
        codes(&p),
        Vec::<String>::new(),
        "the setting off: GSD does not enforce it"
    );
    p.settings(json!({ "workflow.tdd_mode": true }));
    let found = report(&p);
    assert_eq!(codes(&p), ["gsd/E403"], "{found:?}");
    assert!(
        found[0].1.contains("v1.0/01/01-01/T1") && found[0].1.contains("GREEN commit"),
        "{found:?}"
    );
}

#[test]
fn e403_is_clean_with_a_test_commit_first() {
    let p = repository();
    p.settings(json!({ "workflow.tdd_mode": true }));
    tdd(&p);
    p.git_commit(
        "test(01-01): Store module",
        &[("test/tasks.test.js", "t\n")],
        T0,
    );
    p.git_commit("feat(01-01): Store module", &[("src/tasks.js", "a\n")], T1);
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(codes(&p), Vec::<String>::new());
}

fn touch_schema(p: &Project) {
    p.edit(
        "spec/plans.spec",
        FILES,
        "files_modified [\"src/tasks.js\", \"test/tasks.test.js\", \"prisma/schema.prisma\"]",
    );
}

#[test]
fn e405_names_the_orm_and_its_push_command() {
    let p = project("execute");
    touch_schema(&p);
    let found = report(&p);
    assert_eq!(codes(&p), ["gsd/E405"], "{found:?}");
    assert!(
        found[0].1.contains("v1.0/01")
            && found[0].1.contains("prisma/schema.prisma")
            && found[0].1.contains("prisma"),
        "{found:?}"
    );
    let explained = p.run(&["check", "--format", "json"]).stdout;
    assert!(
        explained.contains("npx prisma db push --accept-data-loss"),
        "{explained}"
    );
}

#[test]
fn e405_is_clean_with_push_evidence_with_the_gate_off_and_before_execution() {
    let p = project("execute");
    touch_schema(&p);
    // A summary that mentions the push.
    p.edit(
        "spec/plans.spec",
        "one_liner              \"An in-memory task store with add and list\"",
        "one_liner              \"An in-memory task store; ran prisma migrate deploy\"",
    );
    assert_eq!(codes(&p), Vec::<String>::new());
    // The gate off.
    let p = project("execute");
    touch_schema(&p);
    p.settings(json!({ "workflow.schema_drift_gate": false }));
    assert_eq!(codes(&p), Vec::<String>::new());
    // A phase that has not run: plan 02-01 has no summary and no commit.
    let p = project("execute");
    p.edit(
        "spec/plans.spec",
        "files_modified [\"src/store.js\"]",
        "files_modified [\"src/store.js\", \"prisma/schema.prisma\"]",
    );
    assert_eq!(codes(&p), Vec::<String>::new());
}

#[test]
fn e405_is_an_error_under_every_profile() {
    for profile in ["gsd-compat", "gsd-strict", "gsd-plus"] {
        let p = project("execute");
        p.settings(json!({ "profile": profile }));
        touch_schema(&p);
        let run = p.run(&["check", "--format", "json"]);
        let found: serde_json::Value = serde_json::from_str(&run.stdout).expect("JSON");
        let severities: Vec<String> = found
            .as_array()
            .expect("a list")
            .iter()
            .filter(|d| d["code"] == "gsd/E405")
            .map(|d| d["severity"].as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(severities, ["Error"], "{profile}: {}", run.stdout);
    }
}
