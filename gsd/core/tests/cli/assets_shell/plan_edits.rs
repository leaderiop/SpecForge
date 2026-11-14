//! The plan family's shell lines, run as the assets hold them on a taskboard copy under bash with only `specforge` on
//! the `PATH`.

use super::{Shell, asset, block, fence_after, span, typed};
use crate::harness::taskboard;

/// A review round of phase 02 with one high and one medium actionable concern, as the model fills it.
const ROUND: &str = r#"{"plans": ["v1.0/02/02-01"], "reviewers": [{"name": "codex", "model": "unknown", "source": "unknown"}, {"name": "claude", "model": "unknown", "source": "unknown"}], "lanes": [{"reviewer": "codex", "summary": "Sound, one gap", "strengths": ["small plan"], "suggestions": ["fsync the file"], "risk": "medium"}], "consensus": "The save is not crash-safe", "strengths": ["small plan"], "divergent": ["whether a lock is needed"], "concerns": [{"plan": "v1.0/02/02-01", "reviewers": ["codex", "claude"], "severity": "high", "text": "The rename is not atomic on every platform", "actionable": true}, {"plan": "v1.0/02/02-01", "reviewers": ["codex", "claude"], "severity": "medium", "text": "The file is never synced", "actionable": true}]}"#;

/// The planner's plan check reads the error codes `check` prints: on a copy whose plan lost its required
/// `files_modified`, the step prints `E006` beside what the clean copy prints.
#[test]
fn the_plan_check_step_reads_the_codes_check_prints() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let text = asset("agents/gsd-planner.md");
    let fence = fence_after(&text, "<step name=\"validate_plan\">");
    let codes = |p: &crate::harness::Project| {
        let (out, err, _) = sh.run_in(p.root(), &fence);
        let mut codes: Vec<String> = out.lines().map(str::to_string).collect();
        codes.sort();
        assert!(
            codes.iter().all(|c| !c.is_empty() || codes.len() == 1),
            "{out}{err}"
        );
        codes
    };
    let clean = taskboard();
    let broken = taskboard();
    broken.edit(
        "spec/plans.spec",
        "  files_modified   [\"src/store.js\", \"src/cli.js\", \"test/store.test.js\"]\n",
        "",
    );
    let mut new = codes(&broken);
    for code in codes(&clean) {
        if let Some(at) = new.iter().position(|c| *c == code) {
            new.remove(at);
        }
    }
    assert_eq!(new, ["E006"]);
    for stale in ["valid=false", "task_count", "invalidValue"] {
        assert!(!text.contains(stale), "the step still reads {stale}");
    }
}

