//! A collector with no command and no report reads through its ports (ADR 0345 D2): it is called under a run and a
//! no-run alike, reads no report, needs no approval, and a report run naming it is E058.

use std::sync::{Arc, Mutex};

use serde_json::Map;
use specforge_effects::testing::{self, FakeFs, MemoryConsent};
use specforge_effects::{EffectsMode, Ports};
use specforge_extension_sdk::host;
use specforge_extension_sdk::prelude::*;
use specforge_ops::analysis::collect::{Mode, Outcome, Request, RunnerOutput, collect};
use specforge_ops::asking::Asking;
use specforge_ops::extension::command::{CommandFormat, ExtensionCommands, run};
use specforge_ops::testing::ScriptedMachine;
use specforge_ops::view::ProjectView;
use specforge_ops::{Caller, Containment};
use specforge_project::testing::TestProject;
use specforge_protocol_types::{
    FsContent, HostError, SyntaxAnswer, SyntaxFiles, SyntaxOutcome, SyntaxRequest, SyntaxSource,
};
use specforge_wasm::testing::InProcessRuntime;

use super::{Fixture, Machine, Runner};

#[specforge_test_macros::test(
    behavior = "register_collector_contributions",
    verify = "a collector with no command and no report is called under run and no-run, and ran is false"
)]
fn a_collector_with_no_command_and_no_report_is_called_under_run_and_no_run() {
    let fx = Fixture::new(vec![Runner::new("source").reading_ports()]);
    // Nothing approved: a collector that runs nothing needs no approval.
    let machine = Machine::new();

    let ran = fx.run(Asking::Nobody, &machine).unwrap();
    let read = fx.no_run(None, &machine).unwrap();

    for outcome in [&ran, &read] {
        let [runner] = outcome.runners.as_slice() else {
            panic!("one runner: {:?}", outcome.runners);
        };
        assert_eq!(runner.name, "source");
        assert!(!runner.ran);
        assert_eq!((runner.files, runner.skip_reason.as_deref()), (0, None));
    }
    assert!(machine.runs().is_empty(), "{:?}", machine.runs());
    assert!(machine.approvals().is_empty());
    let calls = fx.dispatched();
    assert_eq!(calls.len(), 2, "called under both modes");
    assert!(
        calls
            .iter()
            .all(|call| call.input["reports"].as_array().is_none_or(Vec::is_empty))
    );
    assert!(!fx.exists(".specforge/reports/source.json"));
}

#[specforge_test_macros::test(
    behavior = "register_collector_contributions",
    verify = "a report run naming a collector that reads through its ports is E058"
)]
fn a_report_run_naming_a_ports_collector_is_e058() {
    let fx = Fixture::new(vec![Runner::new("acme").reading_ports()]);
    fx.write("given.json", "{}");

    let refusal = fx
        .no_run_reports(&[fx.root().join("given.json")])
        .unwrap_err();

    assert_eq!(refusal.error.code, "E058");
    assert!(
        refusal.error.message.contains("reads no report"),
        "{refusal:?}"
    );
    assert!(fx.dispatched().is_empty());
}

const EXT: &str = "@t/source";
const QUERY: &str = "(function_item name: (identifier) @name) @definition.function";

/// What the fs-reading collector was answered, one per call.
static READS: Mutex<Vec<Result<FsContent, HostError>>> = Mutex::new(Vec::new());
/// What the parsing collector was answered, one per call.
static PARSES: Mutex<Vec<Result<SyntaxAnswer, HostError>>> = Mutex::new(Vec::new());

/// `@t/source` reading the project's files: `collector` is what its one collector does.
fn reader(collector: fn()) -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.capability(
        Capability::fs("source")
            .roots(&["project"])
            .ops(&[FsAccess::Read, FsAccess::Walk])
            .call_kinds(&[Operation::Collect, Operation::Command])
            .why("read the project's source"),
    );
    c.collector("source", move |k| {
        k.collect(move |_| {
            collector();
            Ok(CollectOutput::default())
        });
    });
    c
}

