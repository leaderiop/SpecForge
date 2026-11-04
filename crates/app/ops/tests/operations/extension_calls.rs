//! What the host sends the command, collector, scanner and migration-hook
//! exports, compared with the wire goldens in
//! `crates/runtime/wasm/tests/wire/`, and what a collector's failure
//! becomes. These were plan 04's T1 characterization pins; T5 and T7
//! flipped the ones that pinned a bug.

use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};
use specforge_ops::analysis::collect::{Collector, dispatch};
use specforge_ops::authoring::migrate::{MigrationInput, invoke_hooks};
use specforge_protocol_types::{CollectReportFile, ExtensionDeclaration, HandshakeResponse};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{CallFailure, WasmCallResult, WasmTrapInfo};

const EXT: &str = "@pin/ext";

fn wire_dir() -> PathBuf {
    specforge_repo_root::repo_root().join("crates/runtime/wasm/tests/wire")
}

fn sorted(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            Value::Object(
                keys.into_iter()
                    .map(|k| (k.clone(), sorted(&map[k])))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
        other => other.clone(),
    }
}

fn golden(name: &str, actual: &Value) {
    let path = wire_dir().join(name);
    if std::env::var_os("SPECFORGE_BLESS").is_some() {
        let mut text = serde_json::to_string_pretty(&sorted(actual)).unwrap();
        text.push('\n');
        fs::write(&path, text).unwrap();
    }
    let expected: Value = serde_json::from_str(
        &fs::read_to_string(&path).unwrap_or_else(|e| panic!("golden {}: {e}", path.display())),
    )
    .unwrap();
    assert_eq!(actual, &expected, "golden {name}");
}

fn trap(export: &str) -> WasmCallResult {
    WasmCallResult::Trap(WasmTrapInfo {
        kind: "k".to_string(),
        message: "m".to_string(),
        export_name: export.to_string(),
    })
}

fn answering(export: &str, result: WasmCallResult) -> InProcessRuntime {
    InProcessRuntime::new().answer_raw(EXT, export, result)
}

// ── C5 · collector ──

fn collector() -> Collector {
    Collector {
        extension: EXT.to_string(),
        name: "x".to_string(),
        export: "collect__x".to_string(),
        detect: Vec::new(),
        run: Vec::new(),
        report: "report.json".to_string(),
        capture: None,
        writes: false,
        inputs: Vec::new(),
        capability: None,
    }
}

fn reports() -> Vec<CollectReportFile> {
    vec![CollectReportFile {
        path: "target/report.json".to_string(),
        content: "{}".to_string(),
    }]
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "the collector receives a CollectInput and answers a CollectOutput, and an answer that is not one is an error naming the collector"
)]
fn a_collector_receives_a_collect_input_and_answers_a_collect_output() {
    let answer = json!({"entity_results": [{"entity_id": "a", "test_results": [
        {"name": "t", "status": "passed", "verify": "v", "duration_ms": 2.0}
    ]}], "unlinked": [{"name": "m::t", "path": ["m", "t"], "status": "failed"}]});
    let runtime = answering(
        "collect__x",
        WasmCallResult::Ok(answer.to_string().into_bytes()),
    );
    let read = dispatch(Some(&runtime), &collector(), &reports(), None).unwrap();
    // flipped in T7: `stdout` is absent when nothing was captured, not null
    golden("collect.input.json", &runtime.calls()[0].input);
    assert_eq!(read.entity_results[0].entity_id, "a");
    let test = &read.entity_results[0].test_results[0];
    assert_eq!(
        (test.name.as_str(), test.verify.as_deref(), test.duration_ms),
        ("t", Some("v"), Some(2.0))
    );
    assert_eq!(read.unlinked[0].path, ["m", "t"]);
    dispatch(Some(&runtime), &collector(), &reports(), Some("out")).unwrap();
    assert_eq!(runtime.calls()[1].input["stdout"], "out");

    // flipped in T7: an answer that is not a CollectOutput (an empty array
    // or object, a test without a name, not JSON) is an error naming the
    // collector, never empty results; a trap is too.
    let missing_name = json!({"entity_results": [{"entity_id": "a", "test_results": [
        {"status": "passed"}
    ]}]});
    let unknown_status = json!({"entity_results": [{"entity_id": "a", "test_results": [
        {"name": "t", "status": "ok"}
    ]}]});
    for raw in [
        missing_name.to_string().into_bytes(),
        unknown_status.to_string().into_bytes(),
        b"not json".to_vec(),
    ] {
        let runtime = answering("collect__x", WasmCallResult::Ok(raw.clone()));
        let err = dispatch(Some(&runtime), &collector(), &reports(), None).unwrap_err();
        let shown = String::from_utf8_lossy(&raw);
        assert!(
            matches!(
                err.failure,
                CallFailure::Malformed {
                    expected: "CollectOutput",
                    ..
                }
            ),
            "{shown}: {err}"
        );
        assert!(
            err.to_string().starts_with(&format!(
                "collector collect__x() of '{EXT}' answered output that is not a CollectOutput: "
            )),
            "{err}"
        );
    }
    let runtime = answering("collect__x", trap("collect__x"));
    assert_eq!(
        dispatch(Some(&runtime), &collector(), &reports(), None)
            .unwrap_err()
            .to_string(),
        format!("collector collect__x() of '{EXT}' trapped: k: m")
    );
}

