//! The contract of the whole roster of writing commands (K5, K9): a dry run shows the edit plan and writes nothing, a
//! failing gate refuses and writes nothing, and an unknown entity is refused by its name.

use serde_json::json;

use super::cmd_execute::{CHECKPOINT, PLAN, PLANTED, SETUP, STARTED, SUMMARY, stale_plan_check};
use super::cmd_milestone::ready;
use super::cmd_refactor::{PHASE, busy, with_source};
use super::cmd_verify::unanswered;
use super::commands_support::*;
use super::harness::{Project, project};

/// One writing command on a fixture: the project, the arguments, and a fragment of the diff its plan shows.
struct Case {
    command: &'static str,
    project: fn() -> Project,
    args: fn(&Project) -> Vec<String>,
    shows: &'static str,
}

fn strs(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

fn with_pause() -> Project {
    let p = twin();
    assert_eq!(gsd(&p, "pause", &["wait", "--phase", "02"]).code, 0);
    p
}

fn with_blocker() -> Project {
    let p = twin();
    assert_eq!(gsd(&p, "raise-blocker", &["open question"]).code, 0);
    p
}

fn with_proposal() -> Project {
    let p = with_source(&busy("add", 16));
    assert_eq!(gsd(&p, "propose-refactor", &[PHASE]).code, 0);
    p
}

fn planted() -> Project {
    let p = twin();
    plant(&p, "spec/plans.spec", PLANTED);
    p
}

fn unapproved() -> Project {
    let p = project("verify");
    p.edit(
        "spec/validation.spec",
        "  approval {\n    by \"ada\"\n    at \"2026-10-29T11:00:00Z\"\n  }\n",
        "",
    );
    unledger(&p, "v1.0/01/01-VALIDATION", "approval");
    p
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            command: "complete-plan",
            project: planted,
            args: |_| strs(&[SUMMARY, "--started", STARTED]),
            shows: "started",
        },
        Case {
            command: "answer-checkpoint",
            project: || project("execute"),
            args: |_| strs(&[CHECKPOINT, "--option", "flag"]),
            shows: "answer",
        },
        Case {
            command: "confirm-setup",
            project: || project("execute"),
            args: |_| strs(&[SETUP, "--by", "ada"]),
            shows: "done",
        },
        Case {
            command: "propose-refactor",
            project: || with_source(&busy("add", 16)),
            args: |_| strs(&[PHASE]),
            shows: "refactor_proposal RP1",
        },
        Case {
            command: "dispose-refactor",
            project: with_proposal,
            args: |_| strs(&["v1.0/01/RP1", "accepted", "ada"]),
            shows: "disposition",
        },
        Case {
            command: "record-window",
            project: || project("verify"),
            args: |_| strs(&["todo", "01", "A TODO remains"]),
            shows: "broken_window BW2",
        },
        Case {
            command: "waive-window",
            project: || project("verify"),
            args: |_| strs(&["BW1", "tracked elsewhere", "ada"]),
            shows: "waiver",
        },
        Case {
            command: "fix-window",
            project: || project("verify"),
            args: |_| strs(&["BW1"]),
            shows: "fixed_at",
        },
        Case {
            command: "pause",
            project: twin,
            args: |_| strs(&["wait", "--phase", "02"]),
            shows: "pause PS1",
        },
        Case {
            command: "resume",
            project: with_pause,
            args: |_| strs(&["v1.0/02/PS1"]),
            shows: "PS1",
        },
        Case {
            command: "raise-blocker",
            project: twin,
            args: |_| strs(&["open question"]),
            shows: "blocker BL1",
        },
        Case {
            command: "resolve-blocker",
            project: with_blocker,
            args: |_| strs(&["BL1", "decided"]),
            shows: "resolved",
        },
        Case {
            command: "phase",
            project: twin,
            args: |_| strs(&["--name", "Sharing"]),
            shows: "phase 04",
        },
        Case {
            command: "close-milestone",
            project: ready,
            args: |_| strs(&["v1.0"]),
            shows: "completion",
        },
        Case {
            command: "set-uat-result",
            project: unanswered,
            args: |_| strs(&["v1.0/01/01-UAT/U1", "pass"]),
            shows: "result",
        },
        Case {
            command: "approve-validation",
            project: unapproved,
            args: |_| strs(&["v1.0/01/01-VALIDATION", "ada"]),
            shows: "approval",
        },
        Case {
            command: "workstreams",
            project: twin,
            args: |_| strs(&["create", "--name", "billing", "--purpose", "Charge"]),
            shows: "workstream billing",
        },
    ]
}