/// A project serving `build` with `imports`, its files on disk and in the fs port `ports` lends.
struct Reading {
    project: TestProject,
    ports: Ports,
}

impl Reading {
    fn new(build: fn() -> ContributionsBuilder, import: &str, files: &[(&str, &str)]) -> Self {
        let runtime = InProcessRuntime::new()
            .with(build)
            .importing(EXT, &[import]);
        let project = TestProject::new()
            .serving(runtime)
            .enabling(&[EXT])
            .files(files);
        let mut fs = FakeFs::new();
        for (path, text) in files {
            fs = fs.with_file(project.root().join(path), text.as_bytes());
        }
        let ports = Ports {
            fs: Arc::new(fs),
            ..testing::ports()
        };
        Reading { project, ports }
    }

    /// One collect in `mode`, nothing approved and nobody to ask.
    fn collect(&self, mode: Mode<'_>) -> Outcome {
        let compiled = self.project.compile();
        self.collect_over(&ProjectView::of(&compiled), mode)
    }

    /// [`Self::collect`] over `view`.
    fn collect_over(&self, view: &ProjectView<'_>, mode: Mode<'_>) -> Outcome {
        let consent = MemoryConsent::default();
        let containment = Containment::unbounded();
        collect(
            view,
            Request {
                runner: Some("source"),
                mode,
                announce: &mut |_, _| {},
                caller: Caller::new(&containment, Asking::Nobody).lending(
                    &self.ports,
                    &consent,
                    EffectsMode::Online,
                ),
            },
            &ScriptedMachine::new(),
            &consent,
        )
        .unwrap_or_else(|refusal| panic!("collected: {refusal:?}"))
    }
}

#[specforge_test_macros::test(
    behavior = "register_collector_contributions",
    verify = "a collector that reads through its ports is lent its fs reads without an approval"
)]
fn a_ports_collector_is_lent_its_fs_reads_without_an_approval() {
    let reading = Reading::new(
        || {
            reader(|| {
                READS
                    .lock()
                    .unwrap()
                    .push(host::fs::read("project", "src/a.rs", 0));
            })
        },
        "specforge:host/fs@0.1.0",
        &[("src/a.rs", "fn a() {}\n")],
    );

    let outcome = reading.collect(Mode::Run(RunnerOutput::Discard));

    assert!(!outcome.runners[0].ran);
    assert!(outcome.unsaved_approvals.is_empty());
    let reads = READS.lock().unwrap();
    let [Ok(content)] = reads.as_slice() else {
        panic!("one read, answered: {reads:?}");
    };
    assert_eq!(content.bytes, b"fn a() {}\n");
}

/// What the twice-collecting collector was answered, one per call.
static TWICE: Mutex<Vec<Result<SyntaxAnswer, HostError>>> = Mutex::new(Vec::new());

fn parse_sources() {
    parse_into(&PARSES);
}

fn parse_into(answers: &Mutex<Vec<Result<SyntaxAnswer, HostError>>>) {
    let request = SyntaxRequest {
        source: SyntaxSource::Worktree {
            root: "project".into(),
        },
        files: SyntaxFiles::Paths(vec!["src/a.rs".into(), "src/broken.rs".into()]),
        language: "rust".into(),
        query: QUERY.into(),
    };
    answers.lock().unwrap().push(host::syntax::parse(&request));
}

/// The names the matches of `file` capture.
fn names(answer: &SyntaxAnswer, file: usize) -> Vec<String> {
    match &answer.files[file].outcome {
        SyntaxOutcome::Parsed { matches, .. } => matches
            .iter()
            .flat_map(|m| m.captures.iter().map(|c| c.text.clone()))
            .collect(),
        SyntaxOutcome::NotParsed { reason } => panic!("not parsed: {reason:?}"),
    }
}

