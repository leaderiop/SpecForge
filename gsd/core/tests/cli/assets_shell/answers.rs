//! G4 (plan 05 T4): the last lines that ran jq, run as the asset holds them under bash with no jq on the `PATH`. A
//! SpecForge answer is read with `--pick`; what the shell cannot check (a reviewer's answer, a stub) is left to the
//! agent or written as the literal it always was. The taskboard fixture (`gsd/parity/fixtures/taskboard`): phase 01
//! has the UAT tests U1 (pass), U2 (issue) and U3 (pass) and a verification; plan 02-01 has no summary.

use super::{Shell, asset, block, line, q};
use crate::harness::{Project, taskboard};

/// `setup`, then the lines of `path` that start with each of `starts`, in order, then `show`, in the project's root:
/// `(stdout, stderr)`.
fn run(
    sh: &Shell,
    p: &Project,
    path: &str,
    setup: &str,
    starts: &[&str],
    show: &str,
) -> (String, String) {
    let text = asset(path);
    let code: Vec<&str> = starts.iter().map(|s| line(&text, s)).collect();
    let (out, err, _) = sh.run_in(p.root(), &format!("{setup}\n{}\n{show}", code.join("\n")));
    assert!(!err.contains("jq"), "{path}: {err}");
    (out, err)
}

fn shell() -> Option<Shell> {
    Shell::without_jq()
}

/// Port bug: `check --format json | jq '[.diagnostics[]? …]'` read a field of the bare diagnostic list, so the guard
/// was always true; and `gsd/W205` is the package-legitimacy verdict, not the user story. The guard reads the
/// phase's goal against the canonical form, as upstream's `user-story.validate --story "$PHASE_GOAL"` did.
#[test]
fn the_user_story_guard_reads_the_phase_goal() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    let path = "agents/gsd-verifier.md";
    let guard = |phase: &str| {
        run(
            &sh,
            &p,
            path,
            &format!("PHASE_ID={phase}"),
            &["USER_STORY_VALID=$("],
            "printf '[%s]' \"$USER_STORY_VALID\"",
        )
        .0
    };
    assert_eq!(
        guard("v1.0/01"),
        "[false]",
        "\"Tasks can be added and completed\" is no user story"
    );
    let spec = p.root().join("spec/v1.0.spec");
    let text = std::fs::read_to_string(&spec).expect("the roadmap");
    let story = "As a user, I want to save my tasks, so that none is lost.";
    std::fs::write(
        &spec,
        text.replace(
            "\"Tasks are saved to disk and read back\"",
            &format!("\"{story}\""),
        ),
    )
    .expect("a user story goal");
    assert_eq!(guard("v1.0/02"), "[true]");
}

/// Port bug: `grep -v "${PHASE_NUMBER}"` with an empty phase number dropped every prior verification, so the gate ran
/// nothing. With none named, every verification is prior: this phase has none yet (the gate runs before it).
#[test]
fn the_regression_gate_keeps_every_verification_when_no_phase_is_named() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    let path = "gsd-core/workflows/execute-phase/steps/regression-gate.md";
    let prior = |number: &str| {
        run(
            &sh,
            &p,
            path,
            &format!("MILESTONE=v1.0\nPHASE_NUMBER={number}"),
            &["PRIOR_VERIFICATIONS=$("],
            "printf '[%s]' \"$PRIOR_VERIFICATIONS\"",
        )
        .0
    };
    assert_eq!(prior(""), "[v1.0/01/01-VERIFICATION]");
    assert_eq!(prior("02"), "[v1.0/01/01-VERIFICATION]");
    assert_eq!(prior("01"), "[]");
}

