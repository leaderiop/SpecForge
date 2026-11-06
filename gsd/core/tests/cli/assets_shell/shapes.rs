//! The lines that built an upstream-shaped object with jq, and the lines that read it, run as the
//! asset holds them under bash with no jq on the `PATH`. The taskboard fixture (`gsd/parity/fixtures/taskboard`):
//! phases 01 (2 of 2 plans summarised), 02 (0 of 1), 02.1 and 03 (no plan); the execute project, committed, for a
//! quick task's files. A plan's commits are read in `scripts.rs`.

use super::{Shell, asset, block, line};
use crate::harness::{Project, project, taskboard};

const T0: &str = "2026-10-29T09:00:00Z";
const T1: &str = "2026-10-29T09:30:00Z";

/// `script` in the project's root: `(stdout, stderr, exit code)`.
fn run(sh: &Shell, p: &Project, script: &str) -> (String, String, i32) {
    sh.run_in(p.root(), script)
}

/// `setup`, then the lines of `path` that start with each of `starts`, in order, then `printf '[%s]'` of each of
/// `vars`.
fn values(
    sh: &Shell,
    p: &Project,
    path: &str,
    setup: &str,
    starts: &[&str],
    vars: &[&str],
) -> String {
    let text = asset(path);
    let code: Vec<&str> = starts.iter().map(|s| line(&text, s)).collect();
    let show: String = vars
        .iter()
        .map(|v| format!("printf '[%s]' \"${v}\"\n"))
        .collect();
    let (out, err, _) = run(sh, p, &format!("{setup}\n{}\n{show}", code.join("\n")));
    assert!(!err.contains("jq"), "{path}: {err}");
    out
}

/// The execute project, committed once, then `commits` (subject, files) at T0, T1, …
fn committed(commits: &[(&str, &[(&str, &str)])]) -> Project {
    let p = project("execute");
    p.commit();
    for ((subject, files), at) in commits.iter().zip([T0, T1]) {
        p.git_commit(subject, files, at);
    }
    p
}

#[test]
fn a_count_of_plans_and_summaries_is_picked() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let starts = ["PLAN_COUNT=$(", "SUMMARY_COUNT=$("];
    let counts = |path, setup| {
        values(
            &sh,
            &p,
            path,
            setup,
            &starts,
            &["PLAN_COUNT", "SUMMARY_COUNT"],
        )
    };
    let plan = "gsd-core/workflows/execute-plan.md";
    assert_eq!(counts(plan, "PHASE_ID=v1.0/01"), "[2][2]");
    assert_eq!(counts(plan, "PHASE_ID=v1.0/02"), "[1][0]");
    let progress = "gsd-core/workflows/progress.md";
    let xy = values(
        &sh,
        &p,
        progress,
        "CURRENT_PHASE=v1.0/02",
        &["X=$(", "Y=$("],
        &["X", "Y"],
    );
    assert_eq!(xy, "[1][0]");
    // the resume scan reads a phase's counts from the progress report by its number, not by its index
    let scan = values(
        &sh,
        &p,
        progress,
        "PHASE_NUM=02",
        &[
            "PLAN_COUNT=$(specforge gsd progress-report",
            "SUMMARY_COUNT=$(specforge gsd progress-report",
        ],
        &["PLAN_COUNT", "SUMMARY_COUNT"],
    );
    assert_eq!(scan, "[1][0]");
}

#[test]
fn a_phase_is_read_field_by_field() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let mvp = "gsd-core/workflows/mvp-phase.md";
    let starts = ["PHASE_FOUND=", "PHASE_MODE=$(", "DISK_STATUS=$("];
    let vars = ["PHASE_FOUND", "PHASE_MODE", "DISK_STATUS"];
    assert_eq!(
        values(&sh, &p, mvp, "PHASE_ID=v1.0/02", &starts, &vars),
        "[true][][planned]"
    );
    assert_eq!(
        values(&sh, &p, mvp, "PHASE_ID=v1.0/nope", &starts, &vars),
        "[false][][]"
    );
    let name = values(
        &sh,
        &p,
        mvp,
        "PHASE_ID=v1.0/02",
        &["PHASE_NAME=$("],
        &["PHASE_NAME"],
    );
    assert!(name.len() > 2, "{name}");
    let execute = "gsd-core/workflows/execute-phase.md";
    let starts = ["PHASE_STATUS=$(", "case \"$PHASE_STATUS\""];
    assert_eq!(
        values(
            &sh,
            &p,
            execute,
            "PHASE_ID=v1.0/02",
            &starts,
            &["PHASE_MARKED"]
        ),
        "[false]"
    );
}