/// The planner's typed edits, filled as a model fills them, write a plan in its own file with a task, a truth, an
/// artifact, a key link and a dependency, then a checkpoint, a prohibition and a user setup step, and the check reports
/// nothing new but the judged rules' "no verdict yet".
#[test]
fn the_planners_typed_edits_write_a_checked_plan() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let text = asset("agents/gsd-planner.md");
    let plan = typed::filled(
        &typed::typed_fence(&text, "## The Plan's Typed Edits"),
        &[
            ("PHASE_ID", "v1.0/02"),
            ("PLAN", "02-02"),
            ("TITLE", "Read the board back"),
            ("TYPE", "execute"),
            ("DEPENDS_ON", "02-01"),
            ("FILE_1", "src/index.js"),
            ("FILE_2", "src/cli.js"),
            ("REQUIREMENTS", "STORE-02"),
            ("DECISIONS", "D-03"),
            ("OBJECTIVE", "Load the board on start"),
            ("PURPOSE", "a restarted board shows its tasks"),
            ("OUTPUT", "`src/index.js` with `loadBoard`"),
            ("CONTEXT_REF", "spec/project.spec"),
            ("VERIFICATION", "`npm test` passes"),
            ("SUCCESS_CRITERIA", "STORE-02 holds"),
            ("ESTIMATE", r#"{"tokens": 60000}"#),
            ("TASK_NAME", "Task 1: Load the board"),
            ("TASK_FILE", "src/index.js"),
            ("TASK_ACTION", "Read `tasks.json` into a board"),
            ("TASK_VERIFY", "npm test"),
            ("TASK_DONE", "the board loads"),
            ("TRUTH", "A restarted board shows the tasks it had"),
            ("TRUTH_REQUIREMENT", "STORE-02"),
            ("ARTIFACT_NAME", "The board loader"),
            ("ARTIFACT_PATH", "src/index.js"),
            ("ARTIFACT_PROVIDES", "loadBoard"),
            ("LINK_NAME", "The command line loads the board"),
            ("LINK_FROM", "src/cli.js"),
            ("LINK_TO", "src/index.js"),
            ("LINK_VIA", "import { loadBoard }"),
        ],
    );
    let more = typed::filled(
        &typed::typed_fence(&text, "A plan's other entities"),
        &[
            ("CHECKPOINT_TITLE", "The reload is checked by hand"),
            ("CHECKPOINT_KIND", "human_verify"),
            ("CHECKPOINT_AFTER", "T1"),
            ("WHAT_BUILT", "the board loader"),
            ("HOW_TO_VERIFY", "add a task, restart, list"),
            ("RESUME_SIGNAL", "approved"),
            ("STATEMENT", "A reload never drops a task"),
            ("VERIFICATION_KIND", "judgment"),
            ("SERVICE", "none"),
            ("WHY", "the board needs no service"),
            ("NEEDED_PLAN", "v1.0/02/02-01"),
            ("PLAN_ID", "v1.0/02/02-02"),
            ("PLAN_FILE", "spec/plans/02-02.spec"),
        ],
    );
    let first = typed::apply(&sh, &p, &plan);
    assert_eq!(first.code, 0, "{}{}", first.out, first.err);
    let second = typed::apply(&sh, &p, &more);
    assert_eq!(second.code, 0, "{}{}", second.out, second.err);
    assert!(
        first.added.is_empty() && second.added.is_empty(),
        "{:?} {:?}",
        first.added,
        second.added
    );
    assert!(p.exists("spec/plans/02-02.spec"));
    for id in ["", "/T1", "/TR1", "/AR1", "/KL1", "/CP1", "/PR1", "/US1"] {
        let id = format!("v1.0/02/02-02{id}");
        assert!(p.fields_of(&id).is_object(), "{id} is missing");
    }
    assert_eq!(p.fields_of("v1.0/02/02-02/AR1")["path"], "src/index.js");
    assert_eq!(
        p.fields_of("v1.0/02/02-02")["depends_on"],
        serde_json::json!(["v1.0/02/02-01"])
    );
}