#[specforge_test(
    invariant = "collector_output_conformance",
    verify = "a test status outside passed, failed and skipped is an error naming the collector, never a skipped test"
)]
fn a_status_outside_the_vocabulary_is_not_a_collect_output() {
    let answers = [
        json!({"entity_results": [{"entity_id": "a", "test_results": [
            {"name": "t", "status": "ok"}]}]}),
        json!({"entity_results": [{"entity_id": "a", "test_results": [
            {"name": "t", "status": "Passed"}]}]}),
        json!({"entity_results": [], "unlinked": [
            {"name": "m::t", "path": ["m", "t"], "status": "ignored"}]}),
    ];
    for answer in answers {
        let runtime = answering(
            "collect__x",
            WasmCallResult::Ok(answer.to_string().into_bytes()),
        );
        let err = dispatch(Some(&runtime), &collector(), &reports(), None).unwrap_err();
        assert!(
            matches!(
                err.failure,
                CallFailure::Malformed {
                    expected: "CollectOutput",
                    ..
                }
            ),
            "{answer}: {err}"
        );
        let shown = err.to_string();
        assert!(
            shown.starts_with(&format!(
                "collector collect__x() of '{EXT}' answered output that is not a CollectOutput: "
            )),
            "{shown}"
        );
        assert!(shown.contains("unknown variant"), "{shown}");
    }
}

// ── C8 · migration hook ──

fn hooked() -> specforge_registry::RegistryBuild {
    specforge_registry::build_registries(vec![ExtensionDeclaration {
        handshake: HandshakeResponse {
            name: EXT.into(),
            version: "1.0.0".into(),
            migration_hook: Some("migrate__x".into()),
            ..HandshakeResponse::default()
        },
        ..ExtensionDeclaration::default()
    }])
}

fn hook_input() -> MigrationInput {
    MigrationInput {
        from: "0.9".into(),
        to: "1.0".into(),
        files: vec!["old.spec".into()],
    }
}

#[specforge_test(
    behavior = "call_extension_exports",
    verify = "every extension call encodes its input as the protocol type the SDK decodes"
)]
fn the_migration_hook_input_and_any_answer() {
    for answer in [&b"garbage"[..], b"", b"{}"] {
        let runtime = answering("migrate__x", WasmCallResult::Ok(answer.to_vec()));
        let (invoked, failures) = invoke_hooks(&hooked(), &runtime, &hook_input());
        assert_eq!(invoked, [format!("{EXT}:migrate__x")]);
        assert!(failures.is_empty());
        golden("migrate.input.json", &runtime.calls()[0].input);
    }
    let runtime = answering("migrate__x", trap("migrate__x"));
    let (invoked, failures) = invoke_hooks(&hooked(), &runtime, &hook_input());
    assert!(invoked.is_empty());
    assert_eq!(
        failures,
        [format!(
            "migration hook migrate__x() of '{EXT}' trapped: k: m"
        )]
    );
}

