//! Proof that an extension authored with `specforge-extension-sdk` passes the
//! host's protocol (handshake + describe) end to end through the component
//! runtime. The fixture's component blob is vendored at
//! `fixtures/greet-extension/greet.wasm` (refresh:
//! `cd fixtures/greet-extension && cargo build --release --target wasm32-wasip2`).

use std::path::{Path, PathBuf};

use specforge_component::ComponentRuntime;
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};

fn greet_wasm_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/greet-extension/greet.wasm")
}

#[test]
fn sdk_greet_extension_passes_protocol() {
    let blob = std::fs::read(greet_wasm_path()).expect("vendored greet component blob");
    let runtime = ComponentRuntime::new();
    runtime
        .load_module_bytes("@sdk/greet", &blob)
        .expect("greet component instantiates");

    // __handshake: identity, derived flags, protocol version.
    let hs = runtime.call_export("@sdk/greet", "__handshake", b"");
    let bytes = match hs {
        WasmCallResult::Ok(bytes) => bytes,
        other => panic!("handshake failed: {other:?}"),
    };
    let hs: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(hs["name"], "@sdk/greet");
    assert_eq!(hs["version"], "0.1.0");
    assert_eq!(hs["protocol_version"], "1.0.0");
    assert_eq!(hs["contribution_flags"]["entities"], true);
    assert_eq!(hs["contribution_flags"]["validators"], true);
    assert_eq!(hs["contribution_flags"]["renderers"], false);

    // __describe entities: the greeting kind with its style field.
    let de = runtime.call_export("@sdk/greet", "__describe", br#"{"category":"entities"}"#);
    let bytes = match de {
        WasmCallResult::Ok(bytes) => bytes,
        other => panic!("describe failed: {other:?}"),
    };
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["category"], "entities");
    assert_eq!(v["items"][0]["name"], "greeting");
    assert_eq!(v["items"][0]["fields"][0]["name"], "style");
    assert_eq!(v["items"][0]["fields"][0]["field_type"], "enum");
    assert_eq!(v["items"][0]["fields"][0]["enum_values"][0], "warm");

    // __describe validation_rules: the contributed rule round-trips.
    let dr = runtime.call_export(
        "@sdk/greet",
        "__describe",
        br#"{"category":"validation_rules"}"#,
    );
    let bytes = match dr {
        WasmCallResult::Ok(bytes) => bytes,
        other => panic!("describe rules failed: {other:?}"),
    };
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["items"][0]["code"], "G101");
    assert_eq!(v["items"][0]["severity"], "error");
    assert_eq!(v["items"][0]["check"], "field_value_constraint");

    // __describe: unsupported category mirrors the builtin error behavior.
    let bad = runtime.call_export("@sdk/greet", "__describe", br#"{"category":"nope"}"#);
    assert!(
        matches!(bad, WasmCallResult::Trap(_)),
        "expected trap for unsupported category"
    );
}
