//! Phase 0 migration contract (`.plugin/migration-plan.md` §4).
//!
//! For every builtin that has a Wasm guest, the native `BuiltinRuntime`
//! (in-process mirror, the pre-migration oracle) and the `ExtismRuntime`
//! (real Wasm blob) must produce compatible protocol outputs for
//! `__handshake` and every `__describe` category.
//!
//! Contracts:
//! - `__handshake`: strict semantic equality — it gates flags and protocol
//!   version, so the tiers must agree exactly.
//! - `__describe`: every item the native tier serves must also be served by
//!   the Wasm tier (wasm ⊇ native). The inverse is deliberately not
//!   required: the native mirrors are known-deficient in places (e.g.
//!   `@specforge/formal` serves its four `#[compiler_pass]` passes through
//!   Wasm only — the CLI analyze path has always consumed them from Wasm).
//!   Post-migration behavior is the Wasm tier, so subset is the contract
//!   that protects consumers without misreporting that pre-existing
//!   asymmetry as drift.
//!
//! Comparison is *semantic* (`serde_json::Value`), not byte equality: each
//! tier serializes typed protocol structs independently, so key order and
//! whitespace differ while the protocol content must not.
//!
//! This harness is the safety net for the WASM-only migration: nothing on
//! the native side may be deleted until every assertion here passes, and
//! every phase boundary re-runs it. It is deleted together with the native
//! tier in Phase 7 — its job done.

use specforge_emitter::builtins::runtime_for_extensions;
use specforge_extism::{ExtismRuntime, builtins};
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};

/// Declarative builtins: serve the nine standard describe categories.
const DECLARATIVE: &[&str] = &[
    "@specforge/product",
    "@specforge/software",
    "@specforge/governance",
    "@specforge/formal",
];

/// Builtin guests that currently ship a Wasm blob.
const GUEST_BACKED: &[&str] = &[
    "@specforge/product",
    "@specforge/software",
    "@specforge/governance",
    "@specforge/formal",
    "@specforge/rust",
    "@specforge/typescript",
];

/// Describe categories served per extension: the declarative builtins answer
/// the nine standard categories; the scanner guests answer only `analyzers`.
fn describe_categories(extension: &str) -> &'static [&'static str] {
    if DECLARATIVE.contains(&extension) {
        &[
            "entities",
            "edges",
            "fields",
            "shared_fields",
            "enhancements",
            "validation_rules",
            "surfaces",
            "passes",
            "feature_flags",
        ]
    } else {
        &["analyzers"]
    }
}

fn wasm_runtime() -> ExtismRuntime {
    let runtime = ExtismRuntime::new();
    builtins::load_builtins(&runtime).expect("builtin Wasm blobs must load");
    runtime
}

fn native_runtime() -> specforge_wasm::BuiltinRuntime {
    let names: Vec<String> = GUEST_BACKED.iter().map(|s| s.to_string()).collect();
    runtime_for_extensions(&names)
}

fn unwrap_pair(
    extension: &str,
    call: &str,
    native: WasmCallResult,
    wasm: WasmCallResult,
) -> (Vec<u8>, Vec<u8>) {
    match (native, wasm) {
        (WasmCallResult::Ok(n), WasmCallResult::Ok(w)) => (n, w),
        (WasmCallResult::Trap(t), _) => {
            panic!("parity[{extension}/{call}]: native tier trapped: {t:?}");
        }
        (_, WasmCallResult::Trap(t)) => {
            panic!("parity[{extension}/{call}]: wasm tier trapped: {t:?}");
        }
    }
}

fn parse_output(extension: &str, call: &str, bytes: &[u8], tier: &str) -> serde_json::Value {
    serde_json::from_slice(bytes)
        .unwrap_or_else(|e| panic!("parity[{extension}/{call}]: {tier} output not JSON: {e}"))
}