#[test]
fn a_milestone_is_verified_only_when_every_phase_is() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let path = "gsd-core/workflows/complete-milestone.md";
    let verified = values(
        &sh,
        &p,
        path,
        "MILESTONE=v1.0",
        &["ALL_PHASES_VERIFIED="],
        &["ALL_PHASES_VERIFIED"],
    );
    assert_eq!(verified, "[false]");
}

#[test]
fn the_open_windows_are_counted_and_an_unreadable_ledger_is_unknown() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let path = "gsd-core/workflows/ship.md";
    let starts = ["WINDOWS_OPEN_COUNT=$("];
    let count = |setup| values(&sh, &p, path, setup, &starts, &["WINDOWS_OPEN_COUNT"]);
    assert_eq!(count("MILESTONE=v1.0"), "[0]");
    assert_eq!(count("MILESTONE=nope"), "[?]");
    // a window with neither a waiver nor a fix is open
    let mut verify = p.read("spec/verify.spec");
    verify.push_str(OPEN_WINDOW);
    p.write("spec/verify.spec", &verify);
    assert_eq!(count("MILESTONE=v1.0"), "[1]");
}

/// A broken window of phase 01 that is open: no waiver, no fix.
pub(super) const OPEN_WINDOW: &str = "\nbroken_window BW1 \"A stub\" {\n  kind stub\n  phase v1.0/01\n  description \"a stub left in\"\n}\n";

#[test]
fn a_verification_status_routes_in_the_shell() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let goal = "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md";
    let starts = ["STATUS=$(", "case \"$STATUS\""];
    assert_eq!(
        values(
            &sh,
            &p,
            goal,
            "PHASE_ID=v1.0/02",
            &starts,
            &["STATUS", "ROUTE"]
        ),
        "[pending][verify]"
    );
    let work = "gsd-core/workflows/verify-work.md";
    let starts = [
        "VERIFICATION_STATUS_VALUE=$(",
        "case \"$VERIFICATION_STATUS_VALUE\"",
    ];
    let vars = ["VERIFICATION_STATUS_VALUE", "VERIFICATION_ROUTE"];
    assert_eq!(
        values(&sh, &p, work, "PHASE_ID=v1.0/02", &starts, &vars),
        "[pending][verify]"
    );
}

#[test]
fn a_quick_tasks_changed_files_are_its_commits_files() {
    // the collector's capability runs git: nothing else
    let Some(sh) = Shell::only(&["git"]) else {
        return;
    };
    let p = committed(&[
        (
            "feat(01-01): Store module",
            &[
                ("src/b.js", "x\n"),
                ("src/a.js", "y\n"),
                ("src/my file.js", "s\n"),
                ("src/two\nlines.js", "n\n"),
            ],
        ),
        ("fix(01-01): Store fix", &[("src/a.js", "z\n")]),
    ]);
    let text = asset("gsd-core/workflows/quick.md");
    let code = block(&text, "QUICK_SCOPE_RC=0", "CHANGED_FILES=$(");
    let (out, err, _) = run(
        &sh,
        &p,
        &format!("quick_id=v1.0/01/01-01\n{code}\nprintf '[%s]' \"$CHANGED_FILES\""),
    );
    // one path per line: a space stays in its name, and a newline in a name never reads as two paths
    assert_eq!(
        out, "[src/a.js\nsrc/b.js\nsrc/my file.js\nsrc/two\\nlines.js]",
        "{err}"
    );
    assert!(text.contains("Files (one path per line; a path may hold spaces): ${CHANGED_FILES}"));
    // an empty scope says why, in the words of what was read (there is no resolver's status)
    let why = block(
        &text,
        "QUICK_SCOPE_RC=0",
        "Quick-task review scope is empty",
    ) + "\nfi";
    let (out, err, _) = run(&sh, &p, &format!("quick_id=v1.0/01/01-02\n{why}"));
    assert_eq!(
        out, "Quick-task review scope is empty (no commit names this task).\n",
        "{err}"
    );
}

