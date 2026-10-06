//! What the host sends the command, collector, scanner and migration-hook
//! exports, compared with the wire goldens in
//! `crates/specforge-wasm/tests/wire/`, and what a collector's failure
//! becomes. These were plan 04's T1 characterization pins; T5 and T7
//! flipped the ones that pinned a bug.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Graph, Node};
use specforge_ops::collect::{Collector, dispatch};
use specforge_ops::command::{CommandContext, CommandFormat, command_input};
use specforge_ops::migrate::{MigrationInput, invoke_hooks};
use specforge_ops::scan::scan_source_files;
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_protocol_types::{
    AnalyzerDescriptor, CollectReportFile, ExtensionDeclaration, HandshakeResponse,
};
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

// ── C1 · command input ──

fn graph() -> Graph {
    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId {
            raw: Sym::new("f1"),
        },
        kind: EntityKind {
            raw: Sym::new("feature"),
        },
        title: Some("One".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new("main.spec"),
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 2,
        },
        methods: Vec::new(),
    });
    graph
}

#[specforge_test(
    behavior = "call_extension_exports",
    verify = "every extension call encodes its input as the protocol type the SDK decodes"
)]
fn the_command_input() {
    let args: Map<String, Value> = json!({"status": "done", "limit": 2, "all": true})
        .as_object()
        .unwrap()
        .clone();
    let input = command_input(
        &graph(),
        &args,
        Path::new("/p"),
        &CommandContext {
            format: CommandFormat::Json,
            today: "2026-10-03".into(),
        },
    );
    golden("command.input.json", &serde_json::to_value(&input).unwrap());
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

// ── C7 · scanner ──

fn scanner() -> Vec<ExtensionDeclaration> {
    vec![ExtensionDeclaration {
        handshake: HandshakeResponse {
            name: EXT.into(),
            version: "1.0.0".into(),
            ..HandshakeResponse::default()
        },
        analyzers: vec![AnalyzerDescriptor {
            language: "rust".into(),
            file_extensions: vec![".rs".into()],
            excluded_dirs: Vec::new(),
            scan_export: "scan__rust".into(),
            classify_export: "classify__rust".into(),
            map_export: "map__rust".into(),
            description: None,
        }],
        ..ExtensionDeclaration::default()
    }]
}

fn sources() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("a.rs"), "pub fn a() {}\n").unwrap();
    dir
}

#[specforge_test(
    behavior = "call_extension_exports",
    verify = "every extension call encodes its input as the protocol type the SDK decodes"
)]
fn the_scan_request_and_its_answer() {
    let dir = sources();
    let answer = json!({"items": [{"name": "a", "item_kind": "function", "line": 1}],
                        "language": "rust"});
    let runtime = answering(
        "scan__rust",
        WasmCallResult::Ok(answer.to_string().into_bytes()),
    );
    let scanned = scan_source_files(&runtime, &scanner(), dir.path(), &["a.rs".into()]);
    golden("scan.input.json", &runtime.calls()[0].input);
    assert_eq!(scanned.items.len(), 1);
    assert_eq!(
        (scanned.items[0].name.as_str(), scanned.items[0].line),
        ("a", 1)
    );
    assert_eq!(scanned.scanners_used, [EXT]);
    assert!(scanned.failures.is_empty());
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