/// Convergence reads the latest round's open counts from the graph: one high and one medium actionable concern give
/// 1 and 1 with no open conflict; the round takes a drift finding that counts in neither, and a disposition that
/// addresses the high concern closes it.
#[test]
fn the_convergence_step_counts_the_open_concerns() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    // a single-reviewer, unconfirmed HIGH: recorded high with `actionable` false, so HIGH_COUNT stays 1
    let round = ROUND.replace(
        "]}",
        r#", {"plan": "v1.0/02/02-01", "reviewers": ["codex"], "severity": "high", "text": "A lock is missing (single-reviewer, unconfirmed)", "actionable": false}]}"#,
    );
    let record = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/workflows/review.md"),
            "Only reached when `TOTAL_LANE_FAILURE`",
        ),
        &[("PHASE_ID", "v1.0/02"), ("REVIEW_JSON", &round)],
    );
    let run = typed::apply(&sh, &p, &record);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    let text = asset("gsd-core/workflows/plan-review-convergence.md");
    let counts = typed::filled(
        &typed::typed_fence(&text, "### 5b. Read the Open Concerns from the Graph"),
        &[("PHASE_ID", "v1.0/02")],
    );
    // the block's open-concern lines, read through `--fields` with the unit separator
    let run = typed::apply(&sh, &p, &format!("{counts}\nprintf '%s' \"$OPEN_LINES\""));
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(
        run.out
            .contains("REVIEW_ID=v1.0/02/RV1 HIGH_COUNT=1 ACTIONABLE_COUNT=1 OPEN_CONFLICTS=0"),
        "{}",
        run.out
    );
    for line in [
        "- high: The rename is not atomic on every platform",
        "- medium: The file is never synced",
        "- high: A lock is missing (single-reviewer, unconfirmed)",
    ] {
        assert!(run.out.contains(line), "{line}:\n{}", run.out);
    }
    let drift = typed::filled(
        &typed::typed_fence(
            &text,
            "**Record the coverage block and the drift findings**",
        ),
        &[
            ("PHASE_ID", "v1.0/02"),
            (
                "ROUND_JSON",
                r#"{"unchecked": [{"symbol": "--watch", "why": "grep cannot check a signature"}], "concerns": [{"plan": "v1.0/02/02-01", "severity": "low", "text": "The roadmap and the plan disagree on the file name", "drift": true, "evidence": ["spec/v1.0.spec:30", "spec/plans.spec:240"]}]}"#,
            ),
        ],
    );
    let run = typed::apply(&sh, &p, &drift);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    let disposition = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/references/planner-reviews.md"),
            "The ledger is one `disposition` per concern",
        ),
        &[
            ("PLAN_ID", "v1.0/02/02-01"),
            ("DISPOSITION_ID", "DP1"),
            ("CONCERN_ID", "v1.0/02/RV1/RC1"),
            ("ROUND", "1"),
            ("REVIEWER", "codex"),
            ("FEEDBACK", "Rename not atomic"),
            ("OUTCOME", "addressed"),
            ("REASON", "Task 1 writes then renames on the same volume"),
        ],
    );
    let run = typed::apply(&sh, &p, &disposition);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    let run = typed::apply(&sh, &p, &counts);
    assert!(
        run.out
            .contains("HIGH_COUNT=0 ACTIONABLE_COUNT=1 OPEN_CONFLICTS=0"),
        "{}{}",
        run.out,
        run.err
    );
}

/// A round with one reviewer (the `--codex` default) records the concerns its lane raised, so its HIGH concern is open
/// and convergence does not pass on the first cycle.
#[test]
fn a_one_reviewer_round_keeps_its_high_concern() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let round = r#"{"plans": ["v1.0/02/02-01"], "reviewers": [{"name": "codex", "model": "unknown", "source": "unknown"}], "lanes": [{"reviewer": "codex", "summary": "One gap", "strengths": ["small plan"], "suggestions": ["fsync the file"], "risk": "high"}], "consensus": "The save is not crash-safe", "strengths": [], "divergent": [], "concerns": [{"plan": "v1.0/02/02-01", "reviewers": ["codex"], "severity": "high", "text": "The rename is not atomic on every platform", "actionable": true}]}"#;
    let record = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/workflows/review.md"),
            "Only reached when `TOTAL_LANE_FAILURE`",
        ),
        &[("PHASE_ID", "v1.0/02"), ("REVIEW_JSON", round)],
    );
    let run = typed::apply(&sh, &p, &record);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    let counts = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/workflows/plan-review-convergence.md"),
            "### 5b. Read the Open Concerns from the Graph",
        ),
        &[("PHASE_ID", "v1.0/02")],
    );
    let run = typed::apply(&sh, &p, &counts);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.out.contains("HIGH_COUNT=1 "), "{}", run.out);
}

/// The convergence counts block stops when the phase has no recorded round.
#[test]
fn the_convergence_step_stops_without_a_review() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let counts = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/workflows/plan-review-convergence.md"),
            "### 5b. Read the Open Concerns from the Graph",
        ),
        &[("PHASE_ID", "v1.0/02")],
    );
    let (out, err, code) = sh.run_in(p.root(), &counts);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("no review recorded for this cycle"), "{err}");
}

