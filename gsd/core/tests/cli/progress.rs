//! The computed values of progress, end to end (`spec/project.md`): requirement, phase, milestone, project,
//! workstream and context values, from the plans, summaries and recorded commits.

use serde_json::json;

use crate::harness::{Project, project, taskboard};

const T0: &str = "2026-10-28T08:00:00Z";
const T1: &str = "2026-10-30T09:00:00Z";
const SUMMARY_02: &str = "summary 02-01-SUMMARY \"Save summary\" {\n  plan                   v1.0/02/02-01\n  status                 complete\n  one_liner              \"Tasks are saved\"\n  subsystem              \"store\"\n  requirements_completed [STORE-01]\n}\n";
const SUMMARY_12: &str = "summary 01-02-SUMMARY \"Completion summary\" {\n  plan                   v1.0/01/01-02\n  status                 complete\n  one_liner              \"Tasks are completed\"\n  subsystem              \"store\"\n  requirements_completed [TASK-02]\n}\n";

fn repository() -> Project {
    let p = project("execute");
    p.commit();
    p
}

fn field(p: &Project, id: &str, name: &str) -> serde_json::Value {
    p.fields_of(id)[name].clone()
}

fn append(p: &Project, file: &str, text: &str) {
    p.write(file, &format!("{}\n{text}", p.read(file)));
}

fn collect(p: &Project) {
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
}

#[test]
fn a_requirement_is_complete_once_a_plan_naming_it_completes() {
    let p = repository();
    assert_eq!(field(&p, "TASK-01", "status"), "complete");
    assert_eq!(field(&p, "TASK-02", "status"), "pending");
    append(&p, "spec/plans.spec", SUMMARY_12);
    assert_eq!(field(&p, "TASK-02", "status"), "complete");
}

#[test]
fn a_requirement_of_an_executing_plan_is_in_progress() {
    let p = repository();
    p.git_commit(
        "feat(01-02): Complete a task",
        &[("src/complete.js", "x\n")],
        T0,
    );
    collect(&p);
    assert_eq!(field(&p, "TASK-02", "status"), "in_progress");
    assert_eq!(field(&p, "STORE-01", "status"), "pending");
}

#[test]
fn a_requirement_whose_plans_all_halted_is_blocked() {
    let p = repository();
    p.edit(
        "spec/plans.spec",
        "status                 complete",
        "status                 halted",
    );
    assert_eq!(field(&p, "TASK-01", "status"), "blocked");
    // 01-02 depends on the halted 01-01, so TASK-02 is blocked too; STORE-01's plan is in another phase.
    assert_eq!(field(&p, "TASK-02", "status"), "blocked");
    assert_eq!(field(&p, "STORE-01", "status"), "pending");
}

#[test]
fn a_halted_plan_stays_counted_in_the_milestone() {
    let p = repository();
    p.edit(
        "spec/plans.spec",
        "status                 complete",
        "status                 halted",
    );
    // As the phase rows and gsd-tools count it (#2830): one summary, halted or not.
    assert_eq!(field(&p, "v1.0/01", "plans_complete"), "1/2");
    assert_eq!(p.fields_of("v1.0")["progress"]["plans_complete"], 1);
}

#[test]
fn the_phase_ladder_up_to_executed() {
    let p = repository();
    let phase = |id: &str| (field(&p, id, "plans_complete"), field(&p, id, "status"));
    assert_eq!(phase("v1.0/01"), (json!("1/2"), json!("in_progress")));
    assert_eq!(phase("v1.0/02"), (json!("0/1"), json!("planned")));
    append(
        &p,
        "spec/plans.spec",
        &format!("{SUMMARY_12}\n{SUMMARY_02}"),
    );
    assert_eq!(phase("v1.0/01"), (json!("2/2"), json!("executed")));
    assert_eq!(phase("v1.0/02"), (json!("1/1"), json!("executed")));
}

#[test]
fn an_inserted_and_a_backlog_phase_are_flagged() {
    let p = taskboard();
    let phases = p.fields_of("v1.0")["progress"]["phases"].clone();
    append(
        &p,
        "spec/v1.0.spec",
        "phase 999.1 \"Someday\" {\n  milestone        v1.0\n  name             \"Someday\"\n  slug             \"someday\"\n  goal             \"Later\"\n  success_criteria [\"a\", \"b\"]\n}\n",
    );
    let flags = |id: &str| (field(&p, id, "inserted"), field(&p, id, "backlog"));
    assert_eq!(flags("v1.0/02"), (json!(false), json!(false)));
    assert_eq!(flags("v1.0/02.1"), (json!(true), json!(false)));
    // A backlog phase is not an inserted one, and a milestone does not count it.
    assert_eq!(flags("v1.0/999.1"), (json!(false), json!(true)));
    assert_eq!(p.fields_of("v1.0")["progress"]["phases"], phases);
}

