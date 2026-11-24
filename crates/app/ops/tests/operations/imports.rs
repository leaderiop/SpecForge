//! A command that declares `imports` copies history recorded outside the spec, so its plan is judged for host errors
//! only, as a collector's (ADR 0392, ADR 0303): an extension's rule error lands, and a host error still refuses the
//! whole plan. A command that does not declare it is refused for both (E080). `@t/core`'s `import` declares it,
//! `record` does not; the guard pass reports E087 while a phase's goal is [`GUARDED`].

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_project::testing::{CORE, GUARDED, GsdWrites, TestProject, gsd_writes_answer};
use specforge_test_macros::test as specforge_test;

use crate::plans::{Ran, compiled, project, revision, run_command, set};

/// The plan that leaves the project with an extension's error: phase `03`'s goal set to [`GUARDED`].
fn guarded(p: &TestProject) -> Value {
    json!([set(
        "v1.0/03",
        "goal",
        json!(GUARDED),
        &revision(p, "m.spec")
    )])
}

/// The plan that leaves the project with a host error: a decision whose phase is no entity.
fn dangling() -> Value {
    json!([{"op": "create", "kind": "decision", "id": "D-01", "title": "D", "file": "d.spec",
            "revision": "absent", "fields": {"phase": "99", "origin": "context"}}])
}

/// `@t/core`'s command `id` answering `ops`, its guard on.
fn ran(p: &TestProject, id: &str, ops: Value) -> Ran {
    let runtime = Arc::new(
        GsdWrites::new()
            .guarding_a_goal()
            .answering(CORE, &format!("cmd__{id}"), gsd_writes_answer(0, ops))
            .runtime(),
    );
    run_command(p, &compiled(p, &runtime), &runtime, "core", id)
}

/// The code a plan was refused with, if it was.
fn refusal(ran: &Ran) -> Option<&str> {
    ran.refusal.as_ref().map(|(code, _)| code.as_str())
}

#[specforge_test(requirement = "FEATURE-131")]
fn an_imports_plan_keeps_an_extensions_error() {
    let p = project();
    let ran = ran(&p, "import", guarded(&p));

    assert_eq!(refusal(&ran), None);
    assert_eq!(ran.wrote, ["m.spec"]);
    let written = std::fs::read_to_string(p.root().join("m.spec")).unwrap();
    assert!(written.contains(GUARDED), "{written}");
}

#[specforge_test(requirement = "FEATURE-131")]
fn an_imports_plan_that_adds_a_host_error_is_refused() {
    let p = project();
    let ran = ran(&p, "import", dangling());

    assert_eq!(refusal(&ran), Some("E080"));
    assert!(ran.wrote.is_empty(), "{:?}", ran.wrote);
    assert!(!p.root().join("d.spec").exists());
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_command_that_does_not_declare_imports_is_refused_for_both() {
    let p = project();
    let extension = ran(&p, "record", guarded(&p));
    let host = ran(&p, "record", dangling());

    assert_eq!(refusal(&extension), Some("E080"));
    assert_eq!(refusal(&host), Some("E080"));
    assert!(extension.wrote.is_empty() && host.wrote.is_empty());
}