/// execute-phase's worktree handoff: the commits ahead of the base and the uncommitted paths, counted whole and
/// listed indented, at most 20 of them; a read that fails or finds nothing prints no line.
#[test]
fn the_worktree_handoff_counts_and_lists_what_is_left_indented() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = committed(&[
        ("feat: one", &[("a.txt", "1\n")]),
        ("feat: two", &[("b.txt", "2\n")]),
    ]);
    let base = p.git(&["rev-list", "--max-parents=0", "HEAD"], T1);
    let text = asset("gsd-core/workflows/execute-phase.md");
    let code = [
        "_WT_DIRTY_ALL=$(",
        "_WT_DIRTY_N=0",
        "[ -n \"$_WT_DIRTY_ALL\" ] && { _WT_DIRTY_N=",
        "_WT_LOG=$(specforge git log",
    ]
    .map(|start| line(&text, start))
    .join("\n");
    // nothing left and a base git does not know: no line, and the tolerated failure is not the script's
    let (out, err, code_of) = run(
        &sh,
        &p,
        &format!("_WT_BASE=no-such-ref\n{code}\nprintf '[%s]' \"$_WT_DIRTY_N\""),
    );
    assert_eq!((out.as_str(), err.as_str(), code_of), ("[0]", "", 0));
    for n in 1..=22 {
        p.write(&format!("u{n:02}.txt"), "left\n");
    }
    let shown = line(&text, "printf '    %s\\n' \"$_WT_DIRTY_ALL\"");
    let script = format!(
        "_WT_BASE={}\n{code}\n{shown}\nprintf '[%s]' \"$_WT_DIRTY_N\"",
        base.trim()
    );
    let (out, err, _) = run(&sh, &p, &script);
    let err_lines: Vec<&str> = err.lines().collect();
    assert_eq!(err_lines[..2], ["    feat: two", "    feat: one"], "{err}");
    assert_eq!(err_lines.len(), 22, "{err}");
    assert_eq!(err_lines[2], "    u01.txt", "{err}");
    assert!(
        err_lines[2..].iter().all(|l| l.starts_with("    u")),
        "{err}"
    );
    assert_eq!(out, "[22]");
}

/// onboard's entities are one `--fields kind,id` row per project, milestone, phase and requirement: a kind and an
/// id 0x1f-separated, from one export, where a loop over the kinds prefixed each id with `sed`.
#[test]
fn the_onboarding_entities_are_one_row_per_entity() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let read = line(&asset("gsd-core/workflows/onboard.md"), "ENTITIES=$(").to_string();
    let (out, err, _) = run(&sh, &p, &format!("{read}\nprintf '%s\\n' \"$ENTITIES\""));
    let rows: Vec<&str> = out.lines().collect();
    for want in [
        "project\u{1f}taskboard",
        "milestone\u{1f}v1.0",
        "phase\u{1f}v1.0/01",
        "phase\u{1f}v1.0/02.1",
    ] {
        assert!(rows.contains(&want), "{want:?} in {out}{err}");
    }
    assert!(
        rows.iter().any(|row| row.starts_with("requirement\u{1f}")),
        "{out}"
    );
    assert!(
        rows.iter().all(|row| row.split('\u{1f}').count() == 2),
        "{out}"
    );
}