mod evidence {
    use crate::view_support::{Assembled, Viewed, registries};
    use serde_json::Map;
    use specforge_ops::extension::command::{CommandFormat, ExtensionCommand, run};
    use specforge_protocol_types::{
        CommandDescriptor, CommandEvidence, CommandOutput, EntityEvidence,
    };
    use specforge_test_macros::test as specforge_test;
    use specforge_wasm::WasmCallResult;
    use specforge_wasm::testing::InProcessRuntime;

    /// A runtime answering the probe command.
    fn probe_runtime() -> std::sync::Arc<InProcessRuntime> {
        std::sync::Arc::new(
            InProcessRuntime::new().answer_raw(
                "@pin/ext",
                "cmd__probe",
                WasmCallResult::Ok(
                    CommandOutput {
                        exit_code: 0,
                        stdout: "{}".into(),
                        stderr: String::new(),
                        plan: None,
                        state: None,
                    }
                    .to_bytes(),
                ),
            ),
        )
    }

    /// The evidence `project`'s command run sends the export: the `evidence`
    /// of the input it received (`None`: the input carries none).
    fn sent(project: &Assembled, runtime: &InProcessRuntime) -> CommandEvidence {
        let command = ExtensionCommand::new(
            "@pin/ext",
            "pin",
            &CommandDescriptor {
                id: "probe".into(),
                title: "Probe".into(),
                description: String::new(),
                category: None,
                export: "cmd__probe".into(),
                args: Vec::new(),
                writes: false,
                ..Default::default()
            },
        );
        run(
            &project.view(),
            &command,
            &Map::new(),
            CommandFormat::Json,
            None,
            specforge_ops::testing::caller(),
        )
        .expect("the export answers");
        let input = runtime
            .calls()
            .last()
            .expect("the export was called")
            .input
            .clone();
        match input.get("evidence") {
            // Absent, not null (ADR 0013 D7).
            None => CommandEvidence::None,
            Some(evidence) => serde_json::from_value(evidence.clone()).unwrap(),
        }
    }

    const SOURCE: &str = "behavior proven \"Proven\" {\n  verify unit \"a\"\n  verify unit \"b\"\n}\n\
                          behavior half \"Half\" {\n  verify unit \"a\"\n  verify unit \"b\"\n}\n";

    #[specforge_test(
        behavior = "dispatch_surface_command",
        verify = "the CommandInput carries what the recorded tests prove, per entity that counts toward coverage"
    )]
    fn the_input_carries_each_counted_entitys_proof() {
        let runtime = probe_runtime();
        let project =
            Assembled::over_in(registries(&["behavior"], &[]), runtime.clone()).source(SOURCE);
        let pass = |verify: &str| serde_json::json!({"name": verify, "status": "passed", "verify": verify});
        std::fs::write(
            project.root().join("specforge-report.json"),
            serde_json::json!({"runner": "fixture", "results": {
                "proven": {"tests": [pass("a"), pass("b")]},
                "half": {"tests": [pass("a")]},
            }})
            .to_string(),
        )
        .unwrap();
        let CommandEvidence::Recorded { entities } = sent(&project, &runtime) else {
            panic!("a recorded report is evidence");
        };
        assert_eq!(
            entities.get("proven"),
            Some(&EntityEvidence {
                obligations: 2,
                proven: 2,
                failing: 0
            })
        );
        assert!(entities["proven"].is_proven());
        assert_eq!(
            entities.get("half"),
            Some(&EntityEvidence {
                obligations: 2,
                proven: 1,
                failing: 0
            })
        );
        assert!(!entities["half"].is_proven());
        assert_eq!(entities.len(), 2, "only what counts toward coverage");
    }

    #[specforge_test(
        behavior = "dispatch_surface_command",
        verify = "a command's input says when the recorded test report cannot be read, and carries no evidence without one"
    )]
    fn no_report_is_no_evidence_and_a_broken_one_says_why() {
        let runtime = probe_runtime();
        let project =
            Assembled::over_in(registries(&["behavior"], &[]), runtime.clone()).source(SOURCE);
        assert_eq!(sent(&project, &runtime), CommandEvidence::None);
        std::fs::write(project.root().join("specforge-report.json"), "{not json").unwrap();
        let CommandEvidence::Unreadable { reason } = sent(&project, &runtime) else {
            panic!("a broken report is unreadable evidence");
        };
        assert!(reason.contains("invalid test results"), "{reason}");
        let wire = serde_json::to_value(CommandEvidence::Unreadable { reason }).unwrap();
        assert_eq!(wire["state"], "unreadable");
    }
}