/// `/gsd-review` records its round with the concerns the lanes agreed on, then commits it.
#[test]
fn a_review_is_recorded_with_its_concerns() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    p.commit();
    // The commit fence commits through specforge, which reads the repository's identity, not the harness's flags.
    p.git(&["config", "user.name", "gsd"], "2026-01-01T00:00:00Z");
    p.git(
        &["config", "user.email", "gsd@example.com"],
        "2026-01-01T00:00:00Z",
    );
    let text = asset("gsd-core/workflows/review.md");
    let record = typed::filled(
        &typed::typed_fence(&text, "Only reached when `TOTAL_LANE_FAILURE`"),
        &[("PHASE_ID", "v1.0/02"), ("REVIEW_JSON", ROUND)],
    );
    let run = typed::apply(&sh, &p, &record);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    assert!(run.out.contains("v1.0/02/RV1"), "{}", run.out);
    let commit = typed::filled(
        &typed::typed_fence(&text, "Commit the recorded round"),
        &[("PHASE_ID", "v1.0/02")],
    );
    let run = typed::apply(&sh, &p, &commit);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    let (reviews, err, _) = sh.run_in(
        p.root(),
        "specforge query v1.0/02 --depth 1 --kinds review --pick '.nodes[kind=review].id' --count",
    );
    assert_eq!(reviews.trim(), "1", "{err}");
    let (statuses, err, _) = sh.run_in(
        p.root(),
        "specforge query v1.0/02/RV1 --depth 1 --kinds review_concern --pick '.nodes[kind=review_concern].fields.status'",
    );
    assert_eq!(
        statuses.lines().collect::<Vec<_>>(),
        ["open", "open"],
        "{err}"
    );
    let subject = p.git(&["log", "-1", "--format=%s"], "2026-01-01T00:00:00Z");
    assert_eq!(subject.trim(), "docs(review): record the review of v1.0/02");
}

/// The PRD express path records the phase's context as typed edits: the context, a decision per requirement (the
/// line repeated with the next id, as a model repeats it) and a deferred idea, on a phase with no context.
#[test]
fn the_prd_express_path_writes_the_phase_context() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let text = asset("gsd-core/workflows/plan-phase/steps/prd-express-path.md");
    let fence = typed::filled(
        &typed::typed_fence(&text, "4. Record the context as typed edits"),
        &[
            ("PHASE_ID", "v1.0/03"),
            ("CONTEXT", "03-CONTEXT"),
            ("TITLE", "Listing context"),
            ("BOUNDARY", "Open tasks are listed and filtered by a word"),
            ("DISCRETION", "The column widths"),
            ("SPECIFICS", "taskboard list prints one task per line"),
            (
                "CANONICAL_REFS",
                r#"[{"topic": "No external specs — requirements fully captured in decisions above", "paths": []}]"#,
            ),
            ("AREA", "Listing"),
            ("DECISION", "The list shows open tasks, oldest first"),
            ("DEFERRED", "Sorting by due date"),
        ],
    );
    let second = super::line(&fence, "specforge new decision D-01")
        .replace("D-01 \"$DECISION\"", "D-02 \"$DECISION\"");
    let script = format!("{fence}\nDECISION=\"A filter matches a word of the title\"\n{second}");
    let run = typed::apply(&sh, &p, &script);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    for id in ["03-CONTEXT", "D-01", "D-02", "DI1"] {
        let id = format!("v1.0/03/{id}");
        assert!(p.fields_of(&id).is_object(), "{id} is missing");
    }
    assert_eq!(p.fields_of("v1.0/03/D-02")["origin"], "context");
}