/// `/gsd-quick status <slug>` reads the slug's plan: a quick plan's title is prose, and the slug is dashed.
#[test]
fn a_quick_plan_is_found_by_its_slug() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = project("execute");
    let made = p.run(&[
        "new",
        "plan",
        "251031-abc",
        "Sort rows",
        "--in",
        "spec/quick.spec",
        "--type",
        "execute",
        "--files_modified",
        "src/a.js",
        "--objective",
        "Rows are sorted",
        "--slug",
        "sort-rows",
    ]);
    assert_eq!(made.code, 0, "{}", made.all());
    let text = asset("commands/gsd/quick.md");
    let found = |slug: &str| {
        let code = line(&text, "dir=$(").replace("{SLUG}", slug);
        let (out, err, _) = run(&sh, &p, &format!("{code}\nprintf '[%s]' \"$dir\""));
        (out, err)
    };
    assert_eq!(
        found("sort-rows").0,
        "[251031-abc]",
        "{}",
        found("sort-rows").1
    );
    // The whole slug is matched, not a word of the title.
    assert_eq!(found("sort").0, "[]", "{}", found("sort").1);
    // The list reads the quick plans alone, one row each, whatever `spec_root` says.
    let (out, err, _) = run(&sh, &p, line(&text, "specforge export --kinds plan"));
    let rows: Vec<Vec<&str>> = out.lines().map(|l| l.split('\u{1f}').collect()).collect();
    assert_eq!(
        rows.iter().map(|r| r[0]).collect::<Vec<_>>(),
        ["251031-abc", "261028-a1b"],
        "{out}{err}"
    );
    assert_eq!(rows[0][1], "sort-rows");
    assert_eq!(rows[0][3], "Sort rows");

    // A slug run twice: ids hold no order of the runs, so no line picks one. Every run is listed, for the prompt to ask.
    for id in ["251031-zzz", "251031-aaa"] {
        let made = p.run(&[
            "new",
            "plan",
            id,
            "Trim",
            "--in",
            "spec/quick.spec",
            "--type",
            "execute",
            "--files_modified",
            "src/a.js",
            "--objective",
            "Trim",
            "--slug",
            "trim-it",
        ]);
        assert_eq!(made.code, 0, "{}", made.all());
    }
    let (out, err) = found("trim-it");
    assert_eq!(out, "[251031-aaa\n251031-zzz]", "{err}");
    for at in text.match_indices("dir=$(") {
        let rest = &text[at.0..];
        let after = &rest[rest.find("```").expect("the fence ends")..];
        let prose = after.lines().take(4).collect::<Vec<_>>().join("\n");
        assert!(
            prose.contains("more than one") && prose.contains("ask which"),
            "{prose}"
        );
    }
}

/// The inline plan's lines, as the asset holds them, under `specforge` alone: the summary is found from the plan,
/// stamped with the opening time and its `duration` read back.
#[test]
fn an_inline_plan_is_closed_on_its_summary() {
    use crate::cmd_execute::{PLAN, PLANTED, SUMMARY};
    use crate::commands_support::{plant, twin};
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = twin();
    let text = asset("gsd-core/workflows/execute-plan.md");
    let fence = |start: &str| {
        let (out, err, code) = run(&sh, &p, &format!("PLAN_ID={PLAN}\n{}", line(&text, start)));
        assert_eq!(code, 0, "{start}: {out}{err}");
    };
    // Each fence is its own shell here, and a re-run of the first never restarts the clock.
    fence("specforge gsd start-plan");
    let start = format!("{PLAN}/START");
    let first = p.fields_of(&start)["at"].clone();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    fence("specforge gsd start-plan");
    assert_eq!(p.fields_of(&start)["at"], first);
    // The steps in file order: the close comes after the step that writes the summary, and the old step has none of it.
    let at = |needle: &str| text.find(needle).unwrap_or_else(|| panic!("no {needle}"));
    let summary_step = at("<step name=\"create_summary\">");
    let summary_ends = summary_step + text[summary_step..].find("</step>").expect("its end");
    let close_at = at("SUMMARY_ID=$(");
    assert!(summary_step < close_at && close_at < summary_ends);
    let old = at("<step name=\"record_completion_time\">");
    let old_ends = old + text[old..].find("</step>").expect("its end");
    assert!(!text[old..old_ends].contains("complete-plan"));
    let close = block(&text, "SUMMARY_ID=$(", "specforge gsd complete-plan");
    let script = format!("PLAN_ID={PLAN}\n{close}\nprintf '[%s]' \"$SUMMARY_ID\"");
    // No summary yet: said so, and nothing reaches complete-plan.
    let (out, err, code) = run(&sh, &p, &script);
    assert_eq!((out.as_str(), code), ("", 1), "{err}");
    assert!(
        err.contains("exactly one summary") && !err.contains("E003"),
        "{err}"
    );
    plant(&p, "spec/plans.spec", PLANTED);
    let planted = p.read("spec/plans.spec");
    // Two summaries: both named, and no two-line id reaches complete-plan.
    let other = PLANTED.replace("02-01-SUMMARY", "02-01-AGAIN");
    plant(&p, "spec/plans.spec", &other);
    let two = p.read("spec/plans.spec");
    let (out, err, code) = run(&sh, &p, &script);
    assert_eq!((out.as_str(), code), ("", 1), "{err}");
    assert!(
        err.contains("exactly one summary")
            && err.contains("02-01-SUMMARY")
            && err.contains("02-01-AGAIN"),
        "{err}"
    );
    assert_eq!(p.read("spec/plans.spec"), two);
    p.write("spec/plans.spec", &planted);
    let (out, err, code) = run(&sh, &p, &script);
    assert_eq!(code, 0, "{err}");
    assert!(out.ends_with(&format!("[{SUMMARY}]")), "{out}");
    let fields = p.fields_of(SUMMARY);
    assert_eq!(fields["started"], first);
    assert!(fields["completed"].is_string() && fields["duration"].is_string());
}

