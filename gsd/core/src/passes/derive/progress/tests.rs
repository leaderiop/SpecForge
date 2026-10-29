use super::*;

fn statuses<'a>(of: &[(&'a str, &'static str)]) -> Statuses<'a> {
    of.iter().copied().collect()
}

#[test]
fn a_requirement_is_complete_once_a_plan_naming_it_completes() {
    let s = statuses(&[("a", "complete"), ("b", "ready")]);
    assert_eq!(requirement_status(&["a", "b"], &s), "complete");
}

#[test]
fn a_requirement_of_an_executing_plan_is_in_progress() {
    let s = statuses(&[("a", "executing"), ("b", "ready")]);
    assert_eq!(requirement_status(&["a", "b"], &s), "in_progress");
    assert_eq!(requirement_status(&["b"], &s), "pending");
    assert_eq!(requirement_status(&[], &s), "pending");
}

#[test]
fn a_requirement_whose_plans_all_halted_is_blocked() {
    let s = statuses(&[("a", "halted"), ("b", "blocked"), ("c", "ready")]);
    assert_eq!(requirement_status(&["a", "b"], &s), "blocked");
    // One plan that can still run keeps it pending.
    assert_eq!(requirement_status(&["a", "c"], &s), "pending");
}

#[test]
fn milestone_progress_percent_is_the_smaller_fraction() {
    // 2 of 3 plans, 1 of 4 phases: the phases are the smaller side.
    assert_eq!(percent(2, 3, 1, 4), Some(25));
    // 2 of 3 plans and no phase data: the missing side counts 1, so the plans decide (rounded).
    assert_eq!(percent(2, 3, 0, 0), Some(67));
    assert_eq!(percent(0, 0, 3, 4), Some(75));
    assert_eq!(percent(0, 0, 0, 0), None);
    assert_eq!(percent(0, 2, 0, 1), Some(0));
}

fn commit(
    plan: &str,
    kind: &str,
    at: &str,
    files: &[&str],
    insertions: i64,
    deletions: i64,
) -> Commit {
    Commit {
        kind: kind.into(),
        plan: Some(plan.into()),
        at: at.into(),
        files: files.iter().map(|f| f.to_string()).collect(),
        insertions,
        deletions,
        ..Default::default()
    }
}

#[test]
fn milestone_stats_and_git_range_come_from_commits() {
    let commits = [
        commit(
            "v1.0/01/01-01",
            "feat",
            "2026-10-28T08:00:00Z",
            &["a", "b"],
            10,
            1,
        ),
        commit(
            "v1.0/01/01-01",
            "test",
            "2026-10-28T09:00:00Z",
            &["b"],
            5,
            0,
        ),
        commit(
            "v1.0/01/01-02",
            "feat",
            "2026-10-30T09:00:00Z",
            &["c"],
            7,
            2,
        ),
    ];
    let all: Vec<&Commit> = commits.iter().collect();
    assert_eq!(
        git_range(&all).as_deref(),
        Some("`feat(01-01)` → `feat(01-02)`")
    );
    assert_eq!(git_range(&all[1..2]), None);
    let input = PassInput::default();
    let plans = ["v1.0/01/01-01", "v1.0/01/01-02"];
    // Without a completion, the days run to the last commit; with one, to its date.
    assert_eq!(
        stats(&input, &plans, &all, None, 1),
        json!({"phases": 1, "plans": 2, "tasks": 0, "files": 3, "loc": 25, "days": 2})
    );
    assert_eq!(
        stats(&input, &plans, &all, Some("2026-11-01"), 1)["days"],
        3
    );
    // No commit: the counts only.
    assert_eq!(
        stats(&input, &plans, &[], None, 1),
        json!({"phases": 1, "plans": 2, "tasks": 0})
    );
}
