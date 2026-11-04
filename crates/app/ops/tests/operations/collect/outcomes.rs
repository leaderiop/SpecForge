//! A collector that declares `check_outcomes` (ADR 0336): its evidence is the served rows, so it needs no command and
//! no report; it runs when named or under `checks.publish`, and its facts take R1's path unchanged.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_effects::SharedGit;
use specforge_effects::testing::{FakeGit, MemoryConsent};
use specforge_extension_sdk::prelude::*;
use specforge_ops::analysis::collect::{self, Mode, Outcome, Request, RunnerOutput};
use specforge_ops::authoring::apply::apply_collected;
use specforge_ops::testing::{ScriptedMachine, caller};
use specforge_ops::view::ProjectView;
use specforge_project::facts::Store;
use specforge_project::testing::TestProject;
use specforge_project::{CompiledProject, SharedRuntime};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;

const EXT: &str = "@pin/published";
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn declare() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.checker("plain", |k| {
        k.applies_to(&["string"])
            .plan(|_: &CheckItem| PlannedItem::findings(Vec::new()));
    });
    c.kind("service", |k| {
        k.keyword("service");
        k.field("home", |f| {
            f.field_type(FieldType::String).check("plain");
        });
    });
    c.kind("seen", |k| {
        k.keyword("seen").collected(&["seen"]);
        k.field("status", |f| {
            f.field_type(FieldType::String);
        });
    });
    c.collector("outcomes", |k| {
        k.input(Input::CheckOutcomes)
            .collect(|input: &CollectInput| {
                let facts = input.check_outcomes.iter().map(|row| {
                    json!({"kind": "seen", "id": format!("seen:{}", row.entity),
                           "fields": {"status": row.status.as_str()}})
                });
                let facts: Value = facts.collect();
                Ok(CollectOutput {
                    facts: serde_json::from_value(facts).map_err(|e| e.to_string())?,
                    ..Default::default()
                })
            });
    });
    c
}

/// `declare`, and a runner reading nothing.
fn declare_with_a_runner() -> ContributionsBuilder {
    let mut c = declare();
    c.collector("runner", |k| {
        k.collect(|_: &CollectInput| Ok(CollectOutput::default()));
    });
    c
}

fn project(publish: bool) -> TestProject {
    TestProject::new()
        .config(move |c| {
            c["name"] = "p".into();
            c["checks"] = json!({"publish": publish});
        })
        .enabling(&[EXT])
        .installing_enabled()
        .file("a.spec", "service a \"A\" {\n  home \"fine\"\n}\n")
}

fn compiled(project: &TestProject, runtime: &Arc<InProcessRuntime>) -> CompiledProject {
    project.compile_in(Arc::clone(runtime) as SharedRuntime)
}

/// A `collect` of `compiled` in `mode`, naming `runner`.
fn collected(
    compiled: &CompiledProject,
    runner: Option<&str>,
    mode: Mode<'_>,
    machine: &ScriptedMachine,
) -> Result<Outcome, specforge_ops::Refusal<collect::CollectField>> {
    collect::collect(
        &ProjectView::of(compiled),
        Request {
            runner,
            mode,
            announce: &mut |_, _| {},
            caller: caller(),
        },
        machine,
        &MemoryConsent::default(),
    )
}

/// The inputs of every call of the collector.
fn calls(runtime: &InProcessRuntime) -> Vec<Value> {
    runtime
        .calls()
        .into_iter()
        .filter(|call| call.export == "collect__outcomes")
        .map(|call| call.input)
        .collect()
}

#[specforge_test(
    behavior = "auto_detect_collector",
    verify = "checks.publish adds a collector reading check outcomes to a plain collect, and never to a reports run"
)]
fn checks_publish_runs_it_on_a_plain_collect() {
    for (publish, expected) in [(true, 1), (false, 0)] {
        let project = project(publish);
        let runtime = Arc::new(InProcessRuntime::new().with(declare));
        let compiled = compiled(&project, &runtime);
        let _ = collected(&compiled, None, Mode::NoRun, &ScriptedMachine::new());
        assert_eq!(calls(&runtime).len(), expected, "publish {publish}");
    }
}

#[specforge_test(
    behavior = "auto_detect_collector",
    verify = "a collect owes the values phase only when a selected collector reads check outcomes"
)]
fn a_collect_owes_the_values_phase_only_for_a_reader() {
    let reads = |publish, runner, mode: &Mode<'_>| {
        let project = project(publish);
        let runtime = Arc::new(InProcessRuntime::new().with(declare_with_a_runner));
        let compiled = compiled(&project, &runtime);
        collect::reads_outcomes(compiled.environment(), project.root(), runner, mode)
    };
    assert!(reads(true, None, &Mode::NoRun));
    assert!(reads(false, Some("outcomes"), &Mode::NoRun));
    assert!(!reads(false, None, &Mode::NoRun));
    assert!(!reads(true, Some("runner"), &Mode::NoRun));
    assert!(
        !reads(true, None, &Mode::Reports(&[])),
        "never a reports run"
    );
    assert!(!reads(false, Some("unknown"), &Mode::NoRun), "it refuses");
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "a collector named by the runner receives each checked value's outcome"
)]
fn runner_names_it_and_it_receives_the_outcomes() {
    let project = project(false);
    let runtime = Arc::new(InProcessRuntime::new().with(declare));
    let compiled = compiled(&project, &runtime);

    collected(
        &compiled,
        Some("outcomes"),
        Mode::NoRun,
        &ScriptedMachine::new(),
    )
    .unwrap();

    let input = calls(&runtime).remove(0);
    let row = &input["check_outcomes"][0];
    assert_eq!(row["entity"], "a", "{input}");
    assert_eq!(row["field"], "home", "{input}");
    assert_eq!(row["value"], "fine", "{input}");
    assert_eq!(row["checker"], format!("{EXT}:plain"), "{input}");
    assert_eq!(row["status"], "passed", "{input}");
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "a collector reading check outcomes needs no command and no report"
)]
fn a_check_outcomes_collector_needs_no_command_or_report() {
    for mode in [Mode::NoRun, Mode::Run(RunnerOutput::Inherit)] {
        let project = project(false);
        let runtime = Arc::new(InProcessRuntime::new().with(declare));
        let compiled = compiled(&project, &runtime);
        let machine = ScriptedMachine::new();

        collected(&compiled, Some("outcomes"), mode, &machine).unwrap();

        assert_eq!(calls(&runtime).len(), 1);
        assert!(machine.runs().is_empty(), "{:?}", machine.runs());
    }
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "published outcomes are stored under the collect key"
)]
fn published_outcomes_are_stored_under_the_collect_key() {
    let project = project(true);
    let git: SharedGit = Arc::new(FakeGit::repo(project.root(), SHA, Vec::new()));
    let runtime = InProcessRuntime::new().with(declare);
    let compiled = project.compile_with(Arc::new(runtime), Some(Arc::clone(&git)));
    let view = ProjectView::of(&compiled);

    let mut outcome = collected(&compiled, None, Mode::NoRun, &ScriptedMachine::new()).unwrap();
    let facts = std::mem::take(&mut outcome.facts);
    apply_collected(&view, Vec::new(), facts, false, caller()).expect("the facts are stored");

    let bytes = std::fs::read(project.root().join(Store::path("outcomes", SHA))).unwrap();
    let stored: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(stored["facts"][0]["id"], "seen:a", "{stored}");
    assert_eq!(stored["facts"][0]["fields"]["status"], "passed", "{stored}");
}
