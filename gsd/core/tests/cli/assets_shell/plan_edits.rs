//! The plan family's shell lines, run as the assets hold them on a taskboard copy under bash with only `specforge` on
//! the `PATH`.

use super::{Shell, asset, fence_after, typed};
use crate::harness::taskboard;

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

/// Convergence checks the review agent's REVIEWS.md under `phase_dir`, which no line of the workflow assigns: the
/// guard stops every cycle after the first review returns.
#[test]
fn the_convergence_step_stops_on_an_unset_phase_dir() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/plan-review-convergence.md");
    let fence = fence_after(&text, "After agent returns, verify REVIEWS.md exists");
    let dir = tempfile::tempdir().expect("an empty dir");
    let (out, err, code) = sh.run_in(dir.path(), &fence);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("phase_dir is empty"), "{err}");
}

/// The review's commit step, reached only when a lane produced a review, runs `:` and nothing else: the combined
/// review is recorded nowhere, though the `review` kind exists.
#[test]
fn the_review_commit_records_nothing() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let text = asset("gsd-core/workflows/review.md");
    let fence = fence_after(&text, "Commit (only reached when");
    assert!(!fence.contains("specforge"), "{fence}");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/02\n{fence}"));
    assert_eq!(code, 0, "{out}{err}");
    let (count, err, _) = sh.run_in(
        p.root(),
        "specforge query v1.0/02 --depth 1 --kinds review --pick '.nodes[kind=review].id' --count",
    );
    assert_eq!(count.trim(), "0", "{err}");
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
    let prohibition = p.fields_of("v1.0/02/02-01/PR1");
    assert_eq!(prohibition["verification"], "judgment", "{prohibition}");
    assert!(prohibition.get("test").is_none(), "{prohibition}");
    let purpose = p.fields_of("v1.0/02/02-01")["purpose"].to_string();
    assert!(purpose.contains("Assumption:"), "{purpose}");
}
