//! An extension's text is read printable where the host reads it (ADR 0244): no control or
//! bidirectional character of a handshake, describe, pass or validator answer, or of a call's
//! failure, reaches a message, a listing or a help. A command's, an MCP tool's, a collector's
//! and a scanner's answer is read as the extension answered.

use serde_json::{Value, json};
use specforge_common::remedy::printable;
use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::{PassInput, ValidatorContext};
use specforge_wasm::protocol::load_declaration;
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{ExtensionCalls, WasmCallResult, WasmTrapInfo, pass_diagnostics};

const EXT: &str = "@text/x";
const HOSTILE: &str = "\u{1b}]52;c;cm0gLXJmIH4=\u{7}";
const BIDI: &str = "\u{202e}\u{9b}2J";

fn hostile() -> ContributionsBuilder {
    let mut meta = ExtensionMeta::new(EXT, "1.0.0");
    meta.description = Some(format!("d{HOSTILE}"));
    meta.starter_template = Some("a\n\tb\u{1b}c".to_string());
    let mut c = ContributionsBuilder::new(meta);
    c.kind("widget", |k| {
        k.description(&format!("k{HOSTILE}"));
        k.field("style", |f| {
            f.field_type(FieldType::String);
            f.description(&format!("f{HOSTILE}"));
        });
    });
    c.rule("W900", |r| {
        r.check(CheckKind::MissingRequiredField);
        r.target_kind("widget");
        r.field("style");
        r.severity(ValidationSeverity::Warning);
        r.message_template(&format!("m{HOSTILE}{BIDI} '{{id}}'"));
    });
    c.command("hello", |cmd| {
        cmd.title(&format!("t{HOSTILE}"))
            .description(&format!("c{HOSTILE}"))
            .arg("whom", |a| {
                a.description(&format!("a{HOSTILE}"));
            })
            .handler(|_| CommandOutput {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            });
    });
    c
}

fn ok(value: &Value) -> WasmCallResult {
    WasmCallResult::Ok(serde_json::to_vec(value).unwrap())
}

/// Every string and key of `value` holds no control or bidirectional character.
fn assert_printable(value: &Value) {
    match value {
        Value::String(s) => assert_eq!(printable(s), s.as_str(), "{s:?}"),
        Value::Array(items) => items.iter().for_each(assert_printable),
        Value::Object(map) => map.iter().for_each(|(k, v)| {
            assert_eq!(printable(k), k.as_str(), "{k:?}");
            assert_printable(v);
        }),
        _ => {}
    }
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "every text a declaration holds is read printable, a starter template keeping its newlines and tabs"
)]
#[ignore = "flipped by T3"]
fn a_declarations_text_is_read_printable() {
    let runtime = InProcessRuntime::new().with(hostile);
    let declaration = load_declaration(&runtime, EXT).unwrap().declaration;
    assert_eq!(
        declaration.handshake.starter_template.as_deref(),
        Some("a\n\tb\\u{1b}c")
    );
    let mut value = serde_json::to_value(&declaration).unwrap();
    value["handshake"]
        .as_object_mut()
        .unwrap()
        .remove("starter_template");
    assert_printable(&value);
    assert_eq!(
        declaration.handshake.description.as_deref(),
        Some(&*format!("d{}", printable(HOSTILE)))
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a key a load warning quotes is printable"
)]
#[ignore = "flipped by T3"]
fn a_load_warning_quotes_a_declared_key_printable() {
    let mut handshake = serde_json::to_value(hostile().declaration().handshake).unwrap();
    handshake["sandbox_policy"] = json!({ "x\u{1b}]0;k\u{7}": true });
    let runtime = InProcessRuntime::new()
        .with(hostile)
        .answer_raw(EXT, "__handshake", ok(&handshake))
        .answer_raw_to(
            EXT,
            "__describe",
            json!({ "category": "entities" }),
            ok(&json!({ "category": "entities", "items": [{ "name": "x", "t\u{1b}y": true }] })),
        );
    let warnings = load_declaration(&runtime, EXT).unwrap().warnings;
    let codes: Vec<&str> = warnings.iter().map(|w| w.code.as_str()).collect();
    assert_eq!(codes, ["W153", "W138"]);
    for warning in &warnings {
        assert_eq!(printable(&warning.message), warning.message.as_str());
        assert!(warning.message.contains("\\u{1b}"), "{}", warning.message);
    }
}

