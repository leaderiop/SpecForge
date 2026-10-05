//! Characterization of today's extension-call wire (plan 04, T1): the bytes
//! the host sends and how it reads what comes back, for the declaration
//! (handshake/describe) calls. Pins, not proofs: a pin that encodes a bug
//! says which ticket flips it. The command, MCP tool and resource pins
//! flipped in T5: those calls go through `ExtensionCalls` (`tests/calls.rs`),
//! where an answer that is not the protocol type is E028.
//!
//! Goldens live in `tests/wire/` (one JSON per family and direction),
//! compared as JSON values; `SPECFORGE_BLESS=1` rewrites them.

use serde_json::Value;
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