#[test]
fn a_setting_is_picked_by_its_name() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    let path = "agents/gsd-executor.md";
    let starts = ["AUTO_CFG=$(", "HUMAN_VERIFY_MODE=$("];
    let show = "printf '[%s][%s]' \"$AUTO_CFG\" \"$HUMAN_VERIFY_MODE\"";
    assert_eq!(
        run(&sh, &p, path, "", &starts, show).0,
        "[false][end-of-phase]"
    );
    p.settings(serde_json::json!({"workflow.auto_advance": true, "workflow.human_verify_mode": "mid-flight"}));
    assert_eq!(
        run(&sh, &p, path, "", &starts, show).0,
        "[true][mid-flight]"
    );
}

#[test]
fn the_graph_lists_are_picked_and_filtered_in_the_shell() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    let (out, _) = run(
        &sh,
        &p,
        "commands/gsd/review-backlog.md",
        "",
        &["_BACKLOG=$(", "if [ -n \"$_BACKLOG\" ]"],
        "",
    );
    assert_eq!(out, "No backlog items found\n");
    let (out, err) = run(
        &sh,
        &p,
        "gsd-core/workflows/onboard.md",
        "",
        &["ENTITIES=$("],
        "printf '%s\\n' \"$ENTITIES\"",
    );
    assert!(out.lines().any(|l| l == "project taskboard"), "{out}{err}");
    assert!(out.lines().any(|l| l == "phase v1.0/02.1"), "{out}");
    let (out, _) = run(
        &sh,
        &p,
        "gsd-core/workflows/resume-project.md",
        "MILESTONE=v1.0",
        &["for plan in $(specforge"],
        "echo \"[$plan]\"; done",
    );
    // the loop body's own lines are not run: the loop prints each plan with no summary
    assert_eq!(out, "[v1.0/02/02-01]\n");
    let (out, _) = run(
        &sh,
        &p,
        "gsd-core/workflows/progress.md",
        "CURRENT_PHASE=v1.0/01",
        &["for t in $(specforge query \"${CURRENT_PHASE}\""],
        "",
    );
    assert_eq!(out, "v1.0/01/01-UAT/U2\n");
    let (out, _) = run(
        &sh,
        &p,
        "gsd-core/workflows/progress.md",
        "MILESTONE=v1.0\nPHASE_ID=v1.0/01",
        &[
            "CURRENT_DEBT=$(",
            "ARCHIVED_DEBT=",
            "WINDOWS_STATUSES=$(",
            "WINDOWS_OPEN=$(",
            "WINDOWS_WAIVED=$(",
        ],
        "printf '[%s][%s][%s][%s]' \"$CURRENT_DEBT\" \"$ARCHIVED_DEBT\" \"$WINDOWS_OPEN\" \"$WINDOWS_WAIVED\"",
    );
    assert_eq!(out, "[1][0][0][0]");
    let (out, err) = run(
        &sh,
        &p,
        "gsd-core/workflows/ingest-docs.md",
        "",
        &["if [ -n \"$(specforge export --kinds project"],
        "printf '[%s]' \"$PROJECT_EXISTS\"",
    );
    assert_eq!(out, "[1]", "{err}");
    for (path, start) in [
        (
            "gsd-core/workflows/next.md",
            "[ -n \"$(specforge export --kinds pause",
        ),
        (
            "gsd-core/workflows/pause-work.md",
            "for s in $(specforge query \"$MILESTONE\"",
        ),
        (
            "gsd-core/workflows/progress/steps/forensic-audit.md",
            "specforge export --kinds pause,handoff",
        ),
        (
            "gsd-core/workflows/progress/steps/forensic-audit.md",
            "for ph in $(specforge query \"$MILESTONE\"",
        ),
        (
            "gsd-core/workflows/progress/steps/forensic-audit.md",
            "specforge export --kinds lesson",
        ),
    ] {
        let (out, err) = run(&sh, &p, path, "MILESTONE=v1.0", &[start], "echo \"[$?]\"");
        let want = if path.ends_with("next.md") {
            "[1]\n"
        } else {
            "[0]\n"
        };
        assert_eq!(out, want, "{path}: {err}");
    }
    let (out, err) = run(
        &sh,
        &p,
        "gsd-core/workflows/milestone-summary.md",
        "",
        &[],
        &format!(
            "{}; echo \"[$?]\"",
            inline(
                &asset("gsd-core/workflows/milestone-summary.md"),
                "specforge export --kinds milestone --pick '.nodes[fields.state=shipped].id'"
            )
        ),
    );
    assert_eq!(out, "[0]\n", "no milestone is shipped: {err}");
}