#[specforge_test_macros::test(
    behavior = "call_extension_exports",
    verify = "a pass's diagnostics and summary and a validator's verdict are read printable"
)]
#[ignore = "flipped by T3"]
fn a_pass_answers_text_is_read_printable() {
    let answer = json!({
        "diagnostics": [{
            "code": "G1", "severity": "Warning",
            "message": format!("m{HOSTILE}"), "suggestion": format!("s{BIDI}")
        }],
        "summary": { "k\u{1b}": "v\u{7}" }
    });
    let runtime =
        InProcessRuntime::new()
            .with(hostile)
            .answer_raw(EXT, "__pass_audit", ok(&answer));
    let output = ExtensionCalls::new(&runtime)
        .run_pass(EXT, "audit", &ExtensionCalls::encode(&PassInput::default()))
        .unwrap();
    assert_eq!(output.summary["k\\u{1b}"], "v\\u{7}");
    let diagnostics = pass_diagnostics(EXT, output, |_| None);
    assert_eq!(diagnostics[0].message, format!("m{}", printable(HOSTILE)));
    assert_eq!(
        diagnostics[0].suggestion.as_deref(),
        Some(&*format!("s{}", printable(BIDI)))
    );
}

#[specforge_test_macros::test(
    behavior = "call_extension_exports",
    verify = "a pass's diagnostics and summary and a validator's verdict are read printable"
)]
#[ignore = "flipped by T3"]
fn a_verdicts_text_is_read_printable() {
    let verdict = json!({ "verdict": "fail", "field": "f\u{1b}", "value": "v\u{7}" });
    let runtime =
        InProcessRuntime::new()
            .with(hostile)
            .answer_raw(EXT, "validate__x", ok(&verdict));
    let got = ExtensionCalls::new(&runtime)
        .validate(EXT, "validate__x", &context())
        .unwrap();
    assert_eq!(
        got,
        specforge_protocol_types::ValidatorVerdict::Fail {
            field: Some("f\\u{1b}".into()),
            value: Some("v\\u{7}".into())
        }
    );
}

#[specforge_test_macros::test(
    behavior = "call_extension_exports",
    verify = "a call failure's text from the guest is written printable in E028"
)]
#[ignore = "flipped by T3"]
fn a_call_failures_text_is_written_printable() {
    let trap = WasmCallResult::Trap(WasmTrapInfo {
        kind: "guest_error".into(),
        message: "x\u{1b}]0;t\u{7}\n= help: run it".into(),
        export_name: "collect__x".into(),
    });
    let runtime = InProcessRuntime::new()
        .with(hostile)
        .answer_raw(EXT, "collect__x", trap);
    let error = ExtensionCalls::new(&runtime)
        .collect(EXT, "collect__x", &Default::default())
        .unwrap_err();
    for text in [error.to_string(), error.diagnostic().message] {
        assert_eq!(printable(&text), text.as_str(), "{text:?}");
        assert!(text.contains("\\u{1b}"), "{text:?}");
    }
}

#[specforge_test_macros::test(
    behavior = "call_extension_exports",
    verify = "a command's output and an MCP tool's answer are read as the extension answered them"
)]
fn a_commands_and_tools_output_is_read_as_answered() {
    let command = json!({ "exit_code": 0, "stdout": "a\u{1b}[1mb\n", "stderr": "" });
    let tool = json!({ "t": "\u{1b}" });
    let runtime = InProcessRuntime::new()
        .with(hostile)
        .answer_raw(EXT, "cmd__hello", ok(&command))
        .answer_raw(EXT, "mcp__tool", ok(&tool));
    let calls = ExtensionCalls::new(&runtime);
    assert_eq!(
        calls.call_mcp_tool(EXT, "mcp__tool", &json!({})).unwrap(),
        tool
    );
    let output = calls
        .run_command(EXT, "cmd__hello", &command_input())
        .unwrap();
    assert_eq!(output.stdout, "a\u{1b}[1mb\n");
}

fn context() -> ValidatorContext {
    let wire = include_str!("wire/validate.input.json");
    serde_json::from_value(serde_json::from_str::<Value>(wire).unwrap()[1].clone()).unwrap()
}

fn command_input() -> specforge_protocol_types::CommandInput<specforge_protocol_types::RawGraph> {
    specforge_protocol_types::CommandInput {
        args: Default::default(),
        cwd: String::new(),
        format: Default::default(),
        today: String::new(),
        graph: specforge_protocol_types::RawGraph::new(r#"{"nodes":[],"edges":[]}"#.into())
            .unwrap(),
        evidence: Default::default(),
    }
}
