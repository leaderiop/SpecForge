//! Characterization of today's extension-call wire (plan 04, T1): the bytes
//! the host sends and how it reads what comes back, for the command, MCP
//! tool, MCP resource and declaration (handshake/describe) calls. Pins, not
//! proofs: a pin that encodes a bug says which ticket flips it.
//!
//! Goldens live in `tests/wire/` (one JSON per family and direction),
//! compared as JSON values; `SPECFORGE_BLESS=1` rewrites them.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;
use specforge_wasm::protocol::{ProtocolError, load_declaration};
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{WasmCallResult, WasmTrapInfo};
use std::path::{Path, PathBuf};

const EXT: &str = "@x/y";

pub(crate) fn wire_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/wire")
}

/// `value` with every object's keys sorted, for a stable golden file.
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

/// Assert `actual` equals the golden `name` (rewritten under SPECFORGE_BLESS).
pub(crate) fn golden(name: &str, actual: &Value) {
    let path = wire_dir().join(name);
    if std::env::var_os("SPECFORGE_BLESS").is_some() {
        let mut text = serde_json::to_string_pretty(&sorted(actual)).unwrap();
        text.push('\n');
        std::fs::write(&path, text).unwrap();
    }
    let expected: Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("golden {}: {e}", path.display())),
    )
    .unwrap();
    assert_eq!(actual, &expected, "golden {name}");
}

fn raw(bytes: &[u8]) -> WasmCallResult {
    WasmCallResult::Ok(bytes.to_vec())
}

fn trap(kind: &str, message: &str, export: &str) -> WasmCallResult {
    WasmCallResult::Trap(WasmTrapInfo {
        kind: kind.to_string(),
        message: message.to_string(),
        export_name: export.to_string(),
    })
}

fn answering(export: &str, result: WasmCallResult) -> InProcessRuntime {
    InProcessRuntime::new().answer_raw(EXT, export, result)
}

// ── C1 · command ──

