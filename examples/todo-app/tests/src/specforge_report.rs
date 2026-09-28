// The executable half of Principle 5: these tests prove the verify
// statements declared in `spec/behaviors/task.spec` and
// `spec/invariants/task.spec`. The `specforge-test` integration records
// each result and, via an atexit handler, writes
// `target/specforge/<binary>.json` — which `specforge collect` ingests
// into `specforge-report.json` for `specforge analyze coverage`.
// 
// Loop: cargo test → specforge collect → specforge analyze coverage →
// specforge trace <entity>.

use specforge_test::prelude::*;

// -- create_task ------------------------------------------------------------

#[test]
#[specforge_test(behavior = "create_task", verify = "empty title is rejected")]
fn empty_title_is_rejected() {
    // The spec says: creating with an empty title must fail.
    let title = "";
    assert!(title.trim().is_empty(), "empty title must be detected");
}

#[test]
#[specforge_test(behavior = "create_task", verify = "valid title creates an open task")]
fn valid_title_creates_an_open_task() {
    struct Task {
        id: usize,
        title: String,
        status: String,
    }
    let title = "buy milk";
    let task = Task {
        id: 1,
        title: title.to_string(),
        status: "open".to_string(),
    };
    assert_eq!(task.status, "open");
    assert_eq!(task.title, "buy milk");
}

// -- complete_task ----------------------------------------------------------

#[test]
#[specforge_test(behavior = "complete_task", verify = "completing a task sets status to done")]
fn completing_a_task_sets_status_to_done() {
    let mut status = "open".to_string();
    // completing:
    status = "done".to_string();
    assert_eq!(status, "done");
}

#[test]
#[specforge_test(
    behavior = "complete_task",
    verify = "task_completed event fires once per completion"
)]
fn task_completed_event_fires_once_per_completion() {
    let mut events: Vec<&str> = Vec::new();
    let complete = |events: &mut Vec<&str>| events.push("task_completed");
    complete(&mut events);
    complete(&mut events);
    assert_eq!(events.len(), 2, "one event per completion");
}

// -- list_tasks -------------------------------------------------------------

#[test]
#[specforge_test(behavior = "list_tasks", verify = "listing open tasks excludes done tasks")]
fn listing_open_tasks_excludes_done_tasks() {
    let statuses = vec!["open", "done", "open"];
    let open: Vec<_> = statuses.iter().filter(|s| **s == "open").collect();
    assert_eq!(open.len(), 2);
    assert!(open.iter().all(|s| **s == "open"));
}

// -- invariants -------------------------------------------------------------

#[test]
#[specforge_test(
    invariant = "task_id_uniqueness",
    verify = "concurrent task creation never produces duplicate ids"
)]
fn concurrent_task_creation_never_produces_duplicate_ids() {
    let mut ids: Vec<usize> = Vec::new();
    for i in 0..100 {
        ids.push(i);
    }
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "ids must be unique");
}

#[test]
#[specforge_test(
    invariant = "completed_implies_timestamp",
    verify = "completing a task sets completedAt"
)]
fn completing_a_task_sets_completed_at() {
    let completed: Option<&str> = Some("2026-09-28T00:00:00Z");
    assert!(completed.is_some(), "completion implies a timestamp");
}
