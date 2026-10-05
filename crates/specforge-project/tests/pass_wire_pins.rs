//! Characterization of today's compiler-pass and custom-validator wire
//! (plan 04, T1): the input every `__pass_<name>` export and every custom
//! rule's `wasm_function` receives, how the host reads their answers, and
//! what a failure becomes. Pins, not proofs: a pin that encodes a bug says
//! which ticket flips it. Goldens are shared with the other wire pins in
//! `crates/specforge-wasm/tests/wire/`.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use specforge_common::{Diagnostic, Severity};
use specforge_extension_sdk::prelude::*;
use specforge_project::CompiledProject;
use specforge_project::coverage::TestReport;
use specforge_project::passes::{AnalysisContext, pass_input, run_extension_passes};
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{WasmCallResult, WasmTrapInfo};
use tempfile::TempDir;

const EXT: &str = "@pin/passes";

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

fn golden_value(name: &str) -> Value {
    let path = wire_dir().join(name);
    serde_json::from_str(
        &fs::read_to_string(&path).unwrap_or_else(|e| panic!("golden {}: {e}", path.display())),
    )
    .unwrap()
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

/// `gadget`s (testable, verify `unit`, a `needs` reference, a `status`
/// lifecycle field, an `abstract` flag exempting them), a W004-style rule
/// obligating them, a custom rule `validate__shape`, a check pass `audit`
/// and an analyze pass `report`.
fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.kind("gadget", |k| {
        k.keyword("gadget")
            .testable(true)
            .supports_verify(true)
            .verify_kinds(&["unit"])
            .lifecycle_field("status");
        k.field("needs", |f| {
            f.field_type(FieldType::Reference)
                .edge("needs")
                .target_kind("gadget");
        });
        k.field("status", |f| {
            f.field_type(FieldType::String);
        });
        k.field("abstract", |f| {
            f.field_type(FieldType::Bool).exempts_obligations();
        });
    });
    c.rule("W990", |r| {
        r.check(CheckKind::NoVerifyStatements)
            .target_kind("gadget")
            .message_template("gadget '{id}' declares no obligations");
    });
    c.rule("E991", |r| {
        r.check(CheckKind::Custom)
            .target_kind("gadget")
            .wasm_function("validate__shape")
            .severity(ValidationSeverity::Error)
            .message_template("gadget '{id}' has a bad {field}");
    });
    c.pass("audit", |p| {
        p.phase("check");
    });
    c.pass("report", |_| {});
    c
}