/// The inline-code run of `text` that starts with `start`.
fn inline(text: &str, start: &str) -> String {
    let at = text
        .find(&format!("`{start}"))
        .unwrap_or_else(|| panic!("no run starts with {start}"))
        + 1;
    let end = text[at..].find('`').expect("the run ends");
    text[at..at + end].to_string()
}

#[test]
fn an_inline_run_reads_the_graph_with_pick() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    for path in [
        "gsd-core/workflows/discuss-phase.md",
        "gsd-core/workflows/discuss-phase-assumptions.md",
        "gsd-core/workflows/discuss-phase-power.md",
        "gsd-core/workflows/discuss-phase/modes/power.md",
        "gsd-core/workflows/spec-phase.md",
    ] {
        let run = inline(&asset(path), "SPEC_ID=$(");
        let (out, err, _) = sh.run_in(
            p.root(),
            &format!("PHASE_ID=v1.0/02\n{run}\nprintf '[%s]' \"$HAS_SPEC\""),
        );
        assert_eq!(out, "[false]", "{path}: {err}");
    }
    for path in [
        "gsd-core/workflows/quick.md",
        "gsd-core/workflows/quick-batch/steps/planner-wave.md",
    ] {
        let run = inline(
            &asset(path),
            "[ -n \"$(specforge query \"${quick_id}\" --kinds task",
        );
        let (out, err, _) = sh.run_in(
            p.root(),
            &format!("quick_id=v1.0/01/01-01\n{run}; echo \"[$?]\""),
        );
        assert_eq!(out, "[0]\n", "{path}: {err}");
    }
}

#[test]
fn a_user_story_is_checked_in_the_shell() {
    let Some(sh) = shell() else { return };
    let text = asset("gsd-core/workflows/mvp-phase.md");
    let check = block(
        &text,
        "printf '%s' \"$USER_STORY\" | grep -Eq",
        "RE_PROMPT_USER_STORY=true",
    ) + "\nfi";
    for (story, want) in [
        (
            "As a user, I want to add a task, so that I remember it.",
            "[]",
        ),
        ("Tasks can be added", "[true]"),
    ] {
        let (out, err, _) = sh.run_in(
            sh.bin.path(),
            &format!(
                "USER_STORY={}\n{check}\nprintf '[%s]' \"$RE_PROMPT_USER_STORY\"",
                q(story)
            ),
        );
        assert_eq!(out, want, "{story}");
        assert_eq!(
            err.contains("Write the story as"),
            want == "[true]",
            "{err}"
        );
    }
}

#[test]
fn the_decision_coverage_gate_fails_on_a_listed_finding() {
    let Some(sh) = shell() else { return };
    let text = asset("gsd-core/workflows/plan-phase.md");
    let gate = block(&text, "[ -z \"$GATE_RESULT\" ] || {", "}");
    for (result, want) in [
        ("", "[0]"),
        (
            "decision D-01 is cited by no plan",
            "decision D-01 is cited by no plan\n[1]",
        ),
    ] {
        let (out, _, _) = sh.run_in(
            sh.bin.path(),
            &format!(
                "GATE_RESULT={}\n( {gate}\n); printf '[%s]' \"$?\"",
                q(result)
            ),
        );
        assert_eq!(out, want, "{result}");
    }
}

