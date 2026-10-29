//! The rules of execution, end to end (`spec/rules.md`): `gsd/E401`, `E402`, `W403` and `E405` on the `execute`
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
fn w403_needs_a_red_commit_first_and_tdd_mode_on() {
    let p = repository();
    tdd(&p);
    p.settings(json!({ "workflow.tdd_mode": true }));
    assert_eq!(
        codes(&p),
        Vec::<String>::new(),
        "a tdd task nobody committed"
    );
    p.git_commit("feat(01-01): Store module", &[("src/tasks.js", "a\n")], T0);
    // A warning cannot make the host refuse the collector's write: the GREEN commit is recorded.
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    junit(&p, &[("adds a task", true)]);
    let found = report(&p);
    assert_eq!(codes(&p), ["gsd/W403"], "{found:?}");
    assert!(
        found[0].1.contains("v1.0/01/01-01/T1") && found[0].1.contains("GREEN commit"),
        "{found:?}"
    );
    p.settings(json!({ "workflow.tdd_mode": false }));
    assert_eq!(
        codes(&p),
        Vec::<String>::new(),
        "the setting off: GSD does not enforce it"
    );
}

#[test]
fn a_green_without_red_does_not_stop_the_other_commits() {
    let p = repository();
    tdd(&p);
    p.settings(json!({ "workflow.tdd_mode": true }));
    let first = p.git_commit("feat(01-02): Complete a task", &[("src/c.js", "c\n")], T0);
    let green = p.git_commit("feat(01-01): Store module", &[("src/tasks.js", "a\n")], T1);
    let last = p.git_commit("feat(02-01): Write the file", &[("src/w.js", "w\n")], T1);
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let mut all = [&first[..12], &green[..12], &last[..12]];
    all.sort_unstable();
    assert_eq!(
        p.nodes_of("commit")
            .iter()
            .filter_map(|n| n["id"].as_str().and_then(|i| i.rsplit('/').next()))
            .map(String::from)
            .collect::<std::collections::BTreeSet<_>>(),
        all.iter().map(|h| h.to_string()).collect()
    );
    junit(&p, &[("adds a task", true)]);
    assert_eq!(codes(&p), ["gsd/W403"], "{:?}", report(&p));
}

