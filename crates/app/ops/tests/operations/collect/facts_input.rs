//! A collector's `facts` input (ADR 0345 D3): it runs after the collectors that answer a kind it reads and is served
//! their admitted facts of it; a kind no collector of the collect answers is served from the recorded stores; a cycle
//! is E119 before anything runs. The fixture is `@t/chain`: `items` reads `package` facts, `packages` answers them.

use std::sync::{Arc, Mutex};

use serde_json::json;
use specforge_extension_sdk::prelude::*;
use specforge_ops::Refusal;
use specforge_ops::analysis::collect::{CollectField, Mode, Outcome, Request, collect};
use specforge_ops::testing::{ScriptedMachine, caller};
use specforge_ops::view::ProjectView;
use specforge_project::testing::{TestProject, stored};
use specforge_protocol_types::Fact;
use specforge_wasm::testing::InProcessRuntime;

const EXT: &str = "@t/chain";
const KEY: &str = "0123456789abcdef0123456789abcdef01234567";

/// What each collector was served, one entry per call: `(collector, the ids of its input's facts)`.
type Log = Mutex<Vec<(String, Vec<String>)>>;

/// The package facts `packages` answers.
type Answer = fn() -> Vec<Fact>;

fn two_packages() -> Vec<Fact> {
    vec![Fact::new("package", "pkg:b"), Fact::new("package", "pkg:a")]
}

fn refused_packages() -> Vec<Fact> {
    vec![
        Fact::new("package", "pkg:a"),
        Fact::new("package", "nope:x"),
    ]
}

/// `@t/chain`: the collected kinds `package` and `item`; `items` (declared first) reads `package` facts and answers
/// `item`, `packages` answers `package` (and with `cycle`, reads `item` facts). Both read through their ports, are
/// detected by `chain.cfg`, and log what they are served.
fn chain(log: &'static Log, answer: Answer, cycle: bool) -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.kind("package", |k| {
        k.collected(&["pkg"]);
    });
    c.kind("item", |k| {
        k.collected(&["itm"]);
    });
    let served = move |name: &str, input: &CollectInput| {
        let ids = input.facts.iter().map(|f| f.id.clone()).collect();
        log.lock().unwrap().push((name.to_string(), ids));
    };
    c.collector("items", move |k| {
        k.detect_files(&["chain.cfg"])
            .input(Input::facts(&["package"]))
            .answers(&["item"])
            .collect(move |input| {
                served("items", input);
                Ok(CollectOutput {
                    facts: vec![Fact::new("item", "itm:1")],
                    ..CollectOutput::default()
                })
            });
    });
    c.collector("packages", move |k| {
        k.detect_files(&["chain.cfg"]).answers(&["package"]);
        if cycle {
            k.input(Input::facts(&["item"]));
        }
        k.collect(move |input| {
            served("packages", input);
            Ok(CollectOutput {
                facts: answer(),
                ..CollectOutput::default()
            })
        });
    });
    c
}

fn chained(log: &'static Log, answer: Answer, cycle: bool) -> (TestProject, InProcessRuntime) {
    let runtime = InProcessRuntime::new().with(move || chain(log, answer, cycle));
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .file("chain.cfg", "");
    (project, runtime)
}

fn collected(
    project: &TestProject,
    runtime: InProcessRuntime,
    runner: Option<&str>,
) -> Result<Outcome, Refusal<CollectField>> {
    let compiled = project.compile_in(Arc::new(runtime));
    let view = ProjectView::of(&compiled);
    collect(
        &view,
        Request {
            runner,
            mode: Mode::NoRun,
            announce: &mut |_, _| {},
            caller: caller(),
        },
        &ScriptedMachine::new(),
        &specforge_effects::testing::MemoryConsent::default(),
    )
}

fn served(log: &Log) -> Vec<(String, Vec<String>)> {
    log.lock().unwrap().clone()
}

#[specforge_test_macros::test(
    behavior = "auto_detect_collector",
    verify = "a collector whose facts input names a kind another answers runs after it"
)]
fn a_consumer_declared_first_runs_after_its_producer() {
    static LOG: Log = Mutex::new(Vec::new());
    let (project, runtime) = chained(&LOG, two_packages, false);

    let outcome = collected(&project, runtime, None).unwrap();

    let ran: Vec<&str> = outcome.runners.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(ran, ["packages", "items"]);
    let order: Vec<String> = served(&LOG).into_iter().map(|(name, _)| name).collect();
    assert_eq!(order, ["packages", "items"]);
}

