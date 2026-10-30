//! `health`, `stats` and `progress_report`: gsd-tools' reports, answered from the computed values the graph a command
//! receives carries (ADR 0299).

use std::path::Path;

use serde_json::{Value, json};

use super::commands_support::*;
use super::harness::project;

fn golden(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity/goldens/taskboard")
        .join(format!("{name}.json"));
    serde_json::from_str(&std::fs::read_to_string(&path).expect("the golden")).expect("JSON")
}

fn report(p: &super::harness::Project, command: &str) -> Value {
    let run = gsd(p, command, &["--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    json(&run)
}

/// The twin with the verifier's verdicts on phase 01 (its status is `Complete`).
fn verified() -> super::harness::Project {
    let p = twin();
    assert_eq!(p.verify_phase("v1.0/01"), 0);
    p
}

#[test]
fn health_equals_the_golden_on_the_twin() {
    // The phases' computed `plans_complete` reach the command.
    assert_eq!(report(&twin(), "health"), golden("validate-health"));
}

#[test]
fn progress_report_equals_the_golden_on_the_verified_twin() {
    let mut got = report(&verified(), "progress-report");
    // Upstream reads the milestone from STATE.md, which the twin has none of.
    assert_eq!(got["milestone_version"], "v1.0");
    assert_eq!(got["milestone_name"], "Taskboard 1.0");
    let mut want = golden("progress-json");
    for side in [&mut got, &mut want] {
        side["milestone_version"] = Value::Null;
        side["milestone_name"] = Value::Null;
    }
    assert_eq!(got, want);
}

/// The twin as committed, with no `record-verdict` run, is what gsd-tools says: phase 01 is complete.
#[test]
fn phase_01_is_complete_on_disk() {
    let mut got = report(&twin(), "progress-report");
    let mut want = golden("progress-json");
    for side in [&mut got, &mut want] {
        side["milestone_version"] = Value::Null;
        side["milestone_name"] = Value::Null;
    }
    assert_eq!(got, want);
}

#[test]
fn progress_report_on_the_unverified_twin_differs_only_in_phase_01() {
    let p = twin();
    p.strip_verdicts();
    let got = report(&p, "progress-report");
    let mut want = golden("progress-json");
    want["milestone_version"] = json!("v1.0");
    want["milestone_name"] = json!("Taskboard 1.0");
    // Without the verifier's verdicts phase 01 stops at Executed (D-r41-18).
    assert_eq!(want["phases"][0]["status"], "Complete");
    want["phases"][0]["status"] = json!("Executed");
    assert_eq!(got, want);
}

#[test]
fn stats_counts_phases_plans_and_requirements() {
    let got = report(&verified(), "stats");
    assert_eq!(got["phases_total"], 4);
    assert_eq!(got["phases_completed"], 1);
    assert_eq!(got["percent"], 25);
    assert_eq!(got["total_plans"], 3);
    assert_eq!(got["total_summaries"], 2);
    assert_eq!(got["plan_percent"], 67);
    // Only the v1 requirements have a checkbox upstream: the v2 and the out-of-scope ones are not counted.
    assert_eq!(got["requirements_total"], 6);
    assert_eq!(got["requirements_complete"], 2);
    // Nothing is recorded: upstream's `null` last activity, `0` commits and no first-commit date.
    assert!(got["last_activity"].is_null(), "{got}");
    assert_eq!(got["git_commits"], 0, "{got}");
    assert!(got["git_first_commit_date"].is_null(), "{got}");
    // `stats` says `Not Started` where `progress json` says `Pending`.
    assert_eq!(got["phases"][3]["status"], "Not Started", "{got}");
}

#[test]
fn stats_last_activity_is_the_newest_summary_completion() {
    let p = twin();
    let text = p.read("spec/plans.spec");
    p.write(
        "spec/plans.spec",
        &text.replacen(
            "  requirements_completed [TASK-01]\n}",
            "  requirements_completed [TASK-01]\n  completed              \"2026-09-01T10:00:00Z\"\n}",
            1,
        ),
    );
    assert_eq!(report(&p, "stats")["last_activity"], "2026-09-01");
}

/// The reports leave the backlog (`999.x`) phases out, as gsd does (#3167) and `milestone.progress` does.
#[test]
fn reports_leave_backlog_phases_out() {
    let p = super::cmd_phase::twelve();
    for command in ["progress-report", "stats"] {
        let got = report(&p, command);
        let numbers: Vec<&str> = got["phases"]
            .as_array()
            .expect("phases")
            .iter()
            .map(|ph| ph["number"].as_str().unwrap_or_default())
            .collect();
        assert!(!numbers.contains(&"999.1"), "{command}: {numbers:?}");
        assert_eq!(numbers.len(), 13, "{command}: {numbers:?}");
    }
    assert_eq!(report(&p, "stats")["phases_total"], 13);
}

#[test]
fn stats_on_the_twin_leave_a_backlog_phase_out() {
    let p = verified();
    plant(
        &p,
        "spec/v1.0.spec",
        "phase 999.1 \"Someday\" {\n  milestone v1.0\n  name \"Someday\"\n  slug \"someday\"\n}\n",
    );
    let got = report(&p, "stats");
    assert_eq!(got["phases_total"], 4, "{got}");
    assert_eq!(got["percent"], 25, "{got}");
    assert!(!got.to_string().contains("999.1"), "{got}");
}

#[test]
fn stats_git_fields_come_from_recorded_commits() {
    let p = project("execute");
    p.commit();
    p.git_commit(
        "feat(01-01): Store module",
        &[("src/a.js", "x\n")],
        "2026-10-29T09:00:00Z",
    );
    p.git_commit(
        "fix(01-01): Trim",
        &[("src/b.js", "y\n")],
        "2026-10-30T09:00:00Z",
    );
    assert_eq!(p.collect(&["--runner", "gsd-git"]).code, 0);
    let got = report(&p, "stats");
    assert_eq!(got["git_commits"], 2, "{got}");
    assert_eq!(got["git_first_commit_date"], "2026-10-29", "{got}");
}

#[test]
fn gsd_health_and_gsd_stats_skills_render() {
    let p = project("empty");
    let run = p.install("claude-code", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    for (skill, tool) in [
        ("gsd-health", "specforge.gsd.health"),
        ("gsd-stats", "specforge.gsd.stats"),
    ] {
        let text = p.read(&format!(".claude/skills/{skill}/SKILL.md"));
        assert!(
            text.contains(&format!("name: {skill}")) && text.contains(tool),
            "{text}"
        );
    }
}
