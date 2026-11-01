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
/// was always true; and `gsd/W205` is the package-legitimacy verdict, not the user story. The guard hands the
/// phase's goal to `gsd user-story-validate`, upstream's `user-story validate --story "$PHASE_GOAL"`: a blank slot or a
/// story on two lines is no user story, which a `grep` for `.+` let through.
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
    for (story, want) in [
        (
            "As a user, I want to save my tasks, so that none is lost.",
            "[true]",
        ),
        (
            "As a  , I want to save my tasks, so that none is lost.",
            "[false]",
        ),
    ] {
        std::fs::write(
            &spec,
            text.replace(
                "\"Tasks are saved to disk and read back\"",
                &format!("\"{story}\""),
            ),
        )
        .expect("a goal");
        assert_eq!(guard("v1.0/02"), want, "{story}");
    }
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
    // plan 05 review R11: the phase is matched exactly, by its id's prefix, not as a substring of every id (`1` is in
    // `v1.0`)
    assert_eq!(prior("1"), "[v1.0/01/01-VERIFICATION]");
    assert_eq!(prior("0"), "[v1.0/01/01-VERIFICATION]");
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

/// Plan 05 review R4: the UAT debt is SpecForge's rule, `specforge gsd verification-debt`, not a `case` on each test's
/// `result`. A test with no result is pending and a skip without a `Deferred follow-up` reason is open: the
/// progress count missed the skip, and verify-work's gates missed both (a missing result matched no pattern).
#[test]
fn the_uat_debt_follows_the_uat_test_kinds_rule() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    let spec = p.root().join("spec/verify.spec");
    let mut text = std::fs::read_to_string(&spec).expect("the UAT file");
    text.push_str(concat!(
        "\nuat_test U4 \"Delete a task\" {\n  session  v1.0/01/01-UAT\n  expected \"`node src/cli.js rm 1` removes it\"\n}\n",
        "\nuat_test U5 \"Export\" {\n  session  v1.0/01/01-UAT\n  expected \"`node src/cli.js export` prints JSON\"\n",
        "  result   skipped\n  reason   \"no time today\"\n}\n",
    ));
    std::fs::write(&spec, text).expect("two open tests");
    let open = "v1.0/01/01-UAT/U2\nv1.0/01/01-UAT/U4\nv1.0/01/01-UAT/U5";
    let (out, err) = run(
        &sh,
        &p,
        "gsd-core/workflows/progress.md",
        "MILESTONE=v1.0",
        &["CURRENT_DEBT=$(", "ARCHIVED_DEBT="],
        "printf '[%s][%s]' \"$CURRENT_DEBT\" \"$ARCHIVED_DEBT\"",
    );
    assert_eq!(out, "[3][0]", "{err}");
    for start in ["UAT_BLOCKED=$(", "PHASE_COMPLETE_BLOCKERS=$("] {
        let name = start.trim_end_matches("=$(");
        let (out, err) = run(
            &sh,
            &p,
            "gsd-core/workflows/verify-work.md",
            "PHASE_ID=v1.0/01",
            &[start],
            &format!("printf '%s' \"${name}\""),
        );
        assert_eq!(out, open, "{start}: {err}");
    }
}

/// Plan 05 review R5: a lane's plan coverage is `specforge gsd plan-coverage`, over the phase's own plans, not a shell
/// loop over a manifest file. The manifest and the plan copies list the same plans: the phase's, where the
/// `--depth 2` query also reached the next phase's 02-01 (upstream globs the phase's own `*-PLAN.md`).
#[test]
fn the_plan_coverage_is_specforges() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    let dir = tempfile::tempdir().expect("a run dir");
    std::fs::write(
        dir.path().join("gsd-review-s.md"),
        "Reviewed 01-01; T-01-02 is a threat id, not the plan\n",
    )
    .expect("a review");
    let text = asset("gsd-core/workflows/review.md");
    let lines: Vec<&str> = text.lines().collect();
    // the lanes' loop that reads each report: the last `for SLUG` before the line naming the report file
    let report = lines
        .iter()
        .position(|l| {
            l.trim_start()
                .starts_with("REVIEW_FILE=\"$RUN_DIR/gsd-review-$SLUG.md\"")
        })
        .expect("the report's line");
    let from = lines[..report]
        .iter()
        .rposition(|l| l.starts_with("for SLUG in "))
        .expect("the lanes' loop");
    let to = from
        + lines[from..]
            .iter()
            .position(|l| *l == "done")
            .expect("its end");
    let setup = format!(
        "RUN_DIR={}\nPHASE_ID=v1.0/01\nDISPATCH_SLUGS=s",
        q(&dir.path().display().to_string())
    );
    let (out, err, _) = sh.run_in(
        p.root(),
        &format!(
            "{setup}\n{}\ncat \"$RUN_DIR/.plan-coverage-s.json\"",
            lines[from..=to].join("\n")
        ),
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&out).ok(),
        Some(serde_json::json!({"complete": false, "missing_ids": ["01-02"], "total": 2})),
        "{out}{err}"
    );
    let (out, err) = run(
        &sh,
        &p,
        "gsd-core/workflows/review.md",
        &setup,
        &[],
        &format!(
            "{}\n{}\nprintf '[%s]%s' \"$PLAN_COUNT\" \"$PLAN_ID_BULLETS\"; for f in \"$RUN_DIR\"/gsd-review-plan-*; do echo \"${{f##*/}}\"; done",
            block(&text, "for PLAN_REF in $(specforge", "done"),
            block(&text, "PLAN_COUNT=0", "done"),
        ),
    );
    assert_eq!(
        out, "[2]- 01-01\n- 01-02\ngsd-review-plan-01-01.md\ngsd-review-plan-01-02.md\n",
        "{err}"
    );
}