#[test]
fn c1_a_well_formed_command_answer_is_read_field_by_field() {
    let runtime = answering(
        "cmd__x",
        raw(br#"{"exit_code":3,"stdout":"out","stderr":"err"}"#),
    );
    let out = specforge_wasm::dispatch_surface_command(EXT, "cmd__x", b"{}", &runtime).unwrap();
    assert_eq!(
        (out.exit_code, out.stdout.as_slice(), out.stderr.as_slice()),
        (3, &b"out"[..], &b"err"[..])
    );
}

#[test]
fn c1_a_malformed_command_answer_is_exit_0() {
    // pinned: flips in T5 (each is E028)
    for (answer, stdout) in [
        (&b"not json at all"[..], &b"not json at all"[..]),
        (br#"{"exit_code":"3","stdout":"x"}"#, b"x"),
        (br#"{}"#, b""),
        (br#"[1,2]"#, b""),
    ] {
        let runtime = answering("cmd__x", raw(answer));
        let out = specforge_wasm::dispatch_surface_command(EXT, "cmd__x", b"{}", &runtime).unwrap();
        assert_eq!(out.exit_code, 0, "{}", String::from_utf8_lossy(answer));
        assert_eq!(out.stdout, stdout);
    }
}

#[test]
fn c1_a_trapping_command_is_e028() {
    let runtime = answering("cmd__x", trap("unreachable", "p", "cmd__x"));
    let err = specforge_wasm::dispatch_surface_command(EXT, "cmd__x", b"{}", &runtime).unwrap_err();
    assert_eq!(err.code, "E028");
    assert_eq!(
        err.message,
        "surface command cmd__x() trapped: unreachable — p"
    );
}

// ── C2 · MCP tool ──

#[test]
fn c2_tool_arguments_pass_through_byte_identical() {
    let runtime = answering("mcp__t", raw(br#"{"ok":true}"#));
    let arguments = br#"{"query":"x","limit":2}"#;
    let value =
        specforge_wasm::dispatch_surface_mcp_tool(EXT, "mcp__t", arguments, &runtime).unwrap();
    assert_eq!(value, json!({"ok": true}));
    let calls = runtime.calls();
    assert_eq!(calls[0].input, json!({"query": "x", "limit": 2}));
    // Any JSON is a tool answer.
    for answer in [&b"[1,2]"[..], b"3", b"\"s\""] {
        let runtime = answering("mcp__t", raw(answer));
        assert!(specforge_wasm::dispatch_surface_mcp_tool(EXT, "mcp__t", b"{}", &runtime).is_ok());
    }
}

#[test]
fn c2_a_non_json_tool_answer_or_a_trap_is_e028() {
    let runtime = answering("mcp__t", raw(b"oops"));
    let err =
        specforge_wasm::dispatch_surface_mcp_tool(EXT, "mcp__t", b"{}", &runtime).unwrap_err();
    assert_eq!(err.code, "E028");
    assert!(
        err.message
            .starts_with("MCP tool mcp__t() returned invalid JSON: "),
        "{}",
        err.message
    );
    let runtime = answering("mcp__t", trap("k", "m", "mcp__t"));
    let err =
        specforge_wasm::dispatch_surface_mcp_tool(EXT, "mcp__t", b"{}", &runtime).unwrap_err();
    assert_eq!(err.message, "MCP tool mcp__t() trapped: k — m");
}

// ── C3 · MCP resource ──

#[test]
fn c3_a_resource_receives_its_uri() {
    let runtime = answering(
        "mcp__r",
        raw(br#"{"content":"c","mime_type":"text/plain"}"#),
    );
    let (content, mime) =
        specforge_wasm::dispatch_surface_mcp_resource(EXT, "mcp__r", "u://r", &runtime).unwrap();
    assert_eq!(
        (content.as_slice(), mime.as_str()),
        (&b"c"[..], "text/plain")
    );
    golden("resource.input.json", &runtime.calls()[0].input);
}

#[test]
fn c3_a_malformed_resource_answer_is_served_as_octet_stream() {
    // pinned: flips in T5 (each is E028)
    for (answer, content) in [(&b"oops"[..], &b"oops"[..]), (br#"{"text":"t"}"#, b"")] {
        let runtime = answering("mcp__r", raw(answer));
        let (served, mime) =
            specforge_wasm::dispatch_surface_mcp_resource(EXT, "mcp__r", "u://r", &runtime)
                .unwrap();
        assert_eq!(served, content);
        assert_eq!(mime, "application/octet-stream");
    }
}

#[test]
fn c3_a_trapping_resource_is_e028() {
    let runtime = answering("mcp__r", trap("k", "m", "mcp__r"));
    let err = specforge_wasm::dispatch_surface_mcp_resource(EXT, "mcp__r", "u://r", &runtime)
        .unwrap_err();
    assert_eq!(err.code, "E028");
    assert_eq!(err.message, "MCP resource mcp__r() trapped: k — m");
}

// ── C9 · handshake and describe ──

fn greet() -> ContributionsBuilder {
    let mut b = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    b.kind("greeting", |k| {
        k.description("g");
    });
    b
}

#[test]
fn c9_the_handshake_and_describe_requests() {
    let runtime = InProcessRuntime::new().with(greet);
    load_declaration(&runtime, EXT).unwrap();
    let calls = runtime.calls();
    golden("handshake.input.json", &calls[0].input);
    assert_eq!(calls[1].export, "__describe");
    golden("describe.input.json", &calls[1].input);
}

#[test]
fn c9_a_failing_handshake_or_describe_is_a_protocol_error() {
    let runtime = answering("__handshake", trap("guest_error", "m", "__handshake"));
    assert_eq!(
        load_declaration(&runtime, EXT).unwrap_err(),
        ProtocolError::HandshakeFailed("guest_error: m".to_string())
    );
    let runtime = InProcessRuntime::new().with(greet).answer_raw(
        EXT,
        "__describe",
        trap("k", "m", "__describe"),
    );
    assert_eq!(
        load_declaration(&runtime, EXT).unwrap_err(),
        ProtocolError::DescribeFailed {
            category: "entities".to_string(),
            reason: "k: m".to_string()
        }
    );
    let runtime = answering("__handshake", raw(b"{}"));
    assert!(matches!(
        load_declaration(&runtime, EXT).unwrap_err(),
        ProtocolError::DeserializationError(_)
    ));
}