/// The researcher records the phase's research and its items as typed edits, and plan-phase records the pack the
/// research read when the researcher returns.
#[test]
fn the_research_fence_records_the_phase_research() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let text = asset("agents/gsd-phase-researcher.md");
    let fence = typed::filled(
        &typed::typed_fence(&text, "1. **Create the entity first**"),
        &[
            ("PHASE_ID", "v1.0/03"),
            ("RESEARCH", "03-RESEARCH"),
            ("TITLE", "Listing research"),
            ("CONFIDENCE", "high"),
            ("SUMMARY", "Filter in memory; the board is small"),
            ("FINDING", "The board fits in memory"),
            ("PITFALL", "Case-sensitive filters"),
            ("GOES_WRONG", "A filter misses a title in another case"),
            ("WHY", "String includes is case-sensitive"),
            ("AVOID", "Lower-case both sides"),
            ("PATTERN", "Filter then sort"),
            ("WHAT", "Filter the open tasks, then sort them by id"),
        ],
    );
    let run = typed::apply(&sh, &p, &fence);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    for id in ["", "/F1", "/PF1", "/P1"] {
        let id = format!("v1.0/03/03-RESEARCH{id}");
        assert!(p.fields_of(&id).is_object(), "{id} is missing");
    }
    let record = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/workflows/plan-phase.md"),
            "- **`## RESEARCH COMPLETE`:**",
        ),
        &[("RESEARCH_ID", "v1.0/03/03-RESEARCH")],
    );
    let run = typed::apply(&sh, &p, &record);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
}

/// The spec-less lift writes both resolved edges as truths with no tier (a person's field, so the backstop one is never
/// contradicted by a test), a kept prohibition with no test, and an unresolved row as an assumption in the purpose.
#[test]
fn the_spec_less_lift_writes_truths_and_a_descriptor_less_prohibition() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let text = asset("gsd-core/references/specless-probe-fallback.md");
    let fence = typed::filled(
        &typed::typed_fence(&text, "## C. Authoring"),
        &[
            ("PLAN_ID", "v1.0/02/02-01"),
            ("REQUIREMENT", "STORE-01"),
            ("EXPLICIT_ID", "TR4"),
            (
                "CRITERION",
                "A save interrupted midway leaves the previous tasks.json whole",
            ),
            ("BACKSTOP_ID", "TR5"),
            ("CHECK", "A full disk is reported, not ignored"),
            ("PROHIBITION_ID", "PR1"),
            ("STATEMENT", "The board never sends a task over the network"),
            (
                "PURPOSE",
                "Keep the board across restarts. Assumption: a file locked by another process is not handled",
            ),
        ],
    );
    let run = typed::apply(&sh, &p, &fence);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    for id in ["TR4", "TR5"] {
        let truth = p.fields_of(&format!("v1.0/02/02-01/{id}"));
        assert!(truth.is_object(), "{id} is missing");
        assert!(truth.get("verification").is_none(), "{truth}");
    }
    // the backstop edge is proposed; a person confirms it by setting `verification`
    let backstop = p.fields_of("v1.0/02/02-01/TR5");
    assert_eq!(backstop["proposed_verification"], "backstop", "{backstop}");
    assert!(
        p.fields_of("v1.0/02/02-01/TR4")
            .get("proposed_verification")
            .is_none()
    );
    let prohibition = p.fields_of("v1.0/02/02-01/PR1");
    assert_eq!(prohibition["verification"], "judgment", "{prohibition}");
    assert!(prohibition.get("test").is_none(), "{prohibition}");
    let purpose = p.fields_of("v1.0/02/02-01")["purpose"].to_string();
    assert!(purpose.contains("Assumption:"), "{purpose}");
}