/// Plan 05 review R10: quick-batch's create check is the list's own (`QB_CREATE_RC=$?` in a block of its own was
/// always 0, so one item went on), and quick's worktree manifest is tab-separated like the wave's, which it falls
/// back to, not a `{"worktrees":[]}` file.
#[test]
fn the_quick_batch_list_and_the_quick_manifest_are_lines() {
    let Some(sh) = shell() else { return };
    let dir = tempfile::tempdir().expect("a dir");
    let text = asset("gsd-core/workflows/quick-batch/steps/batch-init.md");
    let create = line(&text, "QB_ITEMS=$(");
    let rc = text
        .lines()
        .find(|l| l.contains("QB_CREATE_RC="))
        .expect("the create check");
    for (items, want) in [
        ("- add a task\n", "[1]"),
        ("- add a task\n- list them\n", "[0]"),
    ] {
        let file = dir.path().join("tasks.md");
        std::fs::write(&file, items).expect("a task list");
        let out = sh.run(&format!(
            // `true`: the RC line is a block of its own, so the list's exit status is gone by then
            "TASK_FILE={}\n{create}\ntrue\n{rc}\nprintf '[%s]' \"$QB_CREATE_RC\"",
            q(&file.display().to_string())
        ));
        assert_eq!(out, want, "{items}");
    }
    let text = asset("gsd-core/workflows/quick.md");
    let out = sh.run(&format!(
        "TMPDIR={}\n{}\n{}\nprintf '%s|%s' \"${{QUICK_WORKTREE_MANIFEST##*.}}\" \"$(cat \"$QUICK_WORKTREE_MANIFEST\")\"",
        q(&dir.path().display().to_string()),
        line(&text, "QUICK_WORKTREE_MANIFEST=$(mktemp"),
        text.lines()
            .map(str::trim_start)
            .find(|l| l.contains("> \"$QUICK_WORKTREE_MANIFEST\""))
            .expect("the manifest's first write"),
    ));
    assert_eq!(out, "tsv|");
}

/// Plan 05 review R12: `/gsd-pr-branch` holds no stub that stands for a sub-repository answer (`SUB_REPOS_JSON="[]"`,
/// `RESULT=$(false)`, `REMOTE_SLUG=`): a sub-repository is another SpecForge project, so there is none, and the
/// sub-repository step reports a failure without one.
#[test]
fn the_pr_branch_holds_no_sub_repository_stub() {
    let Some(sh) = shell() else { return };
    let text = asset("gsd-core/workflows/pr-branch.md");
    for stub in ["SUB_REPOS_JSON=\"[]\"", "RESULT=$(false)", "REMOTE_SLUG="] {
        assert!(
            !text.lines().any(|l| l.trim_start().starts_with(stub)),
            "{stub}"
        );
    }
    let step = block(&text, "REPO_SAFE=", "SUBREPO_EXIT=");
    let out = sh.run(&format!(
        "REPO_REL=a\nCURRENT_BRANCH=main\n{step}\nprintf '[%s]' \"$SUBREPO_EXIT\""
    ));
    assert_eq!(out, "[1]");
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
        &["specforge gsd verification-debt --phase \"${CURRENT_PHASE}\""],
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
            "while IFS=$'\\x1f' read -r s line",
        ),
        (
            "gsd-core/workflows/progress/steps/forensic-audit.md",
            "specforge export --kinds pause,handoff",
        ),
        (
            "gsd-core/workflows/progress/steps/forensic-audit.md",
            "while IFS=$'\\x1f' read -r ph goal",
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

/// `/gsd-mvp-phase` checks the story it built with `gsd user-story-validate` and prints the command's errors, so a
/// blank slot (the three answers were only checked for being empty) is re-prompted.
#[test]
fn a_user_story_is_checked_by_specforge() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    let text = asset("gsd-core/workflows/mvp-phase.md");
    let check = block(&text, "USER_STORY_RESULT=$(", "RE_PROMPT_USER_STORY=true") + "\nfi";
    for (story, want, error) in [
        (
            "As a user, I want to add a task, so that I remember it.",
            "[]",
            "",
        ),
        (
            "Tasks can be added",
            "[true]",
            "Story must start with \"As a [user role],\"",
        ),
        (
            "As a user, I want to  , so that I remember it.",
            "[true]",
            "capability must be non-empty",
        ),
    ] {
        let (out, err, _) = sh.run_in(
            p.root(),
            &format!(
                "USER_STORY={}\n{check}\nprintf '[%s]' \"$RE_PROMPT_USER_STORY\"",
                q(story)
            ),
        );
        assert_eq!(out, want, "{story}: {err}");
        assert!(err.contains(error), "{story}: {err}");
    }
}