/// An operation over a view calls extensions in the view's runtime.
mod the_views_runtime {
    use std::sync::Arc;

    use serde_json::Map;
    use specforge_extension_sdk::prelude::*;
    use specforge_ops::analysis::analyze::{AnalyzeOptions, analyze};
    use specforge_ops::extension::command::{CommandFormat, ExtensionCommands, RunError, run};
    use specforge_ops::view::ProjectView;
    use specforge_project::testing::TestProject;
    use specforge_test_macros::test as specforge_test;
    use specforge_wasm::CallFailure;
    use specforge_wasm::testing::InProcessRuntime;
    use tempfile::TempDir;

    /// `@t/x`: an analyze pass `scan` and a command `hello`.
    fn extension() -> InProcessRuntime {
        InProcessRuntime::new().with(|| {
            let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/x", "1.0.0"));
            c.pass("scan", |p| {
                p.run(|_: &PassInput| Vec::<PassDiagnostic>::new());
            });
            c.command("hello", |cmd| {
                cmd.title("Hello").handler(|_| CommandOutput {
                    exit_code: 0,
                    stdout: "{}\n".into(),
                    stderr: String::new(),
                    plan: None,
                    state: None,
                });
            });
            c
        })
    }

    /// A project enabling `@t/x`, with one `main.spec`.
    fn project() -> TempDir {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("specforge.json"),
            r#"{"name":"p","version":"0.1.0","extensions":["@t/x"]}"#,
        )
        .unwrap();
        specforge_installed::testing::install(dir.path(), &extension(), &["@t/x"]);
        std::fs::write(dir.path().join("main.spec"), "behavior a \"A\" {\n}\n").unwrap();
        dir
    }

    #[specforge_test(
        invariant = "extensions_run_in_their_loading_runtime",
        verify = "an operation over a project view calls its extensions in the runtime the view's environment loaded them in"
    )]
    fn operations_over_a_view_call_extensions_in_its_runtime() {
        let dir = project();
        let rt = Arc::new(extension());
        let shared: SharedRuntime = rt.clone();
        let compiled = compile_at(dir.path(), Some(shared));
        let view = ProjectView::of(&compiled);

        let outcome = analyze(&view, &AnalyzeOptions::default()).unwrap();
        assert!(
            outcome.passes.iter().any(|p| p.name == "@t/x:scan"),
            "{:?}",
            outcome.passes.iter().map(|p| &p.name).collect::<Vec<_>>()
        );
        let commands = ExtensionCommands::build(view.registries());
        let hello = commands
            .all()
            .iter()
            .find(|c| c.id() == "hello")
            .expect("the extension declares `hello`");
        run(
            &view,
            hello,
            &Map::new(),
            CommandFormat::Json,
            None,
            specforge_ops::testing::caller(),
        )
        .expect("the export answers");

        let exports: Vec<String> = rt.calls().into_iter().map(|c| c.export).collect();
        assert!(exports.contains(&"__pass_scan".to_string()), "{exports:?}");
        assert!(exports.contains(&"cmd__hello".to_string()), "{exports:?}");
    }

    #[test]
    fn without_a_runtime_an_extension_call_is_not_loaded() {
        let dir = project();
        // Compiled with a runtime only to learn the command the project's
        // extension declares; the environment under test has none.
        let declared = compile_at(dir.path(), Some(Arc::new(extension())));
        let commands = ExtensionCommands::build(ProjectView::of(&declared).registries());
        let hello = commands.all().iter().find(|c| c.id() == "hello").unwrap();

        let bare = compile_at(dir.path(), None);
        let view = ProjectView::of(&bare);
        let RunError::Call(error) = run(
            &view,
            hello,
            &Map::new(),
            CommandFormat::Json,
            None,
            specforge_ops::testing::caller(),
        )
        .unwrap_err() else {
            panic!("a command over an environment with no runtime is a failed call");
        };
        assert_eq!(error.failure, CallFailure::NotLoaded);
        assert_eq!(error.diagnostic().code, "E028");

        let outcome = analyze(&view, &AnalyzeOptions::default()).unwrap();
        assert!(
            !outcome.passes.iter().any(|p| p.name.starts_with("@t/x:")),
            "no extension pass runs without a runtime"
        );
    }
}

