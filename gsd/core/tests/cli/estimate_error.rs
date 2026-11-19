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
fn error_of(p: &Project, id: &str) -> Value {
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