/// The ops of a dry run: the diff its plan shows.
fn diff(run: &super::harness::Run) -> String {
    let all = json(run);
    assert_eq!(all["plan"]["dry_run"], true, "{all}");
    all["plan"]["diff"].as_str().unwrap_or_default().to_string()
}

#[test]
fn dry_run_and_gates() {
    for case in cases() {
        let p = (case.project)();
        let before = bytes(&p);
        let mut args = (case.args)(&p);
        args.extend(strs(&["--dry-run", "--format", "json"]));
        let all: Vec<&str> = args.iter().map(String::as_str).collect();
        let run = gsd(&p, case.command, &all);
        assert_eq!(run.code, 0, "{}: {}", case.command, run.all());
        let shown = diff(&run);
        assert!(
            shown.contains(case.shows),
            "{}: '{}' not in the plan:\n{shown}",
            case.command,
            case.shows
        );
        unchanged(&p, &before);
    }
    // record-verdict is a writing command too: its dry run shows the verdict.
    let p = twin();
    let before = bytes(&p);
    let run = p.record(PLAN, "W311", "passed", &["--dry-run", "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(diff(&run).contains("verdict"), "{}", run.all());
    unchanged(&p, &before);
}

#[test]
fn a_failing_gate_refuses_and_an_off_setting_allows() {
    // start-plan: W318 under strict, E516 under plus; a setting off never refuses.
    let p = twin();
    stale_plan_check(&p);
    strict(&p);
    let kept = bytes(&p);
    refused(
        &gsd(&p, "start-plan", &[PLAN]),
        "gate_failed",
        &["W318", PLAN],
    );
    unchanged(&p, &kept);
    plus(&p);
    refused(&gsd(&p, "start-plan", &[PLAN]), "gate_failed", &["E516"]);
    p.settings(json!({ "profile": "gsd-plus", "workflow.plan_check": false, "specforge.stale_verdict_gate": false }));
    assert_eq!(gsd(&p, "start-plan", &[PLAN]).code, 0);

    // complete-plan: W404 under strict with tdd mode on.
    let p = planted();
    p.edit(
        "spec/plans.spec",
        "plan 02-01 \"Save and load tasks.json\" {\n  phase            v1.0/02\n  type             execute",
        "plan 02-01 \"Save and load tasks.json\" {\n  phase            v1.0/02\n  type             tdd",
    );
    p.settings(json!({ "profile": "gsd-strict", "workflow.tdd_mode": true }));
    let kept = bytes(&p);
    refused(
        &gsd(&p, "complete-plan", &[SUMMARY, "--started", STARTED]),
        "gate_failed",
        &["W404", PLAN],
    );
    unchanged(&p, &kept);
    p.settings(json!({ "profile": "gsd-strict", "workflow.tdd_mode": false }));
    assert_eq!(
        gsd(
            &p,
            "complete-plan",
            &[SUMMARY, "--started", STARTED, "--dry-run"]
        )
        .code,
        0
    );

    // close-milestone: E106 and E511 on a milestone nobody audited or verified.
    let p = twin();
    let kept = bytes(&p);
    let run = gsd(&p, "close-milestone", &["v1.0"]);
    refused(&run, "gate_failed", &["E106", "E511"]);
    unchanged(&p, &kept);
}

#[test]
fn every_command_refuses_an_unknown_entity_with_its_name() {
    let unknown: [(&str, Vec<&str>); 15] = [
        ("start-plan", vec!["NOPE"]),
        ("complete-plan", vec!["NOPE", "--started", STARTED]),
        ("answer-checkpoint", vec!["NOPE", "--option", "x"]),
        ("confirm-setup", vec!["NOPE"]),
        ("propose-refactor", vec!["NOPE"]),
        ("dispose-refactor", vec!["NOPE", "accepted", "ada"]),
        ("waive-window", vec!["NOPE", "why", "ada"]),
        ("fix-window", vec!["NOPE"]),
        ("resume", vec!["NOPE"]),
        ("resolve-blocker", vec!["NOPE", "done"]),
        ("close-milestone", vec!["NOPE"]),
        ("set-uat-result", vec!["NOPE", "pass"]),
        ("approve-validation", vec!["NOPE", "ada"]),
        ("workstreams", vec!["switch", "--name", "NOPE"]),
        ("phase", vec!["--edit", "NOPE", "--goal", "x"]),
    ];
    for (command, args) in unknown {
        let p = twin();
        let before = bytes(&p);
        let run = gsd(&p, command, &args);
        assert_ne!(run.code, 0, "{command}: {}", run.all());
        assert!(
            run.all().contains("NOPE"),
            "{command} does not name the entity: {}",
            run.all()
        );
        unchanged(&p, &before);
    }
}