/// The MVP fence sets the phase's mode and goal, the dependency fence its `depends_on`, and the milestone review a
/// project field: each a typed edit, nothing written to a roadmap or project file.
#[test]
fn the_mvp_and_dependency_edits_change_the_phase() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let story = "As a user, I want to filter my tasks, so that I see only what matters now.";
    let mvp = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/workflows/mvp-phase.md"),
            "## 5. Record the MVP Mode and Goal",
        ),
        &[
            ("PHASE_ID", "v1.0/03"),
            ("PHASE", "03"),
            ("USER_STORY", story),
        ],
    );
    let deps = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/workflows/analyze-dependencies.md"),
            "Each accepted phase's dependencies",
        ),
        &[("PHASE", "03"), ("DEPENDS_ON", r#"["01", "02"]"#)],
    );
    let project = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/workflows/complete-milestone/steps/project-md-evolution-review.md"),
            "Record each change on the project entity",
        ),
        &[
            ("PROJECT", "taskboard"),
            ("FIELD", "core_value"),
            ("VALUE", "A board that never loses a task"),
        ],
    );
    for fence in [mvp, deps, project] {
        let run = typed::apply(&sh, &p, &fence);
        assert_eq!(run.code, 0, "{fence}\n{}{}", run.out, run.err);
        assert!(run.added.is_empty(), "{:?}", run.added);
    }
    let phase = p.fields_of("v1.0/03");
    assert_eq!(phase["mode"], "mvp", "{phase}");
    assert_eq!(phase["goal"], story, "{phase}");
    let depends = phase["depends_on"].to_string();
    assert!(depends.contains("01") && depends.contains("02"), "{phase}");
    assert_eq!(
        p.fields_of("taskboard")["core_value"],
        "A board that never loses a task"
    );
    // the planner's roadmap step fills a phase's goal
    let goal = typed::filled(
        &typed::typed_fence(
            &asset("agents/gsd-planner.md"),
            "<step name=\"update_roadmap\">",
        ),
        &[
            ("PHASE_NUMBER", "03"),
            ("GOAL", "A user sees only the tasks a filter keeps"),
        ],
    );
    let run = typed::apply(&sh, &p, &goal);
    assert_eq!(run.code, 0, "{goal}\n{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    assert_eq!(
        p.fields_of("v1.0/03")["goal"],
        "A user sees only the tasks a filter keeps"
    );
}

/// The checker's coverage trace reaches the phase's requirements: both of phase 02's reach a plan, and a requirement
/// the phase lists that no plan claims says `no plan`.
#[test]
fn the_coverage_trace_names_an_uncovered_requirement() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let text = asset("agents/gsd-plan-checker.md");
    let trace = typed::filled(
        &typed::typed_fence(&text, "2. Trace each of the phase's requirements"),
        &[("PHASE_ID", "v1.0/02")],
    );
    let thread = |p: &crate::harness::Project| {
        let (out, err, code) = sh.run_in(p.root(), &trace);
        assert_eq!(code, 0, "{out}{err}");
        out
    };
    let covered = thread(&p);
    assert!(
        covered.contains("STORE-01") && covered.contains("STORE-02"),
        "{covered}"
    );
    assert!(!covered.contains("no plan"), "{covered}");
    // A requirement the phase lists and no plan claims is written by hand: a typed edit refuses the error it makes.
    p.edit(
        "spec/v1.0.spec",
        "requirements     [STORE-01, STORE-02]",
        "requirements     [STORE-01, STORE-02, STORE-03]",
    );
    let requirements = p.read("spec/requirements.spec");
    p.write(
        "spec/requirements.spec",
        &format!(
            "{requirements}\nrequirement STORE-03 \"A save survives a crash\" {{\n  text     \"A crash mid-save leaves the board whole\"\n  category \"Storage\"\n  tier     v1\n  defined  2026-03-02\n}}\n"
        ),
    );
    let uncovered = thread(&p);
    assert!(uncovered.contains("STORE-03 → no plan"), "{uncovered}");
}