/// The exports nothing declares a handler for: both passes report nothing,
/// the validator passes everything.
fn handler(export: &str, _input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    match export {
        "__pass_audit" | "__pass_report" => Some(Ok(b"[]".to_vec())),
        "validate__shape" => Some(Ok(br#"{"verdict":"pass"}"#.to_vec())),
        _ => None,
    }
}

fn runtime() -> InProcessRuntime {
    InProcessRuntime::new().with_handler(extension, handler)
}

const SPEC: &str = "gadget a \"A\" {\n  needs b\n  status \"active\"\n  verify unit \"a works\"\n}\n\n\
gadget b \"B\" {\n  status \"retired\"\n}\n\ngadget c \"C\" {\n  abstract true\n}\n";

fn project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let config = json!({ "name": "p", "version": "0.1.0", "extensions": [EXT] });
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    fs::write(dir.path().join("a.spec"), SPEC).unwrap();
    dir
}

/// The input the last call to `export` received.
fn last_input(runtime: &InProcessRuntime, export: &str) -> Value {
    runtime
        .calls()
        .into_iter()
        .rev()
        .find(|c| c.export == export)
        .unwrap_or_else(|| panic!("{export} was never called"))
        .input
}

// ── C4 · compiler pass ──

#[test]
fn c4_the_pass_input_of_an_analysis() {
    let dir = project();
    let runtime = runtime();
    let compiled = CompiledProject::compile(dir.path(), Some(&runtime));
    let report: TestReport = serde_json::from_value(json!({
        "runner": "cargo-test",
        "results": {"a": {"file": "a.rs", "tests": [
            {"name": "a_works", "status": "pass", "verify": "a works", "duration_ms": 1.5,
             "runner": "cargo-test"}
        ]}}
    }))
    .unwrap();
    let proved: HashSet<String> = ["b".to_string(), "a".to_string()].into();
    let registries = &compiled.env.registries;
    let input = pass_input(&AnalysisContext {
        graph: &compiled.graph,
        kind_registry: &registries.kinds,
        field_registry: &registries.fields,
        rules: &registries.rules,
        project_root: Some(dir.path()),
        test_results: Some(&report),
        proved_claims: Some(&proved),
    });
    // pinned: the host forwards the report's own keys, which the protocol's
    // PassTestResults does not define (flips in T6)
    let mut expected = golden_value("pass.input.json");
    let a = &mut expected["test_results"]["results"]["a"];
    a["file"] = json!("a.rs");
    a["tests"][0]["duration_ms"] = json!(1.5);
    a["tests"][0]["runner"] = json!("cargo-test");
    assert_eq!(input, expected);
}

#[test]
fn c4_the_pass_input_of_a_compile_carries_nulls_and_previous() {
    let dir = project();
    fs::write(
        dir.path().join("specforge-cache.json"),
        json!({"format": 1, "statuses": {"a": {"kind": "gadget", "status": "draft"}}}).to_string(),
    )
    .unwrap();
    let runtime = runtime();
    CompiledProject::compile(dir.path(), Some(&runtime));
    // pinned: `test_results` and `proved_claims` are null, flips in T6 (absent)
    let mut expected = golden_value("pass.check.input.json");
    expected["test_results"] = Value::Null;
    expected["proved_claims"] = Value::Null;
    assert_eq!(last_input(&runtime, "__pass_audit"), expected);
}

fn span(file: &str, line: usize) -> Value {
    json!({"file": file, "start_line": line, "start_col": 1, "end_line": line, "end_col": 2})
}

/// A check pass answering `answer` (raw bytes or a trap).
fn compile_answering(answer: WasmCallResult) -> Vec<Diagnostic> {
    let dir = project();
    let runtime = runtime().answer_raw(EXT, "__pass_audit", answer);
    CompiledProject::compile(dir.path(), Some(&runtime)).diagnostics()
}

fn pass_findings(diagnostics: &[Diagnostic]) -> Vec<&Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.code.starts_with('X') || d.code == "E028")
        .collect()
}

#[test]
fn c4_a_pass_answer_is_bare_or_with_a_summary_and_comes_back_sorted() {
    let diagnostics = json!([
        {"code": "X2", "severity": "Warning", "message": "b", "span": span("z.spec", 3)},
        {"code": "X1", "severity": "Info", "message": "names a", "entity": "a"},
        {"code": "X2", "severity": "Warning", "message": "a", "span": span("a.spec", 9)},
        {"code": "X2", "severity": "Error", "message": "c", "span": span("a.spec", 9),
         "data": {"kind": "shadowed_keyword", "keyword": "b"}}
    ]);
    for answer in [
        diagnostics.clone(),
        json!({"diagnostics": diagnostics, "summary": {"n": 4}}),
    ] {
        let all = compile_answering(WasmCallResult::Ok(answer.to_string().into_bytes()));
        let found = pass_findings(&all);
        let shown: Vec<(&str, &str, Option<String>)> = found
            .iter()
            .map(|d| {
                (
                    d.code.as_str(),
                    d.message.as_str(),
                    d.span
                        .as_ref()
                        .map(|s| format!("{}:{}", s.file.as_str(), s.start_line)),
                )
            })
            .collect();
        assert_eq!(
            shown,
            [
                ("X1", "names a", Some("a.spec:1".to_string())),
                ("X2", "a", Some("a.spec:9".to_string())),
                ("X2", "c", Some("a.spec:9".to_string())),
                ("X2", "b", Some("z.spec:3".to_string())),
            ],
            "{all:?}"
        );
        // pinned: a guest's `data` passes through, flips in T6 (D9: not carried)
        assert!(found[2].data.is_some(), "{:?}", found[2]);
    }
}