/// First-divergence description for strict comparison failures.
fn json_diff(native: &serde_json::Value, wasm: &serde_json::Value) -> String {
    match (native, wasm) {
        (serde_json::Value::Object(n), serde_json::Value::Object(w)) => {
            for (key, nv) in n {
                match w.get(key) {
                    None => return format!("key '{key}' present natively, missing in wasm"),
                    Some(wv) if wv != nv => {
                        return format!(
                            "key '{key}' diverges:\n  native: {}\n  wasm:   {}",
                            serde_json::to_string_pretty(nv).unwrap_or_default(),
                            serde_json::to_string_pretty(wv).unwrap_or_default()
                        );
                    }
                    Some(_) => continue,
                }
            }
            for key in w.keys() {
                if !n.contains_key(key) {
                    return format!("key '{key}' present in wasm, missing natively");
                }
            }
            "objects equal (unexpected)".into()
        }
        (serde_json::Value::Array(n), serde_json::Value::Array(w)) => {
            if n.len() != w.len() {
                return format!("array length differs: native {}, wasm {}", n.len(), w.len());
            }
            for (i, (nv, wv)) in n.iter().zip(w.iter()).enumerate() {
                if nv != wv {
                    return format!(
                        "array item [{i}] diverges:\n  native: {}\n  wasm:   {}",
                        serde_json::to_string_pretty(nv).unwrap_or_default(),
                        serde_json::to_string_pretty(wv).unwrap_or_default()
                    );
                }
            }
            "arrays equal (unexpected)".into()
        }
        _ => format!("scalars differ:\n  native: {native}\n  wasm:   {wasm}"),
    }
}

#[test]
fn handshake_parity_guest_backed_builtins() {
    for extension in GUEST_BACKED {
        let native = native_runtime().call_export(extension, "__handshake", &[]);
        let wasm = wasm_runtime().call_export(extension, "__handshake", &[]);
        let (native_bytes, wasm_bytes) = unwrap_pair(extension, "__handshake", native, wasm);
        let native_json = parse_output(extension, "__handshake", &native_bytes, "native");
        let wasm_json = parse_output(extension, "__handshake", &wasm_bytes, "wasm");
        if native_json != wasm_json {
            panic!(
                "parity[{extension}/__handshake]: native and wasm handshake diverge\n{}",
                json_diff(&native_json, &wasm_json)
            );
        }
    }
}