/// The MVP UAT framing reads the phase goal with the verifier's own check: a lowercase story without `to` or a final
/// period, which its `grep -qiE` took, is refused.
#[test]
fn the_mvp_uat_framing_checks_the_goal_with_specforge() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    let spec = p.root().join("spec/v1.0.spec");
    let text = std::fs::read_to_string(&spec).expect("the roadmap");
    for (story, want) in [
        (
            "As a user, I want to save my tasks, so that none is lost.",
            "[true]",
        ),
        ("as a user, i want saving so that none is lost", "[false]"),
    ] {
        std::fs::write(
            &spec,
            text.replace(
                "\"Tasks are saved to disk and read back\"",
                &format!("\"{story}\""),
            ),
        )
        .expect("a goal");
        let (out, _) = run(
            &sh,
            &p,
            "gsd-core/workflows/verify-work/steps/mvp-uat-framing.md",
            "PHASE_ID=v1.0/02",
            &["PHASE_GOAL=$(", "USER_STORY_VALID=$("],
            "printf '[%s]' \"$USER_STORY_VALID\"",
        );
        assert_eq!(out, want, "{story}");
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

/// Plan 05 review R8: the edge probe's requirements guard is `specforge gsd edge-probe-requirements`, upstream's rule
/// (a non-empty array whose every item has a string `text` that is not blank and holds no `<replace:` placeholder),
/// not two `grep`s: a blank text, a bare object, a `null` item or a non-string text passed them.
#[test]
fn the_requirements_guard_refuses_a_placeholder_or_an_empty_list() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    let dir = tempfile::tempdir().expect("a dir");
    let reqs = dir.path().join("reqs.json");
    for path in [
        "gsd-core/workflows/spec-phase.md",
        "gsd-core/references/specless-probe-fallback.md",
    ] {
        let text = asset(path);
        let guard = text
            .lines()
            .find(|l| l.starts_with("if ") && l.contains("$REQS_JSON") && !l.contains("COVERAGE"))
            .expect("the guard");
        for (json, want) in [
            (r#"[{"id":"R1","text":"a user signs in"}]"#, "ok"),
            (
                r#"[{"id":"R1","text":"a user signs in"},{"id":"R2","text":"they sign out"}]"#,
                "ok",
            ),
            ("[]", "bad"),
            (
                r#"[{"id":"R1","text":"<replace: requirement text from the SPEC>"}]"#,
                "bad",
            ),
            (r#"[{"text":"  "}]"#, "bad"),
            (r#"{"text":"a user signs in"}"#, "bad"),
            (r#"[null]"#, "bad"),
            (r#"[{"text":1}]"#, "bad"),
            (r#"[{"id":"R1","text":"a"},{"id":"R2"}]"#, "bad"),
            ("not json", "bad"),
        ] {
            std::fs::write(&reqs, json).expect("the requirements");
            let (out, err, _) = sh.run_in(
                p.root(),
                &format!(
                    "REQS_JSON={}\n{guard} echo bad; else echo ok; fi",
                    q(&reqs.display().to_string())
                ),
            );
            assert_eq!(out, format!("{want}\n"), "{path}: {json}: {err}");
        }
    }
}

/// R2: the text scans read one `id<0x1f>text` row per entity (`--fields`) and match in a `case`: a deferred goal and a
/// placeholder one-liner are named by their entity's id, the others not.
#[test]
fn the_text_scans_read_one_row_per_entity() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    let scan = |path: &str, start: &str| run(&sh, &p, path, "MILESTONE=v1.0", &[start], "").0;
    let deferred = (
        "gsd-core/workflows/progress/steps/forensic-audit.md",
        "while IFS=$'\\x1f' read -r ph goal",
    );
    let placeholder = (
        "gsd-core/workflows/pause-work.md",
        "while IFS=$'\\x1f' read -r s line",
    );
    assert_eq!(scan(deferred.0, deferred.1), "");
    assert_eq!(scan(placeholder.0, placeholder.1), "");
    let edit = |file: &str, from: &str, to: &str| {
        let path = p.root().join(file);
        let text = std::fs::read_to_string(&path).expect("the spec");
        assert!(text.contains(from), "{from}");
        std::fs::write(&path, text.replace(from, to)).expect("written");
    };
    edit(
        "spec/v1.0.spec",
        "\"Open tasks are listed and filtered\"",
        "\"Open tasks are listed;\\tfiltering is deferred to Phase 9\"",
    );
    edit(
        "spec/plans.spec",
        "\"`taskboard add` and `taskboard done` over a board whose tasks complete by id.\"",
        "\"TBD\"",
    );
    let phases = scan(deferred.0, deferred.1);
    assert_eq!(phases.lines().count(), 1, "{phases}");
    assert!(phases.starts_with("v1.0/"), "{phases}");
    let summaries = scan(placeholder.0, placeholder.1);
    assert_eq!(summaries.lines().count(), 1, "{summaries}");
    assert!(summaries.contains("-SUMMARY"), "{summaries}");
}

/// Re-review of plan 05, item 4: a phase's plan and summary counts are its own, read by its id, not the progress
/// report's phase of the same number in the current milestone. Here a later milestone restarts its numbers
/// (`--reset-phase-numbers`): v2.0/01 has two plans and no summary, v1.0/01 two plans with their summaries.
#[test]
fn a_phases_counts_are_read_by_its_own_id() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    p.write(
        "spec/v2.0.spec",
        "milestone v2.0 \"Taskboard 2.0\" {\n  name \"Taskboard 2.0\"\n}\n\n\
         phase 01 \"Sync\" {\n  milestone v2.0\n  name \"Sync\"\n  slug \"sync\"\n  goal \"Tasks sync\"\n}\n\n\
         plan 01-01 \"Sync model\" {\n  phase v2.0/01\n  type execute\n  objective \"Sync\"\n}\n\n\
         plan 01-02 \"Sync command\" {\n  phase v2.0/01\n  type execute\n  objective \"Sync\"\n}\n",
    );
    for (path, starts, show) in [
        (
            "gsd-core/workflows/execute-plan.md",
            &["PLAN_COUNT=$(", "SUMMARY_COUNT=$("][..],
            "printf '%s/%s' \"$SUMMARY_COUNT\" \"$PLAN_COUNT\"",
        ),
        (
            "gsd-core/workflows/progress.md",
            &["X=$(", "Y=$("][..],
            "printf '%s/%s' \"$Y\" \"$X\"",
        ),
        (
            "gsd-core/workflows/plan-review-convergence.md",
            &["PLAN_COUNT=$("][..],
            "printf '?/%s' \"$PLAN_COUNT\"",
        ),
    ] {
        for (phase, want) in [("v2.0/01", "0/2"), ("v1.0/01", "2/2"), ("v1.0/02", "0/1")] {
            let want = if path.contains("convergence") {
                format!("?/{}", &want[2..])
            } else {
                want.to_string()
            };
            let setup = format!("PHASE_ID={phase}\nCURRENT_PHASE={phase}");
            let (out, err) = run(&sh, &p, path, &setup, starts, show);
            assert_eq!(out, want, "{path} {phase}: {err}");
        }
    }
}

/// Re-review of plan 05, item 6: the review text reaches `plan-coverage` as an argument, which a long review can
/// overflow (32,767 characters for a whole command line on Windows). A failed check warns and leaves an incomplete
/// report naming the failure, never the empty file a reader takes for "nothing to report".
#[test]
fn a_failed_plan_coverage_check_is_loud() {
    let Some(sh) = shell() else { return };
    let p = taskboard();
    let dir = tempfile::tempdir().expect("a run dir");
    let review = dir.path().join("gsd-review-x.md");
    std::fs::write(&review, "## 01-01\nfine\n").expect("a review");
    let setup = format!(
        "RUN_DIR={}\nSLUG=x\nREVIEW_FILE={}\nPHASE_ID=v1.0/99",
        q(&dir.path().display().to_string()),
        q(&review.display().to_string())
    );
    let (_, err) = run(
        &sh,
        &p,
        "gsd-core/workflows/review.md",
        &setup,
        &["specforge gsd plan-coverage"],
        "",
    );
    let report =
        std::fs::read_to_string(dir.path().join(".plan-coverage-x.json")).expect("a report");
    let report: serde_json::Value =
        serde_json::from_str(&report).unwrap_or_else(|e| panic!("{e}: {report:?}\n{err}"));
    assert_eq!(report["complete"], false, "{report}");
    assert!(report["error"].is_string(), "{report}");
    assert!(err.contains("plan-coverage failed for x"), "{err}");
}