/// The pattern mapper finds an analog from the graph: the store task's search, then the source its facts are anchored
/// to.
#[test]
fn the_analog_lookup_names_the_anchored_task() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let run = p.collect(&[]);
    assert_eq!(run.code, 0, "{}", run.all());
    for body in [
        "agents/gsd-pattern-mapper.md",
        "agents/gsd-pattern-mapper.compact.md",
    ] {
        let text = asset(body);
        let heading = if body.ends_with("compact.md") {
            "Find the closest existing file"
        } else {
            "For each classified file, find the closest"
        };
        let lookup = typed::filled(
            &typed::typed_fence(&text, heading),
            &[("ROLE_WORDS", "board"), ("TASK_ID", "v1.0/01/01-01/T1")],
        );
        let (out, err, code) = sh.run_in(p.root(), &lookup);
        assert_eq!(code, 0, "{body}: {out}{err}");
        assert!(out.contains("v1.0/01/01-01/T1"), "{body}: {out}");
        assert!(out.contains("src/tasks.js"), "{body}: {out}");
        let (owner, err, code) = sh.run_in(p.root(), "specforge anchors --file src/tasks.js");
        assert_eq!(code, 0, "{err}");
        assert!(owner.contains("[task v1.0/01/01-01/T1]"), "{owner}");
    }
}

/// The code spans of the line of `text` that starts with `start`, in order.
fn spans(text: &str, start: &str) -> Vec<String> {
    gsd_kit::markdown::inline_code(super::line(text, start))
        .into_iter()
        .map(|(_, s)| s.to_string())
        .collect()
}

/// The planner spawn's first required reading is the planner route's pack of the phase: it answers the route's core
/// sections, and the block reads no entity path the pack serves and no context-window branch.
#[test]
fn the_planner_spawn_reads_the_planner_route() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let text = asset("gsd-core/workflows/plan-phase.md");
    let reading = block(
        &text,
        "- `specforge context gsd/planner {PHASE_ID}` (the planner pack",
        "</required_reading>",
    );
    let before = text
        .lines()
        .take_while(|l| !l.contains("(the planner pack:"))
        .last();
    assert_eq!(
        before,
        Some("<required_reading>"),
        "the pack is the first reading"
    );
    let first = spans(&text, "- `specforge context gsd/planner {PHASE_ID}`")[0]
        .replace("{PHASE_ID}", "v1.0/02");
    let (out, err, code) = sh.run_in(p.root(), &format!("{first} --pick '.sections[].name'"));
    assert_eq!(code, 0, "{out}{err}");
    for section in ["phase", "requirements", "context", "decisions", "settings"] {
        assert!(out.lines().any(|l| l == section), "no {section}: {out}");
    }
    for gone in ["query {context_path}", "{research_path}", "CONTEXT_WINDOW"] {
        assert!(!reading.contains(gone), "{gone} in:\n{reading}");
    }
}

/// The plan checker spawn reads one check pack per plan of the phase.
#[test]
fn the_plan_checker_spawn_reads_one_pack_per_plan() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let text = asset("gsd-core/workflows/plan-phase.md");
    let [pack, plans] = &spans(&text, "- `specforge context gsd/plan-checker <id>`")[..2] else {
        panic!("the checker line has its two commands");
    };
    let plans = plans.replace("{PHASE_ID}", "v1.0/01");
    let pack = pack.replace("<id>", "\"$id\"");
    let script = format!("for id in $({plans}); do {pack} --pick '.subject'; done");
    let (out, err, code) = sh.run_in(p.root(), &script);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        ["v1.0/01/01-01", "v1.0/01/01-02"],
        "{err}"
    );
    let body = asset("agents/gsd-plan-checker.md");
    let lp = fence_after(&body, "Orchestrator provides CONTEXT.md content");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{lp}"));
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("v1.0/01/01-01") && out.contains("v1.0/01/01-02"),
        "{out}"
    );
}

/// A source-audit gap the developer defers is recorded as a deferred idea of the phase, as the model fills the fence.
#[test]
fn a_deferred_audit_gap_is_a_deferred_idea() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let defer = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/workflows/plan-phase/detail/elaboration.md"),
            "**If \"Defer\":**",
        ),
        &[
            ("PHASE_ID", "v1.0/02"),
            ("IDEA_ID", "DI2"),
            ("DEFERRED", "Export the board as CSV"),
            ("WHY", "the developer deferred it at the source audit"),
        ],
    );
    let run = typed::apply(&sh, &p, &defer);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    // the idea joins the phase context the twin's research was written against, so the research reads as stale
    assert_eq!(run.added, ["gsd/W406"], "{:?}", run.added);
    let idea = p.fields_of("v1.0/02/DI2");
    assert_eq!(idea["phase"], "v1.0/02", "{idea}");
    assert_eq!(idea["text"], "Export the board as CSV", "{idea}");
}

