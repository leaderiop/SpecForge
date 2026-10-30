//! `phase`: add, insert, remove and edit a phase of the roadmap (D-r41-8).

use serde_json::json;

use super::commands_support::*;
use super::harness::{Project, project};

/// A milestone of phases 01 to 12, `05.1` inserted and `999.1` in the backlog; phases 05 to 12 have a plan `NN-01` and
/// a context `NN-CONTEXT`, each depending on the one before.
pub(super) fn twelve() -> Project {
    let p = project("empty");
    let mut text = String::from("milestone v1.0 \"M\" {\n  name \"M\"\n}\n\n");
    let phase = |name: &str, depends: &str| {
        let deps = if depends.is_empty() {
            String::new()
        } else {
            format!("  depends_on [{depends}]\n")
        };
        format!(
            "phase {name} \"P{name}\" {{\n  milestone v1.0\n  name \"P{name}\"\n  slug \"p{name}\"\n  goal \"Goal {name}\"\n  success_criteria [\"a\", \"b\"]\n{deps}}}\n\n"
        )
    };
    for n in 1..=12 {
        let previous = if n == 1 {
            String::new()
        } else {
            format!("{:02}", n - 1)
        };
        text.push_str(&phase(&format!("{n:02}"), &previous));
        if n == 5 {
            text.push_str(&phase("05.1", "05"));
        }
    }
    text.push_str(&phase("999.1", ""));
    for n in 5..=12 {
        text.push_str(&format!(
            "plan {n:02}-01 \"Plan {n}\" {{\n  phase v1.0/{n:02}\n  type execute\n  files_modified [\"a\"]\n  objective \"o\"\n}}\n\ncontext {n:02}-CONTEXT \"Context {n}\" {{\n  phase v1.0/{n:02}\n  gathered 2026-10-28\n  boundary \"b\"\n}}\n\n"
        ));
    }
    p.write("spec/m.spec", &text);
    p
}

fn ids(p: &Project, kind: &str) -> Vec<String> {
    let mut found: Vec<String> = p
        .nodes_of(kind)
        .iter()
        .map(|n| n["id"].as_str().unwrap_or_default().to_string())
        .collect();
    found.sort();
    found
}

