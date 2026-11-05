//! A guest cannot hide what it was refused by failing (ADR 0335 D4): a command and a collector that trap after a denied
//! request still report its E090.

use serde_json::Map;
use specforge_effects::testing::{self, MemoryConsent};
use specforge_effects::{EffectsMode, HttpRequest, Ports};
use specforge_extension_sdk::host;
use specforge_extension_sdk::prelude::*;
use specforge_ops::analysis::collect::{Mode, Request, collect};
use specforge_ops::asking::Asking;
use specforge_ops::asking::testing::Scripted;
use specforge_ops::extension::command::{CommandFormat, ExtensionCommands, RunError, run};
use specforge_ops::extension::lend::port_call;
use specforge_ops::testing::ScriptedMachine;
use specforge_ops::view::ProjectView;
use specforge_ops::{Caller, Containment};
use specforge_project::testing::TestProject;
use specforge_protocol_types::Operation;
use specforge_wasm::testing::InProcessRuntime;

const EXT: &str = "@t/refused";

fn refused_then_trap() -> ! {
    let refused = host::http::send(&HttpRequest {
        method: "GET".to_string(),
        url: "https://elsewhere.test/a".to_string(),
        ..Default::default()
    });
    panic!("traps after {refused:?}");
}

fn declaration() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.capability(
        Capability::http("api")
            .hosts(&["example.test"])
            .schemes(&["https"])
            .why("call the API"),
    );
    c.command("boom", |cmd| {
        cmd.title("Boom").handler(|_| refused_then_trap());
    });
    c.collector("boom", |k| {
        k.detect_files(&["report.json"])
            .run(&["true"])
            .report("report.json")
            .collect(|_| refused_then_trap());
    });
    c
}

fn project() -> TestProject {
    let runtime = InProcessRuntime::new()
        .with(declaration)
        .importing(EXT, &["specforge:host/http@0.1.0"]);
    TestProject::new()
        .serving(runtime)
        .enabling(&[EXT])
        .file("report.json", "{}")
}

/// An approval a person gave holds for the run; the store that lost it is said, as the caller cannot redo it.
#[test]
fn an_approval_that_was_not_kept_is_reported() {
    let project = project();
    let compiled = project.compile();
    let view = ProjectView::of(&compiled);
    let ports: Ports = testing::ports();
    let consent = MemoryConsent::unable_to_remember("disk full");
    let containment = Containment::unbounded();
    let asker = Scripted::answering(true);
    let caller = Caller::new(&containment, Asking::Ask(&asker)).lending(
        &ports,
        &consent,
        EffectsMode::Online,
    );

    let call = port_call(&view, caller, EXT, Operation::Command).expect("its guest imports a port");

    let reported = call.take_denials();
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert_eq!(reported[0].code, "W186");
    assert!(reported[0].message.contains("disk full"), "{reported:?}");
}

#[test]
fn a_command_that_traps_carries_its_denial() {
    let project = project();
    let compiled = project.compile();
    let view = ProjectView::of(&compiled);
    let commands = ExtensionCommands::build(view.registries());
    let boom = commands.all().iter().find(|c| c.id() == "boom").unwrap();
    let ports: Ports = testing::ports();
    let consent = MemoryConsent::approving_everything();
    let containment = Containment::unbounded();
    let caller =
        Caller::new(&containment, Asking::Nobody).lending(&ports, &consent, EffectsMode::Online);

    let failed = run(&view, boom, &Map::new(), CommandFormat::Json, None, caller);

    let Err(RunError::Call(failed)) = failed else {
        panic!("not a failed call: {failed:?}");
    };
    assert_eq!(failed.denials.len(), 1, "{failed:?}");
    assert_eq!(failed.denials[0].code, "E090");
}

#[test]
fn a_collector_that_traps_says_what_it_was_refused() {
    let project = project();
    let compiled = project.compile();
    let view = ProjectView::of(&compiled);
    let ports: Ports = testing::ports();
    let consent = MemoryConsent::approving_everything();
    let containment = Containment::unbounded();

    let Err(refusal) = collect(
        &view,
        Request {
            runner: Some("boom"),
            mode: Mode::NoRun,
            announce: &mut |_, _| {},
            caller: Caller::new(&containment, Asking::Nobody).lending(
                &ports,
                &consent,
                EffectsMode::Online,
            ),
        },
        &ScriptedMachine::new(),
        &consent,
    ) else {
        panic!("the collector traps");
    };

    assert_eq!(refusal.error.code, "E028", "{refusal:?}");
    assert!(refusal.error.message.contains("E090"), "{refusal:?}");
}
