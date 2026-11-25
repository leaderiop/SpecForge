//! A command that declares `imports` copies history recorded outside the spec, so its plan is judged for host errors
//! only (ADR 0392): an extension's rule error about authored entities lands; a host error, a first-party
//! (`@specforge/*`) extension's error and one that judges collected facts still refuse the whole plan. A collector's
//! plan (ADR 0303) lands that first-party error too; an import's does not. A command that does not declare it is
//! refused for every error (E080). `@t/core`'s `import` declares it, `record` does not; the guard pass reports E087
//! while a phase's goal is [`GUARDED`], and E901, a code that judges collected facts, while another phase's goal is
//! [`RACED`] too.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_project::testing::{
    CORE, GSD_WRITES_EXTENSIONS, GUARDED, GsdWrites, RACED, TestProject, component_runtime,
    gsd_writes_answer,
};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;

use crate::plans::{MILESTONE, Ran, compiled, project, revision, run_command, set};

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

/// The runtime in which `@t/core`'s command `id` answers `ops`, its guard on.
fn runtime(id: &str, ops: Value) -> Arc<InProcessRuntime> {
    Arc::new(
        GsdWrites::new()
            .guarding_a_goal()
            .answering(CORE, &format!("cmd__{id}"), gsd_writes_answer(0, ops))
            .runtime(),
    )
}

/// `@t/core`'s command `id` answering `ops`, its guard on.
fn ran(p: &TestProject, id: &str, ops: Value) -> Ran {
    let runtime = runtime(id, ops);
    run_command(p, &compiled(p, &runtime), &runtime, "core", id)
}

/// `import` answering [`guarded`], another writer landing `z.spec` (`landed`) after the compile: the plan is judged
/// again over the disk, under the write lock.
fn imported_after(p: &TestProject, landed: &str) -> Ran {
    let runtime = runtime("import", guarded(p));
    let compiled = compiled(p, &runtime);
    std::fs::write(p.root().join("z.spec"), landed).unwrap();
    run_command(p, &compiled, &runtime, "core", "import")
}

/// The code a plan was refused with, if it was.
fn refusal(ran: &Ran) -> Option<&str> {
    ran.refusal.as_ref().map(|(code, _)| code.as_str())
}

/// The codes of the errors a refusal (E080) names.
fn refused_codes(ran: &Ran) -> Vec<String> {
    let Some((_, data)) = &ran.refusal else {
        return Vec::new();
    };
    data["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| d["code"].as_str().map(str::to_string))
        .collect()
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

/// A first-party extension's error refuses an import (ADR 0392, ADR 0391 D9): a phase's goal is a file
/// `@specforge/checks` reads, and the import names one that does not exist, so `@specforge/checks` reports E016 with
/// its origin and a bare code.
#[specforge_test(requirement = "FEATURE-131")]
fn an_imports_plan_that_adds_a_first_party_extensions_error_is_refused() {
    let mut extensions = GSD_WRITES_EXTENSIONS.to_vec();
    extensions.push("@specforge/checks");
    let p = TestProject::new()
        .config(|c| {
            c["name"] = "p".into();
            c["checks"] = json!({ "fields": { "phase.goal": ["@specforge/checks:file"] } });
        })
        .enabling(&extensions)
        .installing_enabled()
        .file("m.spec", MILESTONE);
    let ops = json!([set(
        "v1.0/03",
        "goal",
        json!("missing.md"),
        &revision(&p, "m.spec")
    )]);
    let runtime = Arc::new(
        GsdWrites::new()
            .answering(CORE, "cmd__import", gsd_writes_answer(0, ops))
            .runtime()
            .beside(component_runtime()),
    );
    let ran = run_command(&p, &compiled(&p, &runtime), &runtime, "core", "import");

    assert_eq!(refusal(&ran), Some("E080"));
    assert_eq!(refused_codes(&ran), ["E016"]);
    assert!(ran.wrote.is_empty(), "{:?}", ran.wrote);
}

/// The re-judgement under the write lock lands what the plan's check landed: another writer's file moved the disk
/// after the compile, and the import's extension rule error is still written.
#[specforge_test(requirement = "FEATURE-131")]
fn the_re_judge_under_the_lock_keeps_an_imports_rule_error() {
    let p = project();
    let ran = imported_after(&p, "phase 04 \"Four\" {\n  milestone v1.0\n}\n");

    assert_eq!(refusal(&ran), None);
    let written = std::fs::read_to_string(p.root().join("m.spec")).unwrap();
    assert!(written.contains(GUARDED), "{written}");
}

/// The re-judgement under the write lock refuses what an import may not land: only the disk as the other writer
/// left it (a phase whose goal is [`RACED`]) adds E901, a code that judges collected facts, so only that judgement
/// sees it, and a collector's rule there would have written it.
#[specforge_test(requirement = "FEATURE-131")]
fn the_re_judge_under_the_lock_refuses_what_an_import_may_not_land() {
    let p = project();
    let landed = format!("phase 04 \"Four\" {{\n  milestone v1.0\n  goal \"{RACED}\"\n}}\n");
    let ran = imported_after(&p, &landed);

    assert_eq!(refusal(&ran), Some("E080"));
    assert_eq!(refused_codes(&ran), ["t/E901"]);
    let written = std::fs::read_to_string(p.root().join("m.spec")).unwrap();
    assert!(!written.contains(GUARDED), "{written}");
}
