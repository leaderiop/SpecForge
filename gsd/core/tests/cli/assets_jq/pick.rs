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

/// T4: the graph reads of `specforge query`, one line of each form: ids of a kind, a count, the first node's field,
/// a computed boolean, a status with its default and its route, a list joined, a string built of two fields.
#[test]
fn a_graph_read_is_picked() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let w = "gsd-core/workflows/";
    let phase = "PHASE_ID=v1.0/01";
    // the plans of the query's answer, as jq's `.nodes[]|select(.kind=="plan")|.id` read them (depth 2 reaches
    // phase 02's plan too, through its dependency)
    let answer: serde_json::Value = serde_json::from_str(
        &p.run(&[
            "query", "v1.0/01", "--depth", "2", "--kinds", "plan", "--format", "json",
        ])
        .stdout,
    )
    .expect("the query's JSON");
    let plans: String = answer["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|n| n["kind"] == "plan")
        .map(|n| format!("{}\n", n["id"].as_str().expect("an id")))
        .collect();
    assert!(
        plans.starts_with("v1.0/01/01-01\nv1.0/01/01-02\n"),
        "{plans}"
    );
    let review = line(&asset(&format!("{w}review.md")), "for PLAN_REF in $(").to_string();
    assert_eq!(
        run(
            &sh,
            &p,
            &format!("{phase}\n{review} echo \"$PLAN_REF\"; done")
        )
        .0,
        plans
    );
    let cases: &[(&str, &str, &str, &str, &str)] = &[
        ("plan-phase.md", phase, "DISK_PLANS=$(", "DISK_PLANS", "[2]"),
        (
            "plan-phase.md",
            "PHASE_ID=v1.0/03",
            "DISK_PLANS=$(",
            "DISK_PLANS",
            "[0]",
        ),
        (
            "execute-plan.md",
            "PLAN_ID=v1.0/01/01-01",
            "TASK_COUNT=$(",
            "TASK_COUNT",
            "[2]",
        ),
        // depth 2, as above: phase 02's plan too
        (
            "next.md",
            "PHASE_NUM=01",
            "PLAN_COUNT=$(",
            "PLAN_COUNT",
            "[3]",
        ),
        (
            "execute-phase/steps/verify-phase-goal.md",
            phase,
            "PHASE_GOAL=$(",
            "PHASE_GOAL",
            "[Tasks can be added and completed]",
        ),
        (
            "verify-work.md",
            phase,
            "MVP_MODE=$(",
            "MVP_MODE",
            "[false]",
        ),
        (
            "plan-phase.md",
            "PHASE_ID=v1.0/01; MVP_FLAG_ARG=--mvp",
            "MVP_MODE=$(",
            "MVP_MODE",
            "[true]",
        ),
        ("ship.md", phase, "STATUS=$(", "STATUS", "[passed]"),
        (
            "ship.md",
            "PHASE_ID=v1.0/02",
            "STATUS=$(",
            "STATUS",
            "[pending]",
        ),
        (
            "execute-phase.md",
            phase,
            "VERIFY_ROUTE=$(",
            "VERIFY_ROUTE",
            "[complete]",
        ),
        (
            "execute-phase.md",
            "PHASE_ID=v1.0/02",
            "VERIFY_ROUTE=$(",
            "VERIFY_ROUTE",
            "[verify]",
        ),
        (
            "plan-phase.md",
            phase,
            "PHASE_REQ_IDS=$(",
            "PHASE_REQ_IDS",
            "[TASK-01,TASK-02]",
        ),
        (
            "plan-phase.md",
            phase,
            "PHASE_DESC=",
            "PHASE_DESC",
            "[Goal: Tasks can be added and completed\nSuccess criteria: A user can add a task with a title; A user can mark a task complete by its id]",
        ),
        ("plan-phase.md", phase, "SPEC_PATH=$(", "SPEC_PATH", "[]"),
        (
            "undo.md",
            "UNDO_MILESTONE=v1.0; TARGET_PHASE=01",
            "PHASE_DIR=$(",
            "PHASE_DIR",
            "[v1.0/01]",
        ),
        (
            "undo.md",
            "UNDO_MILESTONE=v1.0; TARGET_PHASE=09",
            "PHASE_DIR=$(",
            "PHASE_DIR",
            "[]",
        ),
    ];
    for (file, setup, start, var, want) in cases {
        let path = format!("{w}{file}");
        assert_eq!(
            value(&sh, &p, &path, setup, start, var),
            *want,
            "{file}: {start}"
        );
    }
    // a phase with two verifications, both passed: a status read takes the first, as jq's `[…][0]` did
    let twice = taskboard();
    let mut verify = twice.read("spec/verify.spec");
    verify.push_str(
        "\nverification 01-REVERIFICATION \"Phase 01 re-verification\" {\n  phase v1.0/01\n}\n",
    );
    twice.write("spec/verify.spec", &verify);
    let first: &[(&str, &str, &str, &str, &str)] = &[
        (
            "autonomous.md",
            "PHASE_NUM=v1.0/01",
            "VERIFY_STATUS=$(",
            "VERIFY_STATUS",
            "[passed]",
        ),
        (
            "execute-phase.md",
            phase,
            "VERIFY_STATUS=$(",
            "VERIFY_STATUS",
            "[passed]",
        ),
        (
            "execute-phase.md",
            phase,
            "VERIFY_ROUTE=$(",
            "VERIFY_ROUTE",
            "[complete]",
        ),
        ("ship.md", phase, "STATUS=$(specforge", "STATUS", "[passed]"),
        (
            "transition.md",
            phase,
            "VERIFY_STATUS=$(",
            "VERIFY_STATUS",
            "[passed]",
        ),
        (
            "quick/steps/quick-verification.md",
            "quick_id=v1.0/01",
            "STATUS=$(",
            "STATUS",
            "[passed]",
        ),
        (
            "quick-batch/steps/verification-wave.md",
            "quick_id=v1.0/01",
            "STATUS=$(",
            "STATUS",
            "[passed]",
        ),
        (
            "progress.md",
            "PHASE_DIR=v1.0/01",
            "VERIFICATION_STATUS=$(",
            "VERIFICATION_STATUS",
            "[passed]",
        ),
    ];
    for (file, setup, start, var, want) in first {
        let text = asset(&format!("{w}{file}"));
        let reads: Vec<&str> = text
            .lines()
            .map(str::trim_start)
            .filter(|l| l.starts_with(start))
            .collect();
        assert!(!reads.is_empty(), "{file}: no line starts with {start}");
        for code in reads {
            let got = run(
                &sh,
                &twice,
                &format!("{setup}\n{code}\nprintf '[%s]' \"${var}\""),
            )
            .0;
            assert_eq!(got, *want, "{file}: {code}");
        }
    }
}

/// T4: a read whose failure the next line checks keeps the command's exit status (no pipe after `--pick`).
#[test]
fn a_graph_read_keeps_its_exit_status() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let tdd = asset("gsd-core/workflows/execute-phase/steps/tdd-applicability-resolution.md");
    let read = line(&tdd, "TDD_APPLICABLE_RAW=$(").to_string();
    let script = |plan: &str| format!("PLAN_ID={plan}\n{read}\necho \"$? [$TDD_APPLICABLE_RAW]\"");
    assert_eq!(run(&sh, &p, &script("v1.0/01/01-01")).0, "0 [false]\n");
    assert!(run(&sh, &p, &script("nope")).0.starts_with("1 "));
    let ship = line(&asset("gsd-core/workflows/ship.md"), "STATUS=$(").to_string();
    let (out, code) = run(&sh, &p, &format!("PHASE_ID=nope\n{ship}\necho after"));
    assert_eq!((out.as_str(), code), ("", 1));
}