#[test]
fn the_default_reviewers_are_counted_from_the_setting() {
    let Some(sh) = shell() else { return };
    let text = asset("gsd-core/workflows/plan-review-convergence.md");
    let pick = block(
        &text,
        "if [ -z \"$REVIEWER_FLAGS\" ]; then",
        "REVIEWER_DISPLAY=\"${REVIEWER_FLAGS# }\"",
    );
    let script = format!("{pick}\nfi\nprintf '[%s][%s]' \"$REVIEWER_FLAGS\" \"$REVIEWER_DISPLAY\"");
    let (out, err, _) = sh.run_in(
        sh.bin.path(),
        &script.replace(
            "{{setting:review.default_reviewers|}}",
            r#"[\"codex\",\"claude\"]"#,
        ),
    );
    assert_eq!(out, "[][review.default_reviewers (codex, claude)]", "{err}");
    let (out, _, _) = sh.run_in(
        sh.bin.path(),
        &script.replace("{{setting:review.default_reviewers|}}", ""),
    );
    assert_eq!(
        out,
        "[--codex][--codex (default; configure review.default_reviewers to change)]"
    );
}

#[test]
fn the_pull_request_state_is_one_line_from_gh() {
    let Some(sh) = shell() else { return };
    let text = asset("gsd-core/workflows/ship.md");
    let read = [
        line(&text, "HEAD_OID="),
        line(&text, "STATUS=$(echo \"$PR_STATE\""),
        line(&text, "CHECKS="),
    ]
    .join("\n");
    for (state, want) in [
        ("abc123 BLOCKED 0", "[abc123][BLOCKED][0]"),
        ("- UNKNOWN 0", "[-][UNKNOWN][0]"),
    ] {
        let out = sh.run(&format!(
            "PR_STATE={}\n{read}\nprintf '[%s][%s][%s]' \"$HEAD_OID\" \"$STATUS\" \"$CHECKS\"",
            q(state)
        ));
        assert_eq!(out, want, "{state}");
    }
    assert!(line(&text, "PR_STATE=$(gh pr view").contains("--jq"));
}

#[test]
fn the_spec_root_is_read_from_the_manifest_file() {
    let Some(sh) = shell() else { return };
    let dir = tempfile::tempdir().expect("a dir");
    let read = line(&asset("gsd-core/workflows/pr-branch.md"), "SPEC_ROOT=$(").to_string();
    for (manifest, want) in [
        (
            Some("{\n  \"name\": \"x\",\n  \"spec_root\": \"specs\"\n}\n"),
            "[specs]",
        ),
        (Some("{\"name\": \"x\"}"), "[spec]"),
        (None, "[spec]"),
    ] {
        let _ = std::fs::remove_file(dir.path().join("specforge.json"));
        if let Some(m) = manifest {
            std::fs::write(dir.path().join("specforge.json"), m).expect("a manifest");
        }
        let (out, _, _) = sh.run_in(dir.path(), &format!("{read}\nprintf '[%s]' \"$SPEC_ROOT\""));
        assert_eq!(out, want, "{manifest:?}");
    }
}

#[test]
fn the_requirements_guard_refuses_a_placeholder_or_an_empty_list() {
    let Some(sh) = shell() else { return };
    let dir = tempfile::tempdir().expect("a dir");
    let reqs = dir.path().join("reqs.json");
    for path in [
        "gsd-core/workflows/spec-phase.md",
        "gsd-core/references/specless-probe-fallback.md",
    ] {
        let guard = format!(
            "{} echo bad; else echo ok; fi",
            line(&asset(path), "if grep -q '<replace:'")
        );
        for (json, want) in [
            (r#"[{"id":"R1","text":"a user signs in"}]"#, "ok"),
            ("[]", "bad"),
            (
                r#"[{"id":"R1","text":"<replace: requirement text from the SPEC>"}]"#,
                "bad",
            ),
        ] {
            std::fs::write(&reqs, json).expect("the requirements");
            let out = sh.run(&format!(
                "REQS_JSON={}\n{guard}",
                q(&reqs.display().to_string())
            ));
            assert_eq!(out, format!("{want}\n"), "{path}: {json}");
        }
    }
}
