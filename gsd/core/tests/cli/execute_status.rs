//! The computed values of execution, end to end (`spec/plan.md`, `spec/execute.md`): a plan's and a task's status,
//! a summary's duration, actuals and key files, and a deliverable check's status, each from the recorded commits and
//! tests on the `execute` project.

use crate::harness::{Project, project};

const T0: &str = "2026-10-29T09:00:00Z";
const T1: &str = "2026-10-29T09:30:00Z";
const PLAN_1: &str = "v1.0/01/01-01";
const PLAN_2: &str = "v1.0/01/01-02";
const SUMMARY: &str = "v1.0/01/01-01/01-01-SUMMARY";
const DELIVERABLE: &str = "v1.0/01/01-01/01-01-SUMMARY/D1";

/// The execute project, committed once (the baseline), ready for `gsd-git`.
fn repository() -> Project {
    let p = project("execute");
    p.commit();
    p
}

fn status_of(p: &Project, id: &str) -> serde_json::Value {
    p.fields_of(id)["status"].clone()
}

fn collect_commits(p: &Project) {
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
}

#[test]
fn plan_status_follows_commits_and_halts() {
    let p = repository();
    // 01-01 has a complete summary, 01-02 depends on it and nothing names it, 02-01 is in another phase.
    assert_eq!(status_of(&p, PLAN_1), "complete");
    assert_eq!(status_of(&p, PLAN_2), "ready");
    assert_eq!(status_of(&p, "v1.0/02/02-01"), "ready");
    // A commit naming 01-02 makes it executing, and its task T1 (matched by title) done.
    p.git_commit(
        "feat(01-02): Complete a task",
        &[("src/complete.js", "x\n")],
        T0,
    );
    collect_commits(&p);
    assert_eq!(status_of(&p, PLAN_2), "executing");
    assert_eq!(status_of(&p, &format!("{PLAN_2}/T1")), "done");
    assert_eq!(status_of(&p, &format!("{PLAN_1}/T1")), "done");
    assert_eq!(p.codes(), Vec::<String>::new());
    // A halted 01-01 blocks 01-02 even though it has a commit; the other phase is untouched.
    p.edit(
        "spec/plans.spec",
        "status                 complete",
        "status                 halted",
    );
    assert_eq!(status_of(&p, PLAN_1), "halted");
    assert_eq!(status_of(&p, PLAN_2), "blocked");
    assert_eq!(status_of(&p, "v1.0/02/02-01"), "ready");
}

/// Writes a harness time into the summary and blesses it in the ledger, as the command that opens or closes a plan does.
fn stamp(p: &Project, field: &str, at: &str) {
    let anchor = "status                 complete\n";
    p.edit(
        "spec/plans.spec",
        anchor,
        &format!("{anchor}  {field:<22} \"{at}\"\n"),
    );
    p.record_harness(SUMMARY, field, &format!("\"{at}\""));
}

#[test]
fn duration_is_gsds_through_the_host() {
    let p = project("execute");
    assert!(p.fields_of(SUMMARY).get("duration").is_none());
    stamp(&p, "started", "2026-10-28T08:00:00Z");
    assert!(p.fields_of(SUMMARY).get("duration").is_none());
    stamp(&p, "completed", "2026-10-28T09:05:00Z");
    assert_eq!(
        p.fields_of(SUMMARY)["duration"],
        "1h 5m",
        "{}",
        p.fields_of(SUMMARY)
    );
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn actuals_and_key_files_come_from_commits_through_the_host() {
    let p = repository();
    assert_eq!(
        p.fields_of(SUMMARY)["actuals"],
        serde_json::json!({"tasks": 2, "commits": 0})
    );
    assert!(p.fields_of(SUMMARY).get("files_created").is_none());
    p.git_commit("feat(01-01): Store module", &[("src/tasks.js", "a\n")], T0);
    p.git_commit(
        "test(01-01): Store tests",
        &[("test/tasks.test.js", "b\n")],
        T1,
    );
    p.git_commit(
        "fix(01-01): Order",
        &[("src/tasks.js", "c\n")],
        "2026-10-29T10:00:00Z",
    );
    collect_commits(&p);
    let fields = p.fields_of(SUMMARY);
    assert_eq!(
        fields["actuals"],
        serde_json::json!({"tasks": 2, "commits": 3}),
        "{fields}"
    );
    assert_eq!(
        fields["files_created"],
        serde_json::json!(["src/tasks.js", "test/tasks.test.js"]),
        "{fields}"
    );
    assert!(fields.get("files_modified").is_none(), "{fields}");
}

fn junit(p: &Project, testcases: &str) {
    p.write(
        "junit.xml",
        &format!("<?xml version=\"1.0\"?>\n<testsuites>\n<testsuite name=\"s\">\n{testcases}\n</testsuite>\n</testsuites>\n"),
    );
    let run = p.collect(&["--runner", "gsd-junit", "--no-run"]);
    assert_eq!(run.code, 0, "{}", run.all());
}

fn statuses(p: &Project) -> Vec<String> {
    p.fields_of(DELIVERABLE)["checks"]
        .as_array()
        .unwrap_or_else(|| panic!("checks: {}", p.fields_of(DELIVERABLE)))
        .iter()
        .map(|c| c["status"].as_str().unwrap_or("-").to_string())
        .collect()
}

#[test]
fn a_check_status_is_pass_fail_or_unknown_from_the_report() {
    let p = project("execute");
    // Two more checks: one a test that fails, one that nobody ran.
    p.edit(
        "spec/plans.spec",
        "{ kind unit ref \"test/tasks.test.js#adds a task\" }",
        "{ kind unit ref \"test/tasks.test.js#adds a task\" }\n    { kind unit ref \"test/tasks.test.js#lists tasks\" }\n    { kind unit ref \"test/tasks.test.js#never ran\" }",
    );
    assert_eq!(statuses(&p), ["-", "-", "-"], "no report, no status");
    let file = format!(
        "{}/test/tasks.test.js",
        p.root().canonicalize().expect("root").display()
    );
    junit(
        &p,
        &format!(
            "<testcase name=\"adds a task\" classname=\"test\" file=\"{file}\"/>\n<testcase name=\"lists tasks\" classname=\"test\" file=\"{file}\"><failure/></testcase>"
        ),
    );
    assert_eq!(statuses(&p), ["pass", "fail", "unknown"]);
}

#[test]
fn without_a_report_the_compile_says_i205_once_and_collect_silences_it() {
    let p = project("execute");
    let noted = |p: &Project| {
        p.run(&["check", "--format", "json"])
            .stdout
            .matches("\"I205\"")
            .count()
    };
    assert_eq!(noted(&p), 1);
    junit(&p, "<testcase name=\"unrelated\" classname=\"test\"/>");
    assert_eq!(noted(&p), 0);
}
