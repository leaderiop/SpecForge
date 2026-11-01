//! G4 (plan 04): the lines that read a SpecForge answer with `--pick`, run as the asset holds them under bash with
//! no jq on the `PATH`, on the taskboard fixture (`gsd/parity/fixtures/taskboard`): phases 01 (2 of 2 plans
//! summarised), 02 (0 of 1), 02.1 and 03 (no plan), 67% of the plans summarised, no workstream.

use super::{Shell, asset, block, line};
use crate::harness::{Project, taskboard};

/// `script` in the project's root: `(stdout, exit code)`.
fn run(sh: &Shell, p: &Project, script: &str) -> (String, i32) {
    let (out, _, code) = sh.run_in(p.root(), script);
    (out, code)
}

/// `setup`, the line of `path` that starts with `start`, then `printf '[%s]' "$<var>"`.
fn value(sh: &Shell, p: &Project, path: &str, setup: &str, start: &str, var: &str) -> String {
    let code = line(&asset(path), start).to_string();
    run(sh, p, &format!("{setup}\n{code}\nprintf '[%s]' \"${var}\"")).0
}

/// T3: a field of a `specforge gsd` answer.
#[test]
fn a_field_of_a_gsd_answer_is_picked() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let started = value(
        &sh,
        &p,
        "agents/gsd-executor.md",
        "PHASE=v1.0/01/01; PLAN=01",
        "STARTED=$(",
        "STARTED",
    );
    assert!(
        started.starts_with("[20") && started.ends_with("Z]"),
        "{started}"
    );
    // a refused start stops the block, as `jq -e` did
    let code = line(&asset("agents/gsd-executor.md"), "STARTED=$(").to_string();
    let (out, status) = run(&sh, &p, &format!("PHASE=nope; PLAN=01\n{code}\necho after"));
    assert_eq!((out.as_str(), status), ("", 1));
    let cm = "gsd-core/workflows/complete-milestone.md";
    assert_eq!(
        value(&sh, &p, cm, "", "GSD_ACTIVE_WS=$(", "GSD_ACTIVE_WS"),
        "[]"
    );
    let flat = line(&asset(cm), "_gsd_flat_mode() {").to_string();
    assert_eq!(
        run(
            &sh,
            &p,
            &format!("GSD_WS=\n{flat}\n_gsd_flat_mode && echo flat")
        )
        .0,
        "flat\n"
    );
    let next = asset("gsd-core/workflows/next.md");
    assert_eq!(
        value(
            &sh,
            &p,
            "gsd-core/workflows/next.md",
            "",
            "ROADMAP_SCOPE=$(",
            "ROADMAP_SCOPE"
        ),
        "[complete]"
    );
    let phases = line(&next, "for PHASE_NUM in $(").to_string();
    assert_eq!(
        run(&sh, &p, &format!("{phases} echo \"$PHASE_NUM\"; done")).0,
        "01\n02\n02.1\n03\n"
    );
    let progress = "gsd-core/workflows/progress.md";
    assert_eq!(
        value(&sh, &p, progress, "", "PROGRESS_BAR=", "PROGRESS_BAR"),
        "[[67%]]"
    );
    // the first phase with more plans than summaries, read by index
    let scan = block(&asset(progress), "INCOMPLETE_PHASE=\"\"", "  done");
    assert_eq!(
        run(
            &sh,
            &p,
            &format!("ROADMAP=x\n{scan}\nfi\nprintf '[%s]' \"$INCOMPLETE_PHASE\"")
        )
        .0,
        "[02]"
    );
}

/// T3: pause-work's phase in progress is the number at the index of the first `In Progress` status.
#[test]
fn the_phase_in_progress_is_read_by_index() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let path = "gsd-core/workflows/pause-work.md";
    // no phase of the taskboard is in progress
    assert_eq!(value(&sh, &p, path, "", "phase=$(", "phase"), "[]");
    // the shell's part, on a stand-in for the command: the second phase is in progress
    let stub = r#"specforge() { case "$*" in *status*) printf 'Complete\nIn Progress\nIn Progress\n' ;; *'[1].number'*) echo 02 ;; esac; }"#;
    assert_eq!(value(&sh, &p, path, stub, "phase=$(", "phase"), "[02]");
}

/// T3: review-backlog's promote prints the new phase's last segment.
#[test]
fn a_promoted_phase_number_is_picked() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = crate::cmd_phase::backlogged();
    let code =
        line(&asset("commands/gsd/review-backlog.md"), "NEW_NUM=$(").replace("<id>", "999.1");
    assert_eq!(
        run(
            &sh,
            &p,
            &format!("MILESTONE=v1.0\n{code}\nprintf '[%s]' \"$NEW_NUM\"")
        )
        .0,
        "[13]"
    );
}