/// Method 2's line, as the workflow gives it.
fn milestone_count_line() -> String {
    let text = asset("gsd-core/workflows/milestone-summary.md");
    let found = text
        .lines()
        .find(|l| l.contains("specforge query \"v${VERSION}\" --depth 0"))
        .expect("Method 2's line");
    format!("VERSION=1.0\n{found}")
}

/// `git commit` with no identity of the machine's, its message `subject`.
fn committing(subject: &str) -> String {
    format!(
        "git add -A && git -c user.name=gsd -c user.email=gsd@example.com -c commit.gpgsign=false \
         -c core.hooksPath=/dev/null commit -q --allow-empty -m '{subject}'"
    )
}

/// The count starts at the milestone's first recorded commit, and the line collects first, so commits made since the
/// last collect count. With no commit recorded, the line prints nothing.
#[test]
fn a_milestone_summary_counts_from_the_milestones_first_commit() {
    let Some(sh) = Shell::only(&["git"]) else {
        return;
    };
    let p = project("execute");
    p.init_repository();
    let (_, err, code) = run(&sh, &p, &committing("baseline"));
    assert_eq!(code, 0, "{err}");
    let code = milestone_count_line();
    let (out, err, _) = run(&sh, &p, &code);
    assert_eq!(out, "", "{err}");
    let first = p.git_commit("feat(01-01): Store module", &[("src/a.js", "y\n")], T0);
    p.git_commit("fix(01-01): Store fix", &[("src/a.js", "z\n")], T1);
    let (out, err, _) = run(&sh, &p, &code);
    assert_eq!(out.trim(), "2", "{err}");
    assert_eq!(p.fields_of("v1.0")["first_commit"], first[..12]);
}

/// Three milestone commits rebased over two others keep their author times and take new committer times: they count
/// three, however the two clocks read.
#[test]
fn a_rebased_milestone_counts_its_own_commits() {
    let Some(sh) = Shell::only(&["git"]) else {
        return;
    };
    let p = project("execute");
    p.init_repository();
    let script = format!(
        "{} && git branch -M main && git checkout -q -b work",
        committing("baseline")
    );
    let (_, err, code) = run(&sh, &p, &script);
    assert_eq!(code, 0, "{err}");
    p.git_commit("feat(01-01): one", &[("src/a.js", "1\n")], T0);
    p.git_commit("fix(01-01): two", &[("src/a.js", "2\n")], T1);
    p.git_commit("fix(01-01): three", &[("src/a.js", "3\n")], T1);
    let script = format!(
        "git checkout -q main && {} && {} && git -c user.name=gsd -c user.email=gsd@example.com \
         -c commit.gpgsign=false -c core.hooksPath=/dev/null rebase -q main work",
        committing("chore: main moved"),
        committing("chore: main moved again")
    );
    let (out, err, code) = run(&sh, &p, &script);
    assert_eq!(code, 0, "{out}{err}");
    let collected = p.run(&["collect", "--runner", "gsd-git", "--yes"]);
    assert_eq!(collected.code, 0, "{}", collected.all());
    let (out, err, _) = run(&sh, &p, &milestone_count_line());
    assert_eq!(out.trim(), "3", "{err}");
}

/// The handoff is written before the `pause` that holds the time, so it asks for no time of its own.
#[test]
fn a_handoff_asks_for_no_time_the_pause_has_not_stamped() {
    assert!(!asset("gsd-core/workflows/pause-work.md").contains("last_updated"));
}