#[test]
fn a_plan_only_commit_for_a_sibling_task_is_not_a_green() {
    let p = repository();
    tdd(&p);
    p.settings(json!({ "workflow.tdd_mode": true }));
    // GSD words a commit with the plan, not the task: it is T2's as likely as T1's.
    p.git_commit(
        "feat(01-01): scaffold the store",
        &[("src/tasks.js", "a\n")],
        T0,
    );
    assert_eq!(p.collect(&["--runner", "gsd-git"]).code, 0);
    junit(&p, &[("adds a task", true)]);
    assert_eq!(codes(&p), Vec::<String>::new(), "{:?}", report(&p));
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
    junit(&p, &[("adds a task", true)]);
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
    // A phase that has not run: plan 02-01 has no summary.
    let p = project("execute");
    p.edit(
        "spec/plans.spec",
        "files_modified [\"src/store.js\"]",
        "files_modified [\"src/store.js\", \"prisma/schema.prisma\"]",
    );
    assert_eq!(codes(&p), Vec::<String>::new());
    // The phase's first schema commit is recorded and judged only once a summary closes a plan.
    let p = repository();
    p.edit(
        "spec/plans.spec",
        "files_modified [\"src/store.js\"]",
        "files_modified [\"src/store.js\", \"prisma/schema.prisma\"]",
    );
    p.git_commit(
        "feat(02-01): Write the file",
        &[("prisma/schema.prisma", "model A {}\n")],
        T0,
    );
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    junit(&p, &[("adds a task", true)]);
    assert_eq!(codes(&p), Vec::<String>::new(), "{:?}", report(&p));
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

fn commit_gate(on: bool) -> serde_json::Value {
    json!({ "specforge.task_commits": on })
}

#[test]
fn task_without_commit_is_w407_only_when_task_commits_is_on() {
    let p = project("execute");
    assert_eq!(codes(&p), Vec::<String>::new(), "off by default");
    p.settings(commit_gate(true));
    // Plan 01-01 is complete with two tasks and no commit; 01-02 and 02-01 have not run.
    let found = report(&p);
    assert_eq!(codes(&p), ["gsd/W407", "gsd/W407"], "{found:?}");
    assert!(
        found[0].1.contains("v1.0/01/01-01/T1") && found[1].1.contains("v1.0/01/01-01/T2"),
        "{found:?}"
    );
    // A manual task is a human's step.
    p.edit(
        "spec/plans.spec",
        "task T2 \"Store tests\" {\n  plan   v1.0/01/01-01\n  type   tdd",
        "task T2 \"Store tests\" {\n  plan   v1.0/01/01-01\n  type   manual",
    );
    assert_eq!(codes(&p), ["gsd/W407"], "{:?}", report(&p));
    // A commit that names the task settles it.
    let p = repository();
    p.git_commit("feat(01-01): Store module", &[("src/tasks.js", "a\n")], T0);
    p.git_commit(
        "test(01-01): Store tests",
        &[("test/tasks.test.js", "t\n")],
        T1,
    );
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    junit(&p, &[("adds a task", true)]);
    p.settings(commit_gate(true));
    assert_eq!(codes(&p), Vec::<String>::new());
}

#[test]
fn w407_is_on_under_gsd_strict() {
    let p = project("execute");
    p.settings(json!({ "profile": "gsd-strict" }));
    assert_eq!(codes(&p), ["gsd/W407", "gsd/W407"], "{:?}", report(&p));
    p.settings(json!({ "profile": "gsd-compat" }));
    assert_eq!(codes(&p), Vec::<String>::new());
}

const CHECKS: &str = "{ kind unit ref \"test/tasks.test.js#adds a task\" }";

/// A JUnit report whose tests are named by `names`, each linked to the deliverable by the file it ran in.
fn junit(p: &Project, cases: &[(&str, bool)]) {
    let file = format!(
        "{}/test/tasks.test.js",
        p.root().canonicalize().expect("root").display()
    );
    let body: String = cases
        .iter()
        .map(|(name, ok)| {
            let inner = if *ok { "" } else { "<failure/>" };
            format!(
                "<testcase name=\"{name}\" classname=\"test\" file=\"{file}\">{inner}</testcase>\n"
            )
        })
        .collect();
    p.write(
        "junit.xml",
        &format!("<?xml version=\"1.0\"?>\n<testsuites>\n<testsuite name=\"s\">\n{body}</testsuite>\n</testsuites>\n"),
    );
    let run = p.collect(&["--runner", "gsd-junit", "--no-run"]);
    assert_eq!(run.code, 0, "{}", run.all());
}

#[test]
fn a_failing_check_is_w408() {
    let p = project("execute");
    junit(&p, &[("adds a task", false)]);
    let found = report(&p);
    assert_eq!(codes(&p), ["gsd/W408"], "{found:?}");
    assert!(
        found[0]
            .1
            .contains("'test/tasks.test.js#adds a task' is fail"),
        "{found:?}"
    );
    // A check that passes is clean.
    junit(&p, &[("adds a task", true)]);
    assert_eq!(codes(&p), Vec::<String>::new());
}

#[test]
fn an_unknown_check_is_w408() {
    let p = project("execute");
    // The report has a test for the deliverable, but not the one the second check names.
    p.edit("spec/plans.spec", CHECKS, "{ kind unit ref \"test/tasks.test.js#adds a task\" }\n    { kind unit ref \"test/tasks.test.js#lists tasks\" }");
    junit(&p, &[("lists tasks", true)]);
    let found = report(&p);
    assert_eq!(codes(&p), ["gsd/W408"], "{found:?}");
    assert!(
        found[0]
            .1
            .contains("'test/tasks.test.js#adds a task' is unknown"),
        "{found:?}"
    );
}

#[test]
fn a_deliverable_without_checks_is_w408() {
    let p = project("execute");
    p.edit(
        "spec/plans.spec",
        &format!("  checks         [\n    {CHECKS}\n  ]\n"),
        "",
    );
    assert_eq!(codes(&p), ["gsd/W408"], "{:?}", report(&p));
}

#[test]
fn a_human_judged_deliverable_needs_a_rationale() {
    let p = project("execute");
    p.edit(
        "spec/plans.spec",
        "human_judgment false",
        "human_judgment true",
    );
    assert_eq!(codes(&p), ["gsd/W408"], "{:?}", report(&p));
    p.edit(
        "spec/plans.spec",
        "human_judgment true",
        "human_judgment true\n  rationale      \"a person ran it\"",
    );
    assert_eq!(codes(&p), Vec::<String>::new());
}

#[test]
fn without_a_report_w408_does_not_judge_checks() {
    let p = project("execute");
    assert!(!p.exists("specforge-report.json"));
    assert_eq!(codes(&p), Vec::<String>::new());
    p.settings(json!({ "profile": "gsd-strict" }));
    // Under strict the W407 warnings arrive; the checks still say nothing.
    assert_eq!(codes(&p), ["gsd/W407", "gsd/W407"], "{:?}", report(&p));
}

#[test]
fn a_git_only_collect_leaves_the_checks_unjudged_even_under_strict() {
    let p = repository();
    p.settings(json!({ "profile": "gsd-strict" }));
    p.git_commit("feat(01-01): Store module", &[("src/tasks.js", "a\n")], T0);
    p.git_commit(
        "test(01-01): Store tests",
        &[("test/tasks.test.js", "t\n")],
        T1,
    );
    assert_eq!(p.collect(&["--runner", "gsd-git"]).code, 0);
    assert!(
        p.exists("specforge-report.json"),
        "the git collect writes a report"
    );
    // No test ever ran: no W408 (an Error under strict).
    assert_eq!(codes(&p), Vec::<String>::new(), "{:?}", report(&p));
}
