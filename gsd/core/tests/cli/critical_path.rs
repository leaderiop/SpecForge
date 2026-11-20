//! A phase's critical path, end to end on the `plan` project (ADR 0378 D4): the heaviest chain of its plans along
//! the strict waves' edges, named task by task, and its weight.

use crate::harness::{Project, project};
use crate::strict_wave::with_a_cycle;

/// `(critical_path, critical_weight)` of phase `id`, `None` for a field the phase does not have.
pub(super) fn critical(p: &Project, id: &str) -> (Option<Vec<String>>, Option<i64>) {
    let fields = p.fields_of(id);
    let path = fields["critical_path"].as_array().map(|ids| {
        ids.iter()
            .map(|id| id.as_str().expect("a task id").to_string())
            .collect()
    });
    (path, fields["critical_weight"].as_i64())
}

fn path(ids: &[&str]) -> Option<Vec<String>> {
    Some(ids.iter().map(|id| (*id).to_string()).collect())
}

#[test]
fn the_chain_of_a_phase_names_each_task() {
    let p = project("plan");
    assert_eq!(
        critical(&p, "v1.0/01"),
        (
            path(&["v1.0/01/01-01/T1", "v1.0/01/01-01/T2", "v1.0/01/01-02/T1"]),
            Some(3)
        )
    );
    assert_eq!(
        critical(&p, "v1.0/02"),
        (path(&["v1.0/02/02-01/T1"]), Some(1))
    );
}

/// Project `plan` with 01-02's `depends_on [01-01]` dropped: the two plans of phase 01 are independent.
fn without_the_dependency() -> Project {
    let p = project("plan");
    p.edit("spec/plans.spec", "  depends_on     [01-01]\n", "");
    p
}

/// `p` with `estimate { tokens <tokens> }` on 01-02.
fn estimating(p: &Project, tokens: i64) {
    p.edit(
        "spec/plans.spec",
        "  objective      \"A task can be marked complete\"\n",
        &format!(
            "  objective      \"A task can be marked complete\"\n  estimate       {{ tokens {tokens} }}\n"
        ),
    );
}

#[test]
fn a_dropped_dependency_moves_the_path() {
    let p = without_the_dependency();
    // 01-01 weighs its two tasks, 01-02 its one: the heavier plan alone.
    assert_eq!(
        critical(&p, "v1.0/01"),
        (path(&["v1.0/01/01-01/T1", "v1.0/01/01-01/T2"]), Some(2))
    );
}

#[test]
fn a_shared_path_restores_the_chain() {
    let p = without_the_dependency();
    p.edit(
        "spec/plans.spec",
        "  files_modified [\"src/complete.js\"]\n",
        "  files_modified [\"src/complete.js\", \"src/tasks.js\"]\n",
    );
    // 01-02 shares `src/tasks.js` with 01-01: it runs after it, so the chain holds both again.
    assert_eq!(
        critical(&p, "v1.0/01"),
        (
            path(&["v1.0/01/01-01/T1", "v1.0/01/01-01/T2", "v1.0/01/01-02/T1"]),
            Some(3)
        )
    );
    let plan = p.fields_of("v1.0/01/01-02");
    assert_eq!(
        (plan["wave"].as_i64(), plan["strict_wave"].as_i64()),
        (Some(1), Some(2))
    );
}

#[test]
fn an_estimate_weighs_a_plan_by_its_tokens() {
    let p = without_the_dependency();
    estimating(&p, 5);
    assert_eq!(
        critical(&p, "v1.0/01"),
        (path(&["v1.0/01/01-02/T1"]), Some(5))
    );
}

#[test]
fn ties_break_by_plan_id() {
    // 01-02 estimates 2 tokens, 01-01 weighs its 2 tasks: of the two equal chains, the smaller plan id.
    let p = without_the_dependency();
    estimating(&p, 2);
    assert_eq!(
        critical(&p, "v1.0/01"),
        (path(&["v1.0/01/01-01/T1", "v1.0/01/01-01/T2"]), Some(2))
    );
}

#[test]
fn a_zero_weight_plan_stays_on_the_path() {
    // 01-02 still runs after 01-01 and weighs 0: the longer chain of the same weight keeps it.
    let p = project("plan");
    estimating(&p, 0);
    assert_eq!(
        critical(&p, "v1.0/01"),
        (
            path(&["v1.0/01/01-01/T1", "v1.0/01/01-01/T2", "v1.0/01/01-02/T1"]),
            Some(2)
        )
    );
}

#[test]
fn a_cycle_leaves_no_critical_path() {
    // 01-01 and 01-02 depend on each other: phase 01 has no placement, phase 02 keeps its own.
    let p = with_a_cycle();
    assert_eq!(critical(&p, "v1.0/01"), (None, None));
    assert_eq!(
        critical(&p, "v1.0/02"),
        (path(&["v1.0/02/02-01/T1"]), Some(1))
    );
}
