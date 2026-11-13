//! The plan family's shell lines, run as the assets hold them on a taskboard copy under bash with only `specforge` on
//! the `PATH`.

use super::{Shell, asset, fence_after};
use crate::harness::taskboard;

/// The planner's plan check reads `valid`, `missing`, `invalidValue` and `task_count` from
/// `specforge check --format json`, which prints a bare array of diagnostics: none of those keys exists, so the step
/// reads nothing it names.
#[test]
fn the_plan_check_step_reads_keys_the_check_answer_lacks() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let text = asset("agents/gsd-planner.md");
    let fence = fence_after(&text, "<step name=\"validate_plan\">");
    let (out, err, _) = sh.run_in(p.root(), &format!("{fence}\nprintf '%s' \"$VALID\""));
    let answer: serde_json::Value =
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("not JSON ({e}): {out}{err}"));
    let items = answer.as_array().expect("a bare array of diagnostics");
    assert!(!items.is_empty(), "{out}{err}");
    for item in items {
        assert!(item.get("code").is_some(), "{item}");
        for key in ["valid", "missing", "invalidValue", "task_count"] {
            assert!(item.get(key).is_none(), "{key} in {item}");
        }
    }
    assert!(text.contains("valid=false"), "the step still reads `valid`");
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
