//! `workstreams`, `cleanup` and `gsd/E517` (D-r41-9..11).

use serde_json::json;

use super::commands_support::*;
use super::harness::{Project, project};

fn plus_streams() -> Project {
    let p = project("workstreams");
    plus(&p);
    p
}

#[test]
fn switch_writes_the_active_workstream_to_project_state() {
    let p = project("workstreams");
    let run = gsd(&p, "workstreams", &["switch", "--name", "auth"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let state = p.read(".specforge/state/gsd__core/project.json");
    let state: serde_json::Value = serde_json::from_str(&state).expect("JSON");
    assert_eq!(state["active_workstream"], "auth");
}

#[test]
fn resume_writes_it_too_and_a_missing_workstream_is_refused() {
    let p = project("workstreams");
    let before = bytes(&p);
    refused(
        &gsd(&p, "workstreams", &["resume", "--name", "nope"]),
        "invalid_input",
        &["nope"],
    );
    unchanged(&p, &before);
    assert_eq!(
        gsd(&p, "workstreams", &["resume", "--name", "store"]).code,
        0
    );
    assert!(
        p.read(".specforge/state/gsd__core/project.json")
            .contains("store")
    );
}

#[test]
fn create_on_the_twin_creates_a_workstream() {
    let p = twin();
    let run = gsd(
        &p,
        "workstreams",
        &[
            "create",
            "--name",
            "billing",
            "--purpose",
            "Charge for boards",
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of("billing")["purpose"], "Charge for boards");
    let again = gsd(
        &p,
        "workstreams",
        &["create", "--name", "billing", "--purpose", "x"],
    );
    refused(&again, "invalid_input", &["already exists"]);
}

#[test]
fn list_and_progress_answer_from_the_graph() {
    let p = project("workstreams");
    let run = gsd(&p, "workstreams", &["list", "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let found = json(&run);
    let rows = found["workstreams"].as_array().expect("a list");
    assert_eq!(rows.len(), 2, "{found}");
    let progress = json(&gsd(
        &p,
        "workstreams",
        &["progress", "--name", "auth", "--format", "json"],
    ));
    let milestone = &progress["workstreams"][0]["milestones"][0];
    assert_eq!(milestone["milestone"], "auth/v1.0");
    assert_eq!(milestone["plans"], 1, "{progress}");
    assert_eq!(milestone["phases"], 1);
    assert_eq!(milestone["percent"], 0);
}

#[test]
fn complete_reports_unshipped_milestones() {
    let p = project("workstreams");
    let before = bytes(&p);
    let run = gsd(
        &p,
        "workstreams",
        &["complete", "--name", "auth", "--format", "json"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let found = json(&run);
    assert_eq!(found["complete"], false);
    assert_eq!(found["open"], json!(["auth/v1.0"]));
    unchanged(&p, &before);
}

fn codes_e517(p: &Project) -> Vec<String> {
    p.report_all()
        .into_iter()
        .filter(|(c, _)| c == "gsd/E517")
        .map(|(_, m)| m)
        .collect()
}

#[test]
fn e517_fires_on_overlapping_active_plans_under_plus() {
    let p = plus_streams();
    let found = codes_e517(&p);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("src/tasks.js"), "{found:?}");
}

#[test]
fn e517_is_silent_by_default() {
    let p = project("workstreams");
    assert!(codes_e517(&p).is_empty());
}

#[test]
fn e517_is_silent_once_one_plan_completed() {
    let p = plus_streams();
    plant(
        &p,
        "spec/streams.spec",
        "summary 01-01-SUMMARY \"Done\" {\n  plan                   auth/v1.0/01/01-01\n  status                 complete\n  one_liner              \"Signed in\"\n  subsystem              \"auth\"\n  requirements_completed [TASK-01]\n  completed              \"2026-10-29T10:00:00Z\"\n}\n",
    );
    p.record_harness(
        "auth/v1.0/01/01-01/01-01-SUMMARY",
        "completed",
        "\"2026-10-29T10:00:00Z\"",
    );
    assert!(codes_e517(&p).is_empty(), "{:?}", codes_e517(&p));
}

#[test]
fn cleanup_on_the_twin_says_nothing_is_shipped() {
    let p = twin();
    let before = bytes(&p);
    let run = gsd(&p, "cleanup", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    let all = run.all();
    assert!(
        all.contains("nothing is shipped") && all.contains("nothing moves"),
        "{all}"
    );
    unchanged(&p, &before);
}

#[test]
fn cleanup_lists_a_shipped_milestone_and_writes_nothing() {
    let p = project("workstreams");
    p.edit(
        "spec/streams.spec",
        "  name       \"Auth 1.0\"\n  workstream auth\n",
        "  name       \"Auth 1.0\"\n  workstream auth\n  completion { at 2026-10-29 }\n",
    );
    p.record_harness("auth/v1.0", "completion", "{\"at\":\"2026-10-29\"}");
    let before = bytes(&p);
    let run = gsd(&p, "cleanup", &["--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let found = json(&run);
    assert_eq!(found["shipped"][0]["milestone"], "auth/v1.0", "{found}");
    assert_eq!(found["archived"], false);
    unchanged(&p, &before);
}

#[test]
fn gsd_workstreams_and_gsd_cleanup_skills_render() {
    let p = project("empty");
    assert_eq!(p.install("claude-code", &[]).code, 0);
    for skill in ["gsd-workstreams", "gsd-cleanup"] {
        let text = p.read(&format!(".claude/skills/{skill}/SKILL.md"));
        assert!(text.contains(skill), "{text}");
    }
}