#[specforge_test_macros::test(
    behavior = "dispatch_collector",
    verify = "a facts input is served the admitted facts of its kinds that earlier collectors answered, sorted"
)]
fn a_consumer_receives_exactly_the_admitted_facts_of_its_kinds() {
    static LOG: Log = Mutex::new(Vec::new());
    let (project, runtime) = chained(&LOG, two_packages, false);

    collected(&project, runtime, None).unwrap();

    assert_eq!(
        served(&LOG),
        [
            ("packages".to_string(), vec![]),
            (
                "items".to_string(),
                vec!["pkg:a".to_string(), "pkg:b".to_string()]
            ),
        ]
    );

    // Facts the host refuses (E116) are served to no one, and a recorded store does not stand in for them.
    static REFUSED: Log = Mutex::new(Vec::new());
    let (project, runtime) = chained(&REFUSED, refused_packages, false);
    stored(
        project.root(),
        "packages",
        KEY,
        json!([{"kind": "package", "id": "pkg:recorded"}]),
    );

    let outcome = collected(&project, runtime, None).unwrap();

    assert!(outcome.diagnostics.iter().any(|d| d.code == "E116"));
    assert_eq!(served(&REFUSED)[1], ("items".to_string(), vec![]));
}

#[specforge_test_macros::test(
    behavior = "dispatch_collector",
    verify = "a facts input no collector of the collect answers is served the recorded stores"
)]
fn a_consumer_run_alone_is_served_the_recorded_stores() {
    static LOG: Log = Mutex::new(Vec::new());
    let (project, runtime) = chained(&LOG, two_packages, false);
    stored(
        project.root(),
        "packages",
        KEY,
        json!([
            {"kind": "package", "id": "pkg:z"},
            {"kind": "package", "id": "pkg:y"},
        ]),
    );

    let outcome = collected(&project, runtime, Some("items")).unwrap();

    assert_eq!(outcome.runners.len(), 1);
    assert_eq!(
        served(&LOG),
        [(
            "items".to_string(),
            vec!["pkg:y".to_string(), "pkg:z".to_string()]
        )]
    );
}

#[specforge_test_macros::test(
    behavior = "auto_detect_collector",
    verify = "collectors whose facts inputs form a cycle are E119 before any of them runs"
)]
fn collectors_whose_facts_inputs_form_a_cycle_are_e119_before_anything_runs() {
    static LOG: Log = Mutex::new(Vec::new());
    let (project, runtime) = chained(&LOG, two_packages, true);

    let refusal = collected(&project, runtime, None).unwrap_err();

    assert_eq!(refusal.error.code, "E119", "{refusal:?}");
    assert!(
        refusal.error.message.contains("'items', 'packages'"),
        "{refusal:?}"
    );
    assert!(served(&LOG).is_empty());
    assert!(!project.root().join("specforge-report.json").exists());
}

#[specforge_test_macros::test(
    behavior = "dispatch_collector",
    verify = "a facts input served no fact of a kind is W201 naming the collectors that answer it"
)]
fn a_consumer_run_alone_with_nothing_recorded_is_w201_naming_the_producer() {
    static LOG: Log = Mutex::new(Vec::new());
    let (project, runtime) = chained(&LOG, two_packages, false);

    let outcome = collected(&project, runtime, Some("items")).unwrap();

    // `--runner` still runs only what it names: `items`, on an empty input.
    assert_eq!(served(&LOG), [("items".to_string(), vec![])]);
    let w201: Vec<_> = outcome
        .diagnostics
        .iter()
        .filter(|d| d.code == "W201")
        .collect();
    assert_eq!(w201.len(), 1, "{:?}", outcome.diagnostics);
    assert_eq!(
        w201[0].message,
        "'items' reads 'package' facts, and none was served: no collector of this collect answers them and no \
         recorded store holds one; 'packages' answer them"
    );
    assert!(
        w201[0]
            .suggestion
            .as_deref()
            .is_some_and(|s| s.contains("`specforge collect --runner packages`")),
        "{:?}",
        w201[0]
    );
}
