//! Storing a collect's facts (ADR 0327) edits no source, so it rewrites the ledger over the project the collect
//! compiled: the project is checked once. Counted by a check pass the fixture adds, never timed.

use std::sync::Arc;

use specforge_effects::SharedGit;
use specforge_effects::testing::FakeGit;
use specforge_ops::analysis::collect::{self, Mode, Request};
use specforge_ops::authoring::apply::apply_collected;
use specforge_ops::testing::{ScriptedMachine, caller};
use specforge_ops::view::ProjectView;
use specforge_project::testing::{
    COLLECTED_EXTENSIONS, Collected, NODE, NODE_REPORT, TestProject, UNIT_REPORT, collected_answer,
};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;

use super::facts::{SHA, ledger, nothing, two_tests};

/// `@t/counter`: one check pass, `counted`, called once each time the project is checked.
const COUNTER: &str = "@t/counter";

fn counter() -> specforge_extension_sdk::prelude::ContributionsBuilder {
    use specforge_extension_sdk::prelude::*;
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(COUNTER, "1.0.0"));
    c.pass("counted", |p| {
        p.phase("check").run(|_| Vec::new());
    });
    c
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "storing a collect's facts checks the project no second time"
)]
fn storing_a_collects_facts_checks_the_project_once() {
    let mut extensions = COLLECTED_EXTENSIONS.to_vec();
    extensions.push(COUNTER);
    let p = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&extensions)
        .installing_enabled()
        .file("truths.spec", "truth T1 \"One\" {}\n")
        .file("package.json", "{}")
        .file(NODE_REPORT, "<testsuites/>")
        .file(UNIT_REPORT, "{}");
    let git: SharedGit = Arc::new(FakeGit::repo(p.root(), SHA, Vec::new()));
    let runtime = Arc::new(
        Collected::new()
            .answering(NODE, "collect__node", collected_answer(two_tests()))
            .answering(NODE, "collect__unit", nothing())
            .over(InProcessRuntime::new().with(counter)),
    );
    let compiled = p.compile_with(runtime.clone(), Some(Arc::clone(&git)));
    let checks = || {
        runtime
            .calls()
            .iter()
            .filter(|call| call.export == "__pass_counted")
            .count()
    };
    assert_eq!(checks(), 1, "the compile checks the project once");
    let view = ProjectView::of(&compiled);
    let mut outcome = collect::collect(
        &view,
        Request {
            runner: Some("node"),
            mode: Mode::NoRun,
            announce: &mut |_, _| {},
            caller: caller(),
        },
        &ScriptedMachine::new(),
        &specforge_effects::testing::MemoryConsent::default(),
    )
    .expect("the collect runs");
    let facts = std::mem::take(&mut outcome.facts);
    apply_collected(&view, Vec::new(), facts, false, caller()).expect("the facts are stored");
    assert!(ledger(&p).contains("node facts:"), "{}", ledger(&p));
    // The store edits no source: the ledger is rewritten over the compiled project, not a second build of it.
    assert_eq!(checks(), 1);
}