/// Plan-phase's decision coverage reads gsd/E310 the same way in its step and its gate: on a copy whose plan dropped
/// a decision, both name it, and moving the decision into the context's discretion, as the gate's second option says,
/// clears it.
#[test]
fn the_decision_coverage_reads_e310_and_discretion_clears_it() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let plans = p.read("spec/plans.spec");
    let dropped = plans.replace(
        "decisions        [D-01, D-02, D-03]",
        "decisions        [D-01, D-02]",
    );
    assert_ne!(dropped, plans, "the twin's plan lists D-03");
    p.write("spec/plans.spec", &dropped);
    let text = asset("gsd-core/workflows/plan-phase.md");
    let step = [span(
        &text,
        "specforge check --format json --pick '[code=gsd/E310].message'",
    )];
    let gate = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("GATE_RESULT=$(specforge check"))
        .expect("the gate's read");
    let gate = gate.split("   #").next().expect("the line").to_string();
    let (out, err, code) = sh.run_in(p.root(), &step[0]);
    assert!(code <= 1, "{out}{err}");
    assert!(out.contains("D-03"), "{out}{err}");
    let (out, err, code) = sh.run_in(
        p.root(),
        &format!("{gate}\nprintf '%s|%s' \"$GATE_EXIT\" \"$GATE_RESULT\""),
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.starts_with("1|") && out.contains("D-03"), "{out}{err}");
    let discretion = span(&text, "specforge set <context> discretion")
        .replace("<context>", "v1.0/02/02-CONTEXT")
        .replace(
            "<each item>…",
            "\"The indent of the JSON file\" \"A title may be trimmed\"",
        );
    let delete = span(&text, "specforge delete <decision>").replace("<decision>", "v1.0/02/D-03");
    // the twin's research cites D-03 in `user_constraints`: the delete's refusal names it, so it goes first
    let uncite = "specforge set v1.0/02/02-RESEARCH user_constraints D-01 D-02".to_string();
    for line in [discretion, uncite, delete] {
        let (out, err, code) = sh.run_in(p.root(), &line);
        assert_eq!(code, 0, "{line}: {out}{err}");
    }
    assert_eq!(
        p.fields_of("v1.0/02/02-CONTEXT")["discretion"][1],
        "A title may be trimmed"
    );
    let (out, _, _) = sh.run_in(p.root(), &step[0]);
    assert!(!out.contains("D-03"), "{out}");
}

/// revision-loop's record fence, filled for a plan of the taskboard with a property holding an apostrophe, a
/// backtick and a `$` and no alternatives: it records one conflict that keeps the text as written and has no
/// alternatives.
#[test]
fn the_conflict_fence_records_its_values_as_written() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let property = "the user's `save` keeps $HOME";
    let fence = typed::filled(
        &super::verify_edits::fence_holding(
            &asset("gsd-core/references/revision-loop.md"),
            "record-conflict",
        ),
        &[
            ("PLAN_ID", "v1.0/01/01-02"),
            ("DIMENSION", "task_completeness"),
            ("PROPERTY", property),
            ("CONFLICTS_WITH", "D-02"),
            ("ALTERNATIVES", ""),
        ],
    );
    let done = typed::apply(&sh, &p, &fence);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert!(done.added.is_empty(), "{:?}", done.added);
    let conflicts = p.nodes_of("revision_conflict");
    assert_eq!(conflicts.len(), 1, "{conflicts:?}");
    let fields = p.fields_of(conflicts[0]["id"].as_str().expect("an id"));
    assert_eq!(fields["property"], property);
    assert!(
        fields.get("alternatives").is_none_or(|a| a.is_null()),
        "{fields}"
    );
}
