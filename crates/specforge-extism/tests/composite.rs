use std::io::Write as _;
use std::path::PathBuf;

use specforge_extism::{CompositeRuntime, ExtismRuntime};
use specforge_wasm::builtin::{BuiltinExtension, BuiltinRuntime};
use specforge_wasm::protocol::{DescribeResponse, HandshakeResponse, ContributionFlags};
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};

struct TestBuiltinExtension;

impl BuiltinExtension for TestBuiltinExtension {
    fn handshake(&self) -> HandshakeResponse {
        HandshakeResponse {
            protocol_version: "1.0.0".to_string(),
            name: "@test/builtin".to_string(),
            version: "1.0.0".to_string(),
            contribution_flags: ContributionFlags {
                entities: true,
                ..Default::default()
            },
            peer_dependencies: vec![],
            sandbox_policy: None,
        }
    }

    fn describe(&self, category: &str) -> Option<DescribeResponse> {
        if category == "entities" {
            Some(DescribeResponse {
                category: "entities".to_string(),
                items: serde_json::json!([{
                    "name": "test_kind",
                    "fields": [],
                    "testable": false,
                    "singleton": false,
                    "supports_verify": false
                }]),
            })
        } else {
            None
        }
    }
}

fn fixture_wasm_path() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/test-extension/target/wasm32-unknown-unknown/release/specforge_test_extension.wasm")
}

fn has_fixture() -> bool {
    fixture_wasm_path().exists()
}

// --- Behavior 1: Builtins are dispatched for known extension names ---

#[test]
fn composite_dispatches_to_builtin_for_registered_name() {
    let builtin = BuiltinRuntime::new()
        .with_extension("@test/builtin", Box::new(TestBuiltinExtension));
    let extism = ExtismRuntime::new();
    let runtime = CompositeRuntime::new(builtin, extism);

    let input = serde_json::to_vec(&serde_json::json!({
        "host_version": "1.0.0",
        "supported_categories": ["entities"]
    }))
    .unwrap();

    let result = runtime.call_export("@test/builtin", "__handshake", &input);
    match result {
        WasmCallResult::Ok(output) => {
            let response: serde_json::Value = serde_json::from_slice(&output).unwrap();
            assert_eq!(response["name"], "@test/builtin");
            assert_eq!(response["protocol_version"], "1.0.0");
        }
        WasmCallResult::Trap(t) => panic!("Expected Ok, got trap: {:?}", t),
    }
}

// --- Behavior 2: Wasm extensions dispatched via Extism ---

#[test]
fn composite_dispatches_to_extism_for_wasm_extension() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }

    let builtin = BuiltinRuntime::new()
        .with_extension("@test/builtin", Box::new(TestBuiltinExtension));
    let extism = ExtismRuntime::new();
    let runtime = CompositeRuntime::new(builtin, extism);

    // Load a Wasm extension
    runtime
        .load_wasm_extension("@test/wasm-ext", &fixture_wasm_path())
        .unwrap();

    let input = serde_json::to_vec(&serde_json::json!({
        "host_version": "1.0.0",
        "supported_categories": ["entities"]
    }))
    .unwrap();

    // Wasm extension responds correctly
    let result = runtime.call_export("@test/wasm-ext", "__handshake", &input);
    match result {
        WasmCallResult::Ok(output) => {
            let response: serde_json::Value = serde_json::from_slice(&output).unwrap();
            assert_eq!(response["name"], "@test/hello");
        }
        WasmCallResult::Trap(t) => panic!("Expected Ok, got trap: {:?}", t),
    }

    // Builtin still works in same runtime
    let result = runtime.call_export("@test/builtin", "__handshake", &input);
    assert!(matches!(result, WasmCallResult::Ok(_)));
}

// --- Behavior 3: Unknown extension in both runtimes returns trap ---

#[test]
fn composite_unknown_extension_returns_trap() {
    let builtin = BuiltinRuntime::new();
    let extism = ExtismRuntime::new();
    let runtime = CompositeRuntime::new(builtin, extism);

    let result = runtime.call_export("@nonexistent/ext", "__handshake", b"{}");
    match result {
        WasmCallResult::Trap(t) => {
            assert_eq!(t.kind, "extension_not_found");
        }
        WasmCallResult::Ok(_) => panic!("Expected trap"),
    }
}

// --- Behavior 4: load_wasm_extension makes it callable ---

#[test]
fn load_wasm_extension_registers_under_given_name() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    // Write minimal valid wasm
    {
        let mut f = &tmp;
        f.write_all(&[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00])
            .unwrap();
        f.flush().unwrap();
    }

    let builtin = BuiltinRuntime::new();
    let extism = ExtismRuntime::new();
    let runtime = CompositeRuntime::new(builtin, extism);

    runtime
        .load_wasm_extension("@my/custom", tmp.path())
        .unwrap();

    // Calling a non-existent export on a loaded module gives call_failed (not extension_not_found)
    let result = runtime.call_export("@my/custom", "no_such_export", b"{}");
    match result {
        WasmCallResult::Trap(t) => {
            assert_eq!(t.kind, "call_failed", "Expected call_failed, got: {:?}", t);
        }
        WasmCallResult::Ok(_) => panic!("Expected trap for nonexistent export"),
    }
}