#[test]
fn describe_parity_all_categories_guest_backed_builtins() {
    for extension in GUEST_BACKED {
        for category in describe_categories(extension) {
            let input = format!(r#"{{"category":"{category}"}}"#);
            let native = native_runtime().call_export(extension, "__describe", input.as_bytes());
            let wasm = wasm_runtime().call_export(extension, "__describe", input.as_bytes());
            let (native_bytes, wasm_bytes) = unwrap_pair(extension, "__describe", native, wasm);
            let native_json = parse_output(extension, "__describe", &native_bytes, "native");
            let wasm_json = parse_output(extension, "__describe", &wasm_bytes, "wasm");

            if native_json["category"] != wasm_json["category"] {
                panic!(
                    "parity[{extension}/{category}]: category echo differs: native {}, wasm {}",
                    native_json["category"], wasm_json["category"]
                );
            }

            let (Some(native_items), Some(wasm_items)) = (
                native_json["items"].as_array(),
                wasm_json["items"].as_array(),
            ) else {
                panic!(
                    "parity[{extension}/{category}]: 'items' must be an array on both tiers\nnative: {native_json}\nwasm: {wasm_json}"
                );
            };

            for item in native_items {
                if !wasm_items.contains(item) {
                    panic!(
                        "parity[{extension}/{category}]: native item missing from wasm output:\n{}\n(wasm served {} items, native served {})",
                        serde_json::to_string_pretty(item).unwrap_or_default(),
                        wasm_items.len(),
                        native_items.len()
                    );
                }
            }
        }
    }
}

#[test]
fn unknown_category_traps_on_both_tiers() {
    let input = br#"{"category":"does_not_exist"}"#;
    for extension in GUEST_BACKED {
        let native = native_runtime().call_export(extension, "__describe", input);
        let wasm = wasm_runtime().call_export(extension, "__describe", input);
        assert!(
            matches!(native, WasmCallResult::Trap(_)),
            "native must trap on unknown category"
        );
        assert!(
            matches!(wasm, WasmCallResult::Trap(_)),
            "wasm must trap on unknown category"
        );
    }
}

#[test]
fn unknown_extension_traps_on_both_tiers() {
    let native = native_runtime().call_export("@no/such", "__handshake", &[]);
    let wasm = wasm_runtime().call_export("@no/such", "__handshake", &[]);
    assert!(matches!(native, WasmCallResult::Trap(_)));
    assert!(matches!(wasm, WasmCallResult::Trap(_)));
}

/// Analyzer parity: scanner guests must answer `scan__*`, `classify__*`, and
/// `map__*` byte-identically to the native mirror over a fixture matrix that
/// exercises every code path — public items, private items, comments,
/// test-file suppression, classification, and symbol-mapping strategies.
#[test]
fn analyzer_parity_scan_classify_map() {
    let rust_source = "\
use std::fmt;

/// Doc comment, not an item.
pub fn visible_helper(count: usize) -> bool {
    count > 0
}

fn private_helper() {}

pub struct Config {
    pub retries: u32,
}

pub enum Mode {
    Fast,
    Safe,
}

pub trait Runnable {
    fn run(&self);
}

pub const MAX_RETRIES: u32 = 3;
";

    let ts_source = "\
import { z } from 'zod';

// a comment
export function createUser(name: string): boolean {
    return name.length > 0;
}

function internalHelper() {}

export class UserStore {
    save(): void {}
}

export interface UserRecord {
    id: string;
}

export const DEFAULT_LIMIT = 10;
";

    let cases: &[(&str, &str, String)] = &[
        // ── scan ──
        (
            "@specforge/rust",
            "scan__rust",
            scan("src/lib.rs", rust_source),
        ),
        (
            "@specforge/rust",
            "scan__rust",
            scan("tests/integration.rs", rust_source),
        ),
        (
            "@specforge/typescript",
            "scan__typescript",
            scan("src/app.ts", ts_source),
        ),
        (
            "@specforge/typescript",
            "scan__typescript",
            scan("src/app.test.ts", ts_source),
        ),
        // ── classify ──
        (
            "@specforge/rust",
            "classify__rust",
            classify(
                "src/lib.rs",
                r#"[
                    {"name":"visible_helper","item_kind":"function","line":4},
                    {"name":"Config","item_kind":"struct","line":10},
                    {"name":"Mode","item_kind":"enum","line":15},
                    {"name":"Runnable","item_kind":"trait","line":20}
                ]"#,
            ),
        ),
        (
            "@specforge/typescript",
            "classify__typescript",
            classify(
                "src/app.ts",
                r#"[
                    {"name":"createUser","item_kind":"function","line":4},
                    {"name":"UserStore","item_kind":"class","line":10},
                    {"name":"UserRecord","item_kind":"interface","line":14}
                ]"#,
            ),
        ),
        // ── map ──
        (
            "@specforge/rust",
            "map__rust",
            map(
                "visible_helper",
                "function",
                "src/lib.rs",
                &["visible_helper", "other"],
            ),
        ),
        (
            "@specforge/rust",
            "map__rust",
            map("parse_config", "function", "src/lib.rs", &["unrelated"]),
        ),
        (
            "@specforge/typescript",
            "map__typescript",
            map("UserStore", "class", "src/app.ts", &["user_store"]),
        ),
        (
            "@specforge/typescript",
            "map__typescript",
            map("Missing", "function", "src/app.ts", &[]),
        ),
    ];

    for (extension, export, input) in cases {
        let native = native_runtime().call_export(extension, export, input.as_bytes());
        let wasm = wasm_runtime().call_export(extension, export, input.as_bytes());
        let (native_bytes, wasm_bytes) = unwrap_pair(extension, export, native, wasm);
        assert_eq!(
            native_bytes, wasm_bytes,
            "parity[{extension}/{export}]: analyzer outputs diverge (input: {})",
            input
        );
    }
}

fn scan(file_path: &str, content: &str) -> String {
    format!(
        r#"{{"file_path":"{file_path}","content":{}}}"#,
        serde_json::to_string(content).unwrap()
    )
}

fn classify(file_path: &str, items: &str) -> String {
    format!(r#"{{"items":{items},"file_path":"{file_path}"}}"#)
}

fn map(name: &str, item_kind: &str, file_path: &str, existing: &[&str]) -> String {
    let ids: Vec<String> = existing.iter().map(|e| format!("\"{e}\"")).collect();
    format!(
        r#"{{"name":"{name}","item_kind":"{item_kind}","file_path":"{file_path}","existing_entity_ids":[{}]}}"#,
        ids.join(",")
    )
}
