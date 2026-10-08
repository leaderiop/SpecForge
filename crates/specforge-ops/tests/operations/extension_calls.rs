//! What the host sends the command, collector, scanner and migration-hook
//! exports, compared with the wire goldens in
//! `crates/specforge-wasm/tests/wire/`, and what a collector's failure
//! becomes. These were plan 04's T1 characterization pins; T5 and T7
//! flipped the ones that pinned a bug.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use specforge_ops::collect::{Collector, dispatch};
use specforge_ops::migrate::{MigrationInput, invoke_hooks};
use specforge_protocol_types::{CollectReportFile, ExtensionDeclaration, HandshakeResponse};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{CallFailure, WasmCallResult, WasmTrapInfo};

const EXT: &str = "@pin/ext";

fn wire_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../specforge-wasm/tests/wire")
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
    let read = dispatch(&runtime, &collector(), &reports(), None).unwrap();
    // flipped in T7: `stdout` is absent when nothing was captured, not null
    golden("collect.input.json", &runtime.calls()[0].input);
    assert_eq!(read.entity_results[0].entity_id, "a");
    let test = &read.entity_results[0].test_results[0];
    assert_eq!(
        (test.name.as_str(), test.verify.as_deref(), test.duration_ms),
        ("t", Some("v"), Some(2.0))
    );
    assert_eq!(read.unlinked[0].path, ["m", "t"]);
    dispatch(&runtime, &collector(), &reports(), Some("out")).unwrap();
    assert_eq!(runtime.calls()[1].input["stdout"], "out");

    // flipped in T7: an answer that is not a CollectOutput (an empty array
    // or object, a test without a name, not JSON) is an error naming the
    // collector, never empty results; a trap is too.
    let missing_name = json!({"entity_results": [{"entity_id": "a", "test_results": [
        {"status": "passed"}
    ]}]});
    for raw in [
        b"[]".to_vec(),
        b"{}".to_vec(),
        missing_name.to_string().into_bytes(),
        b"not json".to_vec(),
    ] {
        let runtime = answering("collect__x", WasmCallResult::Ok(raw.clone()));
        let err = dispatch(&runtime, &collector(), &reports(), None).unwrap_err();
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
        dispatch(&runtime, &collector(), &reports(), None)
            .unwrap_err()
            .to_string(),
        format!("collector collect__x() of '{EXT}' trapped: k: m")
    );
}

// ── C8 · migration hook ──

fn hooked() -> Vec<ExtensionDeclaration> {
    vec![ExtensionDeclaration {
        handshake: HandshakeResponse {
            name: EXT.into(),
            version: "1.0.0".into(),
            migration_hook: Some("migrate__x".into()),
            ..HandshakeResponse::default()
        },
        ..ExtensionDeclaration::default()
    }]
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
    use crate::view_support::{Project, registries};
    use serde_json::Map;
    use specforge_ops::command::{CommandFormat, ExtensionCommand, run};
    use specforge_protocol_types::{
        CommandDescriptor, CommandEvidence, CommandOutput, EntityEvidence,
    };
    use specforge_test_macros::test as specforge_test;
    use specforge_wasm::WasmCallResult;
    use specforge_wasm::testing::InProcessRuntime;

    /// The evidence `project`'s command run sends the export: the `evidence`
    /// of the input it received (`None`: the input carries none).
    fn sent(project: &Project) -> CommandEvidence {
        let runtime = InProcessRuntime::new().answer_raw(
            "@pin/ext",
            "cmd__probe",
            WasmCallResult::Ok(
                CommandOutput {
                    exit_code: 0,
                    stdout: "{}".into(),
                    stderr: String::new(),
                }
                .to_bytes(),
            ),
        );
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
            },
        );
        run(
            &project.view(),
            &runtime,
            &command,
            &Map::new(),
            CommandFormat::Json,
        )
        .expect("the export answers");
        let input = runtime.calls()[0].input.clone();
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
        let project = Project::new(SOURCE, registries(&["behavior"], &[]));
        let pass = |verify: &str| serde_json::json!({"status": "pass", "verify": verify});
        std::fs::write(
            project.dir.path().join("specforge-report.json"),
            serde_json::json!({"runner": "fixture", "results": {
                "proven": {"tests": [pass("a"), pass("b")]},
                "half": {"tests": [pass("a")]},
            }})
            .to_string(),
        )
        .unwrap();
        let CommandEvidence::Recorded { entities } = sent(&project) else {
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
        let project = Project::new(SOURCE, registries(&["behavior"], &[]));
        assert_eq!(sent(&project), CommandEvidence::None);
        std::fs::write(
            project.dir.path().join("specforge-report.json"),
            "{not json",
        )
        .unwrap();
        let CommandEvidence::Unreadable { reason } = sent(&project) else {
            panic!("a broken report is unreadable evidence");
        };
        assert!(reason.contains("invalid test results"), "{reason}");
        let wire = serde_json::to_value(CommandEvidence::Unreadable { reason }).unwrap();
        assert_eq!(wire["state"], "unreadable");
    }
}