/// A command is lent the ports its caller holds (ADR 0335 D4), and what it runs never sees a secret (ADR 0288).
mod lending {
    use std::sync::Arc;

    use serde_json::Map;
    use specforge_effects::testing::{self, FakeHttp, MemoryConsent};
    use specforge_effects::{
        EffectJournal, EffectsMode, Grant, Ports, ProcessRequest, Provenance, Roots,
    };
    use specforge_extension_sdk::host;
    use specforge_extension_sdk::prelude::*;
    use specforge_ops::asking::Asking;
    use specforge_ops::extension::command::{CommandFormat, ExtensionCommands, run};
    use specforge_ops::view::ProjectView;
    use specforge_ops::{Caller, Containment};
    use specforge_project::testing::TestProject;
    use specforge_protocol_types::Operation;
    use specforge_wasm::testing::InProcessRuntime;
    use specforge_wasm::{Limits, PortCall, PortSession};
    use tempfile::TempDir;

    const EXT: &str = "@t/lent";
    const SECRET_VALUE: &str = "s3cret-value-never-seen";

    fn declaration() -> ContributionsBuilder {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
        c.secret("api-token", "ACME_API_VALUE", "the API's token");
        c.capability(
            Capability::http("api")
                .hosts(&["example.test"])
                .schemes(&["https"])
                .why("call the API"),
        );
        c.capability(
            Capability::process("env")
                .argv(&["env"])
                .why("read the env"),
        );
        c.command("ping", |cmd| {
            cmd.title("Ping").handler(|_| {
                let answered = host::http::send(&specforge_effects::HttpRequest {
                    method: "GET".to_string(),
                    url: "https://example.test/a".to_string(),
                    ..Default::default()
                });
                CommandOutput::ok(format!("{:?}\n", answered.map(|r| r.status)))
            });
        });
        c
    }

    fn runtime() -> InProcessRuntime {
        InProcessRuntime::new()
            .with(declaration)
            .importing(EXT, &["specforge:host/http@0.1.0"])
    }

    #[test]
    fn a_command_run_lends_the_views_ports() {
        let project = TestProject::new().serving(runtime()).enabling(&[EXT]);
        let compiled = project.compile();
        let view = ProjectView::of(&compiled);
        let commands = ExtensionCommands::build(view.registries());
        let ping = commands.all().iter().find(|c| c.id() == "ping").unwrap();

        let http = Arc::new(FakeHttp::new().route("https://example.test/a", 200, "answer"));
        let ports = Ports {
            http: http.clone(),
            ..testing::ports()
        };
        let consent = MemoryConsent::approving_everything();
        let containment = Containment::unbounded();
        let caller = Caller::new(&containment, Asking::Nobody).lending(
            &ports,
            &consent,
            EffectsMode::Online,
        );

        let output = run(&view, ping, &Map::new(), CommandFormat::Json, None, caller).unwrap();

        assert_eq!(output.stdout, "Ok(200)\n");
        assert_eq!(http.requests().len(), 1, "the lent port saw the request");

        // A caller that lends nothing: the same guest is refused, and nothing is reached.
        let plain = Caller::new(&containment, Asking::Nobody);
        let output = run(&view, ping, &Map::new(), CommandFormat::Json, None, plain).unwrap();
        assert!(
            output.stdout.starts_with("Err(Denied(Scope"),
            "{}",
            output.stdout
        );
        assert_eq!(http.requests().len(), 1);
    }