#[specforge_test_macros::test(
    behavior = "register_collector_contributions",
    verify = "a collector parses source through the syntax import, and what it found broken is noted once"
)]
fn a_ports_collector_parses_through_syntax_and_its_work_is_on_the_call() {
    let reading = Reading::new(
        || reader(parse_sources),
        "specforge:host/syntax@0.1.0",
        &[
            ("src/a.rs", "fn alpha() {}\nfn beta() {}\n"),
            ("src/broken.rs", "fn gamma( {\n"),
        ],
    );

    let outcome = reading.collect(Mode::NoRun);

    let parses = PARSES.lock().unwrap();
    let [Ok(answer)] = parses.as_slice() else {
        panic!("one parse, answered: {parses:?}");
    };
    assert_eq!((answer.parsed, answer.cached), (2, 0));
    assert_eq!(names(answer, 0), ["alpha", "beta"]);
    // What the parse found broken is noted on the call, and reported once.
    let noted: Vec<&str> = outcome
        .diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    assert_eq!(noted, ["I211"], "{:?}", outcome.diagnostics);
    assert!(outcome.diagnostics[0].message.contains("`src/broken.rs`"));
}

#[specforge_test_macros::test(
    behavior = "read_source_syntax",
    verify = "a second collect answers every file from the project's syntax cache"
)]
fn a_second_collect_answers_every_file_from_the_projects_syntax_cache() {
    let reading = Reading::new(
        || reader(|| parse_into(&TWICE)),
        "specforge:host/syntax@0.1.0",
        &[
            ("src/a.rs", "fn alpha() {}\nfn beta() {}\n"),
            ("src/broken.rs", "fn gamma( {\n"),
        ],
    );

    reading.collect(Mode::NoRun);
    reading.collect(Mode::Run(RunnerOutput::Discard));

    let answers = TWICE.lock().unwrap();
    let [Ok(first), Ok(second)] = answers.as_slice() else {
        panic!("two parses, answered: {answers:?}");
    };
    assert_eq!((first.parsed, first.cached), (2, 0));
    assert_eq!((second.parsed, second.cached), (0, 2));
    assert_eq!(second.files, first.files);
    assert!(
        reading
            .project
            .root()
            .join(specforge_source::CACHE_DIR)
            .is_dir()
    );
}

/// What the collect of the command test was answered, then what its command was.
static FILLED: Mutex<Vec<Result<SyntaxAnswer, HostError>>> = Mutex::new(Vec::new());
static COMMANDED: Mutex<Vec<Result<SyntaxAnswer, HostError>>> = Mutex::new(Vec::new());

#[specforge_test_macros::test(
    behavior = "read_source_syntax",
    verify = "a command's syntax request is answered from the cache a collect filled"
)]
fn a_commands_syntax_request_is_answered_from_the_cache_a_collect_filled() {
    let reading = Reading::new(
        || {
            let mut c = reader(|| parse_into(&FILLED));
            c.command("parse", |cmd| {
                cmd.title("Parse").handler(|_| {
                    parse_into(&COMMANDED);
                    CommandOutput::ok(String::new())
                });
            });
            c
        },
        "specforge:host/syntax@0.1.0",
        &[
            ("src/a.rs", "fn alpha() {}\nfn beta() {}\n"),
            ("src/broken.rs", "fn gamma( {\n"),
        ],
    );
    let compiled = reading.project.compile();
    let view = ProjectView::of(&compiled);

    reading.collect_over(&view, Mode::NoRun);
    let commands = ExtensionCommands::build(view.registries());
    let parse = commands.all().iter().find(|c| c.id() == "parse").unwrap();
    let consent = MemoryConsent::approving_everything();
    let containment = Containment::unbounded();
    let caller = Caller::new(&containment, Asking::Nobody).lending(
        &reading.ports,
        &consent,
        EffectsMode::Online,
    );
    run(&view, parse, &Map::new(), CommandFormat::Json, None, caller).unwrap();

    let filled = FILLED.lock().unwrap();
    let commanded = COMMANDED.lock().unwrap();
    let ([Ok(filled)], [Ok(commanded)]) = (filled.as_slice(), commanded.as_slice()) else {
        panic!("one parse each, answered: {filled:?} {commanded:?}");
    };
    assert_eq!((filled.parsed, filled.cached), (2, 0));
    assert_eq!((commanded.parsed, commanded.cached), (0, 2));
    assert_eq!(commanded.files, filled.files);
}
