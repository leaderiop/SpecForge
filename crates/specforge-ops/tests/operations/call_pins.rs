//! Characterization of today's command-input, collector, scanner and
//! migration-hook wire (plan 04, T1): what the host sends, how it reads the
//! answer, and what a failure becomes. Pins, not proofs: a pin that encodes
//! a bug says which ticket flips it. Goldens are shared with the other wire
//! pins in `crates/specforge-wasm/tests/wire/`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Graph, Node};
use specforge_ops::collect::{Collector, ReportFile, dispatch};
use specforge_ops::command::{CommandContext, CommandFormat, command_input};
use specforge_ops::migrate::{HookInput, invoke_hooks};
use specforge_ops::scan::scan_source_files;
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_protocol_types::{AnalyzerDescriptor, ExtensionDeclaration, HandshakeResponse};
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{WasmCallResult, WasmTrapInfo};

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

#[test]
fn c1_the_command_input() {
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
    golden(
        "command.input.json",
        &serde_json::from_slice(&input).unwrap(),
    );
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

fn reports() -> Vec<ReportFile> {
    vec![ReportFile {
        path: "target/report.json".to_string(),
        content: "{}".to_string(),
    }]
}

#[test]
fn c5_the_collect_input() {
    let runtime = answering(
        "collect__x",
        WasmCallResult::Ok(br#"{"entity_results":[]}"#.to_vec()),
    );
    dispatch(&runtime, &collector(), &reports(), None).unwrap();
    // pinned: `stdout` is null, flips in T7 (absent)
    golden("collect.input.json", &runtime.calls()[0].input);
    dispatch(&runtime, &collector(), &reports(), Some("out")).unwrap();
    assert_eq!(runtime.calls()[1].input["stdout"], "out");
}

#[test]
fn c5_a_collector_answer_is_read_with_or_without_unlinked_tests() {
    let answer = json!({"entity_results": [{"entity_id": "a", "test_results": [
        {"name": "t", "status": "passed", "verify": "v", "duration_ms": 2.0}
    ]}]});
    let runtime = answering(
        "collect__x",
        WasmCallResult::Ok(answer.to_string().into_bytes()),
    );
    let read = dispatch(&runtime, &collector(), &reports(), None).unwrap();
    assert_eq!(read.entity_results[0].entity_id, "a");
    assert_eq!(
        read.entity_results[0].test_results[0].verify.as_deref(),
        Some("v")
    );
    assert!(read.unlinked.is_empty());

    let answer = json!({"entity_results": [], "unlinked": [
        {"name": "m::t", "path": ["m", "t"], "status": "failed"}
    ]});
    let runtime = answering(
        "collect__x",
        WasmCallResult::Ok(answer.to_string().into_bytes()),
    );
    let read = dispatch(&runtime, &collector(), &reports(), None).unwrap();
    assert_eq!(read.unlinked[0].path, ["m", "t"]);
}

#[test]
fn c5_an_empty_array_or_object_reads_as_no_results() {
    // pinned: flips in T7 (an answer that is not a CollectOutput is an error)
    for answer in [&b"[]"[..], b"{}"] {
        let runtime = answering("collect__x", WasmCallResult::Ok(answer.to_vec()));
        let read = dispatch(&runtime, &collector(), &reports(), None).unwrap();
        assert!(read.entity_results.is_empty() && read.unlinked.is_empty());
    }
}

#[test]
fn c5_a_collected_test_without_a_name_is_named_empty() {
    // pinned: flips in T7 (an answer that is not a CollectOutput is an error)
    let answer = json!({"entity_results": [{"entity_id": "a", "test_results": [
        {"status": "passed"}
    ]}]});
    let runtime = answering(
        "collect__x",
        WasmCallResult::Ok(answer.to_string().into_bytes()),
    );
    let read = dispatch(&runtime, &collector(), &reports(), None).unwrap();
    assert_eq!(read.entity_results[0].test_results[0].name, "");
}

#[test]
fn c5_a_failing_collector_is_an_error_naming_it() {
    let runtime = answering("collect__x", trap("collect__x"));
    assert_eq!(
        dispatch(&runtime, &collector(), &reports(), None).unwrap_err(),
        format!("{EXT}: collect__x() trapped: k: m")
    );
    let runtime = answering("collect__x", WasmCallResult::Ok(b"not json".to_vec()));
    let err = dispatch(&runtime, &collector(), &reports(), None).unwrap_err();
    assert!(
        err.starts_with(&format!("{EXT}: collect__x() returned malformed results: ")),
        "{err}"
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

#[test]
fn c7_the_scan_request_and_its_answer() {
    let dir = sources();
    let answer = json!({"items": [{"name": "a", "item_kind": "function", "line": 1}],
                        "language": "rust"});
    let runtime = answering(
        "scan__rust",
        WasmCallResult::Ok(answer.to_string().into_bytes()),
    );
    let (items, used) = scan_source_files(&runtime, &scanner(), dir.path(), &["a.rs".into()]);
    golden("scan.input.json", &runtime.calls()[0].input);
    assert_eq!(items.len(), 1);
    assert_eq!((items[0].name.as_str(), items[0].line), ("a", 1));
    assert_eq!(used, [EXT]);
}

#[test]
fn c7_a_failing_scanner_is_dropped() {
    // pinned: flips in T7 (a failure per file, reported)
    let dir = sources();
    for answer in [trap("scan__rust"), WasmCallResult::Ok(b"garbage".to_vec())] {
        let runtime = answering("scan__rust", answer);
        let (items, used) = scan_source_files(&runtime, &scanner(), dir.path(), &["a.rs".into()]);
        assert!(items.is_empty() && used.is_empty());
    }
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

fn hook_input() -> HookInput {
    HookInput {
        from: "0.9".into(),
        to: "1.0".into(),
        files: vec!["old.spec".into()],
    }
}

#[test]
fn c8_the_migration_hook_input_and_any_answer() {
    for answer in [&b"garbage"[..], b"", b"{}"] {
        let runtime = answering("migrate__x", WasmCallResult::Ok(answer.to_vec()));
        let (invoked, failures) = invoke_hooks(&hooked(), &runtime, &hook_input());
        assert_eq!(invoked, [format!("{EXT}:migrate__x")]);
        assert!(failures.is_empty());
        golden("migrate.input.json", &runtime.calls()[0].input);
    }
}

#[test]
fn c8_a_trapping_hook_is_a_failure_line() {
    let runtime = answering("migrate__x", trap("migrate__x"));
    let (invoked, failures) = invoke_hooks(&hooked(), &runtime, &hook_input());
    assert!(invoked.is_empty());
    assert_eq!(
        failures,
        [format!(
            "migration hook 'migrate__x' of {EXT} did not execute: k: m"
        )]
    );
}