    /// Another extension's answer, kept where the journal keeps it: a project read does not reach it.
    #[test]
    fn a_guest_with_a_project_read_cannot_read_the_effects_journal() {
        let project = TestProject::new();
        let root = project.root().canonicalize().unwrap();
        let journal = EffectJournal::at(&root);
        let request = specforge_effects::EffectRequest::Http(specforge_effects::HttpRequest {
            method: "GET".to_string(),
            url: "https://example.test/private".to_string(),
            body_cap: 1024,
            ..Default::default()
        });
        let answer = specforge_effects::Answer {
            outcome: specforge_effects::Outcome {
                class: specforge_effects::OutcomeClass::Ok,
                digest: "sha256:0".to_string(),
            },
            payload: Some(SECRET_VALUE.as_bytes().to_vec()),
        };
        let (record, _) = journal
            .keep(&request, &answer, specforge_effects::UnixMillis(1))
            .unwrap()
            .expect("journaled");
        let payload = record
            .with_extension("payload")
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert!(root.join(&payload).is_file(), "{payload}");

        let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
        c.capability(
            Capability::fs("files")
                .ops(&[FsAccess::Stat, FsAccess::Read, FsAccess::Walk])
                .why("read the project"),
        );
        let roots = Roots::new(&root, &root);
        let grant = Grant::decide(
            &c.declaration(),
            &Provenance::third_party(b"x"),
            &specforge_effects::EffectsCeiling::default(),
            &roots,
            &MemoryConsent::approving_everything(),
        );
        let ports = Ports {
            fs: Arc::new(specforge_fs::LocalFs),
            ..testing::ports()
        };
        let call = PortCall::online(&ports, EffectJournal::detached(), roots, grant, Vec::new());
        let mut session =
            PortSession::effectful(EXT, Operation::Command, Some(call), Limits::CEILING);

        let refused = session.fs_read("project", &payload, 0);
        assert!(
            matches!(refused, Err(specforge_protocol_types::HostError::Denied(_))),
            "{refused:?}"
        );
        for (path, glob) in [
            ("", Some(".specforge/**")),
            ("", Some("**/*.payload")),
            ("", None),
        ] {
            let listed = session.fs_walk("project", path, glob, 100).unwrap();
            assert!(listed.entries.is_empty(), "{glob:?}: {listed:?}");
        }
        assert!(
            session.fs_walk("project", ".specforge", None, 100).is_err(),
            "a walk from inside it"
        );
    }

    /// The real runner, the real inherited environment: the value set here is what a guest must not read back.
    #[cfg(unix)]
    #[test]
    #[allow(
        unsafe_code,
        reason = "set_var before any thread of this test reads the environment; the test runs in its own process under nextest"
    )]
    fn a_real_program_a_guest_runs_does_not_inherit_a_declared_secret() {
        // SAFETY: nextest runs each test in its own process, and no thread has started yet.
        unsafe { std::env::set_var("ACME_API_VALUE", SECRET_VALUE) };
        let dir = TempDir::new().unwrap();
        let roots = Roots::new(dir.path(), dir.path());
        let grant = Grant::decide(
            &declaration().declaration(),
            &Provenance::third_party(b"x"),
            &specforge_effects::EffectsCeiling::default(),
            &roots,
            &MemoryConsent::approving_everything(),
        );
        let ports = Ports {
            process: Arc::new(specforge_process::Runner),
            ..testing::ports()
        };
        let call = PortCall::online(
            &ports,
            EffectJournal::detached(),
            roots,
            grant,
            vec!["ACME_API_VALUE".to_string()],
        );
        let mut session =
            PortSession::effectful(EXT, Operation::Collect, Some(call), Limits::CEILING);

        let output = session
            .process_run(ProcessRequest {
                argv: vec!["env".to_string()],
                ..ProcessRequest::default()
            })
            .expect("the capability grants `env`");

        let seen = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.exit_code, Some(0), "{seen}");
        assert!(seen.contains("PATH="), "env ran and printed: {seen}");
        assert!(
            !seen.contains(SECRET_VALUE),
            "the secret reached a guest: {seen}"
        );
        assert!(!seen.contains("ACME_API_VALUE"), "{seen}");
    }
}