#[test]
fn c4_a_failing_check_pass_is_e028() {
    let trapped = compile_answering(WasmCallResult::Trap(WasmTrapInfo {
        kind: "k".into(),
        message: "m".into(),
        export_name: "__pass_audit".into(),
    }));
    let e028: Vec<&Diagnostic> = trapped.iter().filter(|d| d.code == "E028").collect();
    assert_eq!(e028.len(), 1, "{trapped:?}");
    assert_eq!(e028[0].severity, Severity::Error);
    assert_eq!(
        e028[0].message,
        format!("extension pass '{EXT}:audit' did not execute: k: m")
    );
    assert_eq!(
        e028[0].suggestion.as_deref(),
        Some(
            format!("report the failure to the author of '{EXT}', or check it is installed and up to date")
                .as_str()
        )
    );
    let malformed = compile_answering(WasmCallResult::Ok(b"{\"nope\":1}".to_vec()));
    let e028: Vec<&Diagnostic> = malformed.iter().filter(|d| d.code == "E028").collect();
    assert_eq!(e028.len(), 1, "{malformed:?}");
    assert!(
        e028[0].message.starts_with(&format!(
            "extension pass '{EXT}:audit' returned malformed diagnostics: "
        )),
        "{}",
        e028[0].message
    );
}

#[test]
fn c4_a_failing_analyze_pass_has_no_report() {
    // pinned: flips in T6 (an E028 finding of that pass)
    let dir = project();
    let runtime = runtime().answer_raw(
        EXT,
        "__pass_report",
        WasmCallResult::Trap(WasmTrapInfo {
            kind: "k".into(),
            message: "m".into(),
            export_name: "__pass_report".into(),
        }),
    );
    let compiled = CompiledProject::compile(dir.path(), Some(&runtime));
    let registries = &compiled.env.registries;
    let reports = run_extension_passes(
        &registries.passes,
        &AnalysisContext {
            graph: &compiled.graph,
            kind_registry: &registries.kinds,
            field_registry: &registries.fields,
            rules: &registries.rules,
            project_root: Some(dir.path()),
            test_results: None,
            proved_claims: None,
        },
        &runtime,
        "all",
    );
    assert!(reports.is_empty(), "{:?}", reports.len());
}

// ── C6 · custom validator ──

#[test]
fn c6_the_validator_context() {
    let dir = project();
    let runtime = runtime();
    CompiledProject::compile(dir.path(), Some(&runtime));
    let contexts: Vec<Value> = runtime
        .calls()
        .into_iter()
        .filter(|c| c.export == "validate__shape")
        .map(|c| c.input)
        .collect();
    // The probe on load, then one call per gadget.
    golden("validate.input.json", &Value::Array(contexts));
}

#[test]
fn c6_a_verdict_is_read_and_a_failure_is_w112_on_load() {
    let dir = project();
    let runtime = runtime().answer_raw(
        EXT,
        "validate__shape",
        WasmCallResult::Ok(br#"{"verdict":"fail","field":"needs","value":"b"}"#.to_vec()),
    );
    let failed = CompiledProject::compile(dir.path(), Some(&runtime)).diagnostics();
    let e991: Vec<&Diagnostic> = failed.iter().filter(|d| d.code == "E991").collect();
    assert_eq!(e991.len(), 3, "{failed:?}");
    assert_eq!(e991[0].message, "gadget 'a' has a bad needs");

    let runtime = runtime_with_trap();
    let trapped = CompiledProject::compile(dir.path(), Some(&runtime)).diagnostics();
    let w112: Vec<&Diagnostic> = trapped.iter().filter(|d| d.code == "W112").collect();
    assert_eq!(w112.len(), 1, "{trapped:?}");
    assert_eq!(
        w112[0].message,
        format!(
            "extension '{EXT}': rule 'E991': wasm_function 'validate__shape' could not be resolved \
             (custom validator 'validate__shape' did not execute: k — m) — the rule will not fire"
        )
    );
    assert!(trapped.iter().all(|d| d.code != "E991"), "{trapped:?}");
}

fn runtime_with_trap() -> InProcessRuntime {
    runtime().answer_raw(
        EXT,
        "validate__shape",
        WasmCallResult::Trap(WasmTrapInfo {
            kind: "k".into(),
            message: "m".into(),
            export_name: "validate__shape".into(),
        }),
    )
}