#[test]
fn remove_renumbers_twelve_phases() {
    let p = twelve();
    let run = gsd(&p, "phase", &["--remove", "04"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let phases: Vec<String> = ids(&p, "phase")
        .into_iter()
        .map(|i| i.replace("v1.0/", ""))
        .collect();
    assert_eq!(
        phases,
        [
            "01", "02", "03", "04", "04.1", "05", "06", "07", "08", "09", "10", "11", "999.1"
        ]
    );
    let plans: Vec<String> = ids(&p, "plan");
    assert_eq!(
        plans.first().map(String::as_str),
        Some("v1.0/04/04-01"),
        "{plans:?}"
    );
    assert_eq!(
        plans.last().map(String::as_str),
        Some("v1.0/11/11-01"),
        "{plans:?}"
    );
    assert_eq!(
        ids(&p, "context").first().map(String::as_str),
        Some("v1.0/04/04-CONTEXT")
    );
    // The removed phase's dependent inherits its dependency (the chain stays linear); the others follow their rename.
    assert_eq!(p.fields_of("v1.0/04")["depends_on"], json!(["03"]));
    assert_eq!(p.fields_of("v1.0/05")["depends_on"], json!(["04"]));
    assert_eq!(p.fields_of("v1.0/11")["depends_on"], json!(["10"]));
    assert_eq!(p.fields_of("v1.0/04.1")["depends_on"], json!(["04"]));
    assert_eq!(p.fields_of("v1.0/999.1")["goal"], "Goal 999.1");
    assert_eq!(p.fields_of("v1.0/04/04-01")["phase"], "v1.0/04");
}

/// Phases 05 and 05.1 have run: a plan, a summary, a deliverable-free UAT session and a requirement they trace.
fn started_twelve() -> Project {
    let p = twelve();
    p.edit(
        "spec/m.spec",
        "phase 05 \"P05\" {\n  milestone v1.0\n",
        "phase 05 \"P05\" {\n  milestone v1.0\n  requirements [R1]\n",
    );
    plant(
        &p,
        "spec/m.spec",
        "requirement R1 \"One\" {\n  text \"t\"\n  category \"c\"\n  tier v1\n}\n\nplan 05.1-01 \"Plan 5.1\" {\n  phase v1.0/05.1\n  type execute\n  files_modified [\"a\"]\n  objective \"o\"\n}\n",
    );
    for (phase, plan) in [("05", "05-01"), ("05.1", "05.1-01")] {
        plant(
            &p,
            "spec/m.spec",
            &format!(
                "summary {plan}-SUMMARY \"S\" {{\n  plan v1.0/{phase}/{plan}\n  status complete\n  one_liner \"o\"\n  subsystem \"s\"\n  requirements_completed [R1]\n}}\n\nuat_session {phase}-UAT \"U\" {{\n  phase v1.0/{phase}\n  source [v1.0/{phase}/{plan}/{plan}-SUMMARY]\n}}\n"
            ),
        );
    }
    p
}

#[test]
fn remove_renames_every_nested_child_and_leaves_nothing_dangling() {
    let p = started_twelve();
    let before = p.codes();
    let run = gsd(&p, "phase", &["--remove", "04", "--force"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(
        ids(&p, "summary"),
        [
            "v1.0/04.1/04.1-01/04.1-01-SUMMARY",
            "v1.0/04/04-01/04-01-SUMMARY"
        ]
    );
    assert_eq!(
        ids(&p, "uat_session"),
        ["v1.0/04.1/04.1-UAT", "v1.0/04/04-UAT"]
    );
    assert_eq!(
        p.fields_of("v1.0/04/04-01/04-01-SUMMARY")["plan"],
        "v1.0/04/04-01"
    );
    assert_eq!(
        p.fields_of("v1.0/04.1/04.1-UAT")["source"],
        json!(["v1.0/04.1/04.1-01/04.1-01-SUMMARY"])
    );
    assert_eq!(p.fields_of("v1.0/04")["requirements"], json!(["R1"]));
    // Nothing dangles: the check adds no code.
    let after = p.codes();
    assert!(
        after.iter().all(|c| before.contains(c)),
        "{before:?} -> {after:?}"
    );
}

#[test]
fn remove_refuses_a_later_phase_that_has_started_without_force() {
    let p = started_twelve();
    let before = bytes(&p);
    let run = gsd(&p, "phase", &["--remove", "04"]);
    refused(&run, "invalid_input", &["phase 05 has started", "--force"]);
    unchanged(&p, &before);
}

#[test]
fn insert_after_02_on_the_twin_makes_02_2() {
    let p = twin();
    let run = gsd(&p, "phase", &["--insert", "02", "--name", "Urgent fix"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let fields = p.fields_of("v1.0/02.2");
    assert_eq!(fields["goal"], "[Urgent work - to be planned]");
    assert_eq!(fields["depends_on"], json!(["v1.0/02"]));
    assert_eq!(fields["slug"], "urgent-fix");
    assert!(ids(&p, "phase").contains(&"v1.0/02.1".to_string()));
}

#[test]
fn add_appends_with_to_be_planned() {
    let p = twin();
    let run = gsd(&p, "phase", &["--name", "Sharing"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let fields = p.fields_of("v1.0/04");
    assert_eq!(fields["goal"], "[To be planned]");
    assert_eq!(fields["success_criteria"], json!(["[To be planned]"]));
    assert_eq!(fields["depends_on"], json!(["v1.0/03"]));
    assert!(
        p.codes().contains(&"gsd/W103".to_string()),
        "{:?}",
        p.codes()
    );
}

#[test]
fn remove_refuses_an_executed_phase_without_force() {
    let p = project("execute");
    let before = bytes(&p);
    let run = gsd(&p, "phase", &["--remove", "01"]);
    refused(&run, "invalid_input", &["has started", "--force"]);
    unchanged(&p, &before);
}

#[test]
fn remove_with_force_deletes_its_plans() {
    let p = project("execute");
    // The host refuses an edit that adds an error: phase 01's requirements would be in no phase.
    let text = p.read("spec/v1.0.spec");
    let kept: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with("requirements "))
        .map(|l| format!("{l}\n"))
        .collect();
    p.write("spec/v1.0.spec", &kept);
    let run = gsd(&p, "phase", &["--remove", "01", "--force"]);
    assert_eq!(run.code, 0, "{}", run.all());
    // Phase 01's plans and summary are gone; phase 02's plan took the name 01-01.
    let plans = ids(&p, "plan");
    assert!(plans.contains(&"v1.0/01/01-01".to_string()), "{plans:?}");
    assert!(!plans.iter().any(|i| i.ends_with("01-02")), "{plans:?}");
    assert!(ids(&p, "summary").is_empty(), "{:?}", ids(&p, "summary"));
    assert_eq!(
        p.fields_of("v1.0/01/01-01")["objective"],
        "Tasks are written to one JSON file"
    );
}

const OVERRIDE: &str = "\noverride OV9 \"Accepted\" {\n  verification v1.0/01/01-VERIFICATION\n  truth        TRUTH\n  reason       \"by hand\"\n  by           \"ada\"\n  at           2026-10-29\n}\n";

#[test]
fn remove_refuses_a_phase_holding_an_override_and_names_the_step() {
    let p = project("verify");
    let before = bytes(&p);
    let run = gsd(
        &p,
        "phase",
        &["--remove", "01", "--force", "--format", "json"],
    );
    assert_eq!(run.code, 1, "{}", run.all());
    let all = run.all();
    for word in [
        "human_only",
        "OV1",
        "delete or move it in your editor",
        "/gsd-phase --remove 01",
    ] {
        assert!(all.contains(word), "'{word}' not in: {all}");
    }
    unchanged(&p, &before);
}

#[test]
fn remove_refuses_when_an_override_outside_names_a_moved_truth() {
    let p = project("verify");
    // The fixture's own override goes; a person's override names phase 02's truth, which would move down.
    let text = p.read("spec/verify.spec");
    let at = text.find("override OV1").expect("the override");
    let end = text[at..].find("\n}\n").map_or(text.len(), |n| at + n + 3);
    p.write(
        "spec/verify.spec",
        &format!(
            "{}{}{}",
            &text[..at],
            OVERRIDE
                .replace("TRUTH", "v1.0/02/02-01/TR1")
                .replace("OV9", "OV2")
                .trim_start(),
            &text[end..]
        ),
    );
    let before = bytes(&p);
    let run = gsd(
        &p,
        "phase",
        &["--remove", "01", "--force", "--format", "json"],
    );
    assert_eq!(run.code, 1, "{}", run.all());
    assert!(
        run.all().contains("human_only") && run.all().contains("OV2"),
        "{}",
        run.all()
    );
    unchanged(&p, &before);
}

#[test]
fn edit_sets_goal_and_criteria() {
    let p = twin();
    let run = gsd(
        &p,
        "phase",
        &[
            "--edit",
            "02",
            "--goal",
            "Tasks survive",
            "--criteria",
            r#"["saved","loaded"]"#,
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let fields = p.fields_of("v1.0/02");
    assert_eq!(fields["goal"], "Tasks survive");
    assert_eq!(fields["success_criteria"], json!(["saved", "loaded"]));
}

#[test]
fn insert_and_edit_refuse_a_backlog_phase() {
    let p = twelve();
    let before = bytes(&p);
    for args in [
        vec!["--insert", "999.1", "--name", "x"],
        vec!["--edit", "999.1", "--goal", "x"],
    ] {
        let run = gsd(&p, "phase", &args);
        refused(&run, "invalid_input", &["backlog"]);
    }
    unchanged(&p, &before);
}

/// `twelve()` with a context `999.1-CONTEXT` under the backlog phase and a second backlog phase `999.2`.
pub(super) fn backlogged() -> Project {
    let p = twelve();
    let mut text = p.read("spec/m.spec");
    text.push_str("context 999.1-CONTEXT \"Idea\" {\n  phase v1.0/999.1\n  gathered 2026-10-28\n  boundary \"b\"\n  canonical_refs [{ topic \"none\" paths [] }]\n}\n\nphase 999.2 \"P999.2\" {\n  milestone v1.0\n  name \"P999.2\"\n  slug \"p999-2\"\n  goal \"Goal 999.2\"\n  success_criteria [\"a\"]\n}\n");
    p.write("spec/m.spec", &text);
    p
}

#[test]
fn promote_moves_a_backlog_phase_into_the_milestone() {
    let p = backlogged();
    let run = gsd(&p, "phase", &["--promote", "999.1"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let phases = ids(&p, "phase");
    assert!(phases.contains(&"v1.0/13".to_string()), "{phases:?}");
    assert!(!phases.contains(&"v1.0/999.1".to_string()), "{phases:?}");
    assert!(phases.contains(&"v1.0/999.2".to_string()), "{phases:?}");
    assert_eq!(
        ids(&p, "context").last().map(String::as_str),
        Some("v1.0/13/13-CONTEXT")
    );
    let fields = p.fields_of("v1.0/13");
    assert!(fields["depends_on"].to_string().contains("12"), "{fields}");
    assert_eq!(fields["goal"], "Goal 999.1");
    assert_eq!(p.fields_of("v1.0/13/13-CONTEXT")["phase"], "v1.0/13");
}

#[test]
fn promote_dry_run_shows_the_plan() {
    let p = backlogged();
    let before = bytes(&p);
    let run = gsd(&p, "phase", &["--promote", "999.1", "--dry-run"]);
    assert_eq!(run.code, 0, "{}", run.all());
    unchanged(&p, &before);
}

#[test]
fn promote_refuses_a_phase_that_is_not_backlog() {
    let p = backlogged();
    let before = bytes(&p);
    let run = gsd(&p, "phase", &["--promote", "05"]);
    refused(&run, "invalid_input", &["backlog"]);
    unchanged(&p, &before);
}

#[test]
fn promote_refuses_a_phase_holding_a_human_value() {
    let p = backlogged();
    let mut text = p.read("spec/m.spec");
    text.push_str("\nverification 999.1-VERIFICATION \"v\" {\n  phase v1.0/999.1\n}\n");
    text.push_str("\noverride OV9 \"Accepted\" {\n  verification v1.0/999.1/999.1-VERIFICATION\n  truth        TRUTH\n  reason       \"by hand\"\n  by           \"ada\"\n  at           2026-10-29\n}\n");
    p.write("spec/m.spec", &text);
    let before = bytes(&p);
    let run = gsd(&p, "phase", &["--promote", "999.1", "--format", "json"]);
    assert_eq!(run.code, 1, "{}", run.all());
    assert!(run.all().contains("human_only"), "{}", run.all());
    unchanged(&p, &before);
}

#[test]
fn remove_deletes_a_backlog_phase_without_renumbering() {
    let p = backlogged();
    let run = gsd(&p, "phase", &["--remove", "999.1"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let phases = ids(&p, "phase");
    assert!(!phases.contains(&"v1.0/999.1".to_string()), "{phases:?}");
    assert!(phases.contains(&"v1.0/999.2".to_string()), "{phases:?}");
    assert!(phases.contains(&"v1.0/12".to_string()), "{phases:?}");
    assert!(ids(&p, "context").iter().all(|c| !c.contains("999")));
}

#[test]
fn gsd_phase_skill_renders() {
    let p = project("empty");
    let run = p.install("claude-code", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    let skill = p.read(".claude/skills/gsd-phase/SKILL.md");
    assert!(skill.contains("gsd-phase"), "{skill}");
    assert!(
        skill.contains("[--insert | --remove | --edit] <phase-name-or-number>"),
        "{skill}"
    );
    assert!(skill.contains("commands/gsd/phase.md"), "{skill}");
    assert!(
        skill.contains("gsd phase") || skill.contains("gsd_phase"),
        "{skill}"
    );
}

#[test]
fn a_criterion_with_a_control_character_or_an_empty_name_is_refused() {
    let p = twin();
    let before = bytes(&p);
    for (args, words) in [
        (
            vec![
                "--edit",
                "02",
                "--criteria",
                "[\"ok\", \"a\\u001b[31mred\"]",
            ],
            "'criteria' holds a control character (U+001B)",
        ),
        (
            vec!["--edit", "02", "--criteria", "[\"a\\u202eb\"]"],
            "(U+202E)",
        ),
        (vec!["--edit", "02", "--name", ""], "'name' is empty"),
        (vec!["--edit", "02", "--goal", "  "], "'goal' is empty"),
    ] {
        let run = gsd(&p, "phase", &args);
        refused(&run, "invalid_input", &[words]);
        unchanged(&p, &before);
    }
}

/// `backlogged()` with a second milestone `v1.1` that holds phase `01` (or nothing, when `empty`).
fn two_milestones(empty: bool) -> Project {
    let p = backlogged();
    let phase = if empty {
        ""
    } else {
        "\nphase 01 \"P01\" {\n  milestone v1.1\n  name \"P01\"\n  slug \"p01\"\n  goal \"g\"\n  success_criteria [\"a\"]\n}\n"
    };
    p.write(
        "spec/n.spec",
        &format!("milestone v1.1 \"N\" {{\n  name \"N\"\n}}\n{phase}"),
    );
    p
}

/// REVIEW-r43 M13: the backlog is roadmap-wide (upstream's `## Backlog`), so a 999.x of v1.0 is promoted into v1.1.
#[test]
fn promote_takes_a_backlog_phase_of_another_milestone() {
    let p = two_milestones(false);
    let run = gsd(&p, "phase", &["--promote", "999.1", "--milestone", "v1.1"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let phases = ids(&p, "phase");
    assert!(phases.contains(&"v1.1/02".to_string()), "{phases:?}");
    assert!(!phases.contains(&"v1.0/999.1".to_string()), "{phases:?}");
    assert!(phases.contains(&"v1.0/999.2".to_string()), "{phases:?}");
    assert!(
        ids(&p, "context").contains(&"v1.1/02/02-CONTEXT".to_string()),
        "{:?}",
        ids(&p, "context")
    );
    assert_eq!(p.fields_of("v1.1/02")["milestone"], "v1.1");
    let fields = p.fields_of("v1.1/02");
    assert!(fields["depends_on"].to_string().contains("01"), "{fields}");
}

#[test]
fn promote_into_a_milestone_without_phases_depends_on_nothing() {
    let p = two_milestones(true);
    let run = gsd(&p, "phase", &["--promote", "999.1", "--milestone", "v1.1"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(ids(&p, "phase").contains(&"v1.1/01".to_string()));
}

#[test]
fn promote_names_the_milestone_when_two_backlogs_share_a_number() {
    let p = two_milestones(false);
    let mut text = p.read("spec/n.spec");
    text.push_str("\nphase 999.1 \"Other\" {\n  milestone v1.1\n  name \"Other\"\n  slug \"other\"\n  goal \"g\"\n  success_criteria [\"a\"]\n}\n");
    p.write("spec/n.spec", &text);
    // v1.1's own 999.1 wins for --milestone v1.1; the other milestone's is named with its id
    let before = bytes(&p);
    let run = gsd(
        &p,
        "phase",
        &["--promote", "v1.0/999.1", "--milestone", "v1.1"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_ne!(before, bytes(&p));
    assert!(ids(&p, "phase").contains(&"v1.1/999.1".to_string()));
}
