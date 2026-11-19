//! A phase's critical path, end to end on the `plan` project (ADR 0378 D4): the heaviest chain of its plans along
//! the strict waves' edges, named task by task, and its weight.

use crate::harness::{Project, project};

/// `(critical_path, critical_weight)` of phase `id`, `None` for a field the phase does not have.
fn critical(p: &Project, id: &str) -> (Option<Vec<String>>, Option<i64>) {
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
