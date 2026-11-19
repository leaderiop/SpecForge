//! A plan's estimate error, end to end on the taskboard twin (ADR 0378 D5): a completed plan's estimate against what
//! it took, as the host keeps the computed number, and absent, never zero, where nothing was measured.

use serde_json::{Value, json};

use crate::harness::{Project, taskboard};

/// 01-01's `files_modified` line: the anchor its estimate is written after.
const PLAN_01_01: &str =
    "  files_modified   [\"package.json\", \"src/tasks.js\", \"test/tasks.test.js\"]\n";

/// 02-01's `files_modified` line: a plan with no summary.
const PLAN_02_01: &str =
    "  files_modified   [\"src/store.js\", \"src/cli.js\", \"test/store.test.js\"]\n";

/// The taskboard twin with `estimate { <estimate> }` written after `anchor`.
fn estimating(anchor: &str, estimate: &str) -> Project {
    let p = taskboard();
    p.edit(
        "spec/plans.spec",
        anchor,
        &format!("{anchor}  estimate         {{ {estimate} }}\n"),
    );
    p
}

/// The `estimate_error` of plan `id`, `Null` when the plan has none.
pub(super) fn error_of(p: &Project, id: &str) -> Value {
    p.fields_of(id)["estimate_error"].clone()
}

/// No computed value was refused by its field's form.
fn admitted(p: &Project) {
    let codes = p.codes();
    assert!(!codes.contains(&"W165".to_string()), "{codes:?}");
}

#[test]
fn a_completed_plan_measures_its_task_estimate() {
    // 01-01 has two tasks and a complete summary: three estimated is a third too many.
    let p = estimating(PLAN_01_01, "tokens 60000 tasks 3");
    assert_eq!(error_of(&p, "v1.0/01/01-01"), json!({ "tasks": -0.3333 }));
    admitted(&p);
    assert_eq!(error_of(&p, "v1.0/01/01-02"), Value::Null, "no estimate");

    // An estimate that held is a measured zero, kept as one.
    let p = estimating(PLAN_01_01, "tasks 2");
    assert_eq!(error_of(&p, "v1.0/01/01-01"), json!({ "tasks": 0 }));
    admitted(&p);
}

#[test]
fn without_an_actual_the_member_is_absent_not_zero() {
    // No harness records the tokens a plan took: the tokens member is absent, not 0.
    let p = estimating(PLAN_01_01, "tokens 60000");
    assert_eq!(error_of(&p, "v1.0/01/01-01"), Value::Null);

    // 02-01 has no summary: nothing was measured.
    let p = estimating(PLAN_02_01, "tokens 60000 tasks 2");
    assert_eq!(error_of(&p, "v1.0/02/02-01"), Value::Null);

    let p = taskboard();
    for plan in ["v1.0/01/01-01", "v1.0/01/01-02", "v1.0/02/02-01"] {
        assert_eq!(error_of(&p, plan), Value::Null, "{plan}");
    }
}

/// The text `specforge gsd progress-report` prints.
pub(super) fn report_text(p: &Project) -> String {
    let run = p.run(&["gsd", "progress-report"]);
    assert_eq!(run.code, 0, "{}", run.all());
    run.stdout
}

/// The JSON `specforge gsd progress-report --format json` prints.
fn report_json(p: &Project) -> Value {
    let run = p.run(&["gsd", "progress-report", "--format", "json"]);
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("JSON ({e}): {}", run.all()))
}

/// The progress report of the unedited twin, byte for byte as it printed before a plan was measured.
const UNMEASURED: &str = "\
01     Complete     2/2 plans  core
02     Planned      0/1 plans  persistence
02.1   Pending      0/0 plans  hotfix titles survive a reload
03     Pending      0/0 plans  listing
67% of plans have a summary
";

/// 01-01 with an estimate that measures nothing (no harness records its tokens): edited as a measured plan is, so its
/// verifier's verdicts are as stale, and the report differs from a measured plan's only by the estimate error.
fn unmeasured() -> Project {
    estimating(PLAN_01_01, "tokens 60000")
}

#[test]
fn progress_report_shows_the_error_of_a_measured_plan() {
    let p = estimating(PLAN_01_01, "tokens 60000 tasks 3");
    let text = report_text(&p);
    let lines: Vec<&str> = text.lines().collect();
    let phase = lines
        .iter()
        .position(|l| l.starts_with("01 "))
        .unwrap_or_else(|| panic!("phase 01's line: {text}"));
    assert_eq!(
        lines[phase + 1],
        "  estimate error 01-01: tasks -33.33%",
        "{text}"
    );
    assert_eq!(text.matches("estimate error").count(), 1, "{text}");
    // Every other line is the report of the same plan unmeasured.
    let mut rest = lines.clone();
    rest.remove(phase + 1);
    let unmeasured = report_text(&unmeasured());
    assert_eq!(rest, unmeasured.lines().collect::<Vec<_>>());

    for (estimate, said) in [
        ("tasks 2", "tasks 0%"),
        ("tasks 1", "tasks +100%"),
        ("tasks 4", "tasks -50%"),
    ] {
        let text = report_text(&estimating(PLAN_01_01, estimate));
        assert!(
            text.contains(&format!("\n  estimate error 01-01: {said}\n")),
            "{estimate}: {text}"
        );
    }
}

#[test]
fn progress_report_is_unchanged_without_a_measured_plan() {
    assert_eq!(report_text(&taskboard()), UNMEASURED);
    let text = report_text(&unmeasured());
    assert!(!text.contains("estimate error"), "{text}");
    // The JSON answer is upstream's `progress json`: a measured plan does not move it.
    let measured = estimating(PLAN_01_01, "tokens 60000 tasks 3");
    assert_eq!(report_json(&measured), report_json(&unmeasured()));
}