#[test]
fn milestone_stats_and_git_range_come_from_commits() {
    let p = repository();
    p.git_commit(
        "feat(01-01): Store module",
        &[("src/tasks.js", "a\nb\n")],
        T0,
    );
    p.git_commit(
        "feat(01-02): Complete a task",
        &[("src/complete.js", "x\n")],
        T1,
    );
    collect(&p);
    let milestone = p.fields_of("v1.0");
    assert_eq!(
        milestone["git_range"], "`feat(01-01)` → `feat(01-02)`",
        "{milestone}"
    );
    assert_eq!(
        milestone["stats"],
        json!({"phases": 2, "plans": 3, "tasks": 4, "files": 2, "loc": 3, "days": 2}),
        "{milestone}"
    );
    assert_eq!(milestone["state"], "in_progress");
    assert_eq!(
        milestone["progress"],
        json!({"phases": 2, "phases_complete": 0, "plans": 3, "plans_complete": 1, "percent": 0})
    );
}

#[test]
fn project_updated_is_the_latest_commit_or_completion() {
    let p = repository();
    assert!(p.fields_of("taskboard").get("updated").is_none());
    p.git_commit(
        "feat(01-02): Complete a task",
        &[("src/complete.js", "x\n")],
        T0,
    );
    collect(&p);
    assert_eq!(field(&p, "taskboard", "updated"), T0);
    // A summary completed later moves it.
    let anchor = "status                 complete\n";
    p.edit(
        "spec/plans.spec",
        anchor,
        &format!("{anchor}  completed              \"{T1}\"\n"),
    );
    p.record_harness(
        "v1.0/01/01-01/01-01-SUMMARY",
        "completed",
        &format!("\"{T1}\""),
    );
    assert_eq!(field(&p, "taskboard", "updated"), T1);
    // The latest is compared as a time, not as text: a fraction of a second is later than none.
    let fraction = "2026-10-29T10:00:00.250Z";
    p.edit(
        "spec/plans.spec",
        &format!("completed              \"{T1}\""),
        &format!("completed              \"{fraction}\""),
    );
    p.record_harness(
        "v1.0/01/01-01/01-01-SUMMARY",
        "completed",
        &format!("\"{fraction}\""),
    );
    p.git_commit(
        "feat(01-02): Complete a task",
        &[("src/z.js", "z\n")],
        "2026-10-29T10:00:00Z",
    );
    collect(&p);
    assert_eq!(field(&p, "taskboard", "updated"), fraction);
}

#[test]
fn validated_and_active_follow_requirement_status() {
    let p = repository();
    let project = p.fields_of("taskboard");
    assert_eq!(project["validated"], json!(["TASK-01"]), "{project}");
    assert_eq!(
        project["active"],
        json!(["STORE-01", "TASK-02"]),
        "{project}"
    );
}

#[test]
fn context_status_is_ready_once_bounded_and_referenced() {
    let p = project("execute");
    assert_eq!(
        field(&p, "v1.0/01/01-CONTEXT", "status"),
        "ready_for_planning"
    );
    p.edit(
        "spec/discuss.spec",
        "{ topic \"No external specs: requirements are fully captured in the decisions above\" paths [] }",
        "",
    );
    assert_eq!(field(&p, "v1.0/01/01-CONTEXT", "status"), "gathering");
}

#[test]
fn a_workstream_is_complete_when_its_milestones_shipped() {
    let p = project("execute");
    append(
        &p,
        "spec/v1.0.spec",
        "workstream auth \"Auth\" {\n  purpose \"Replace the store\"\n}\n\nmilestone v1.0 \"Auth 1.0\" {\n  name       \"Auth 1.0\"\n  workstream auth\n  completion { at 2026-10-30 }\n}\n",
    );
    p.record_harness("auth/v1.0", "completion", "{\"at\":\"2026-10-30\"}");
    assert_eq!(
        field(&p, "auth/v1.0", "state"),
        "shipped",
        "{}",
        p.fields_of("auth/v1.0")
    );
    assert_eq!(field(&p, "auth/v1.0", "shipped"), "2026-10-30");
    assert_eq!(field(&p, "auth", "status"), "complete");
    assert_eq!(field(&p, "v1.0", "state"), "in_progress");
}
