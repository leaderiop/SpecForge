use std::io::Write as _;
use std::path::{Path, PathBuf};

use specforge_extism::ExtismRuntime;
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};

/// Path to the pre-built test extension Wasm fixture.
/// Built separately: cargo build --manifest-path fixtures/test-extension/Cargo.toml --target wasm32-unknown-unknown --release
fn fixture_wasm_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/test-extension/target/wasm32-unknown-unknown/release/specforge_test_extension.wasm")
}

fn has_fixture() -> bool {
    fixture_wasm_path().exists()
}

/// Creates a minimal valid Wasm module (empty, but passes magic byte validation).
/// This is a real valid Wasm module with no exports - useful for testing load behavior.
fn minimal_wasm_module() -> Vec<u8> {
    // Minimal valid Wasm binary: magic + version + empty module
    // \0asm (magic) + version 1 (little-endian u32)
    vec![
        0x00, 0x61, 0x73, 0x6D, // \0asm magic
        0x01, 0x00, 0x00, 0x00, // version 1
    ]
}

/// Creates a temp file containing a valid minimal Wasm module.
fn write_minimal_wasm() -> tempfile::NamedTempFile {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.write_all(&minimal_wasm_module()).unwrap();
    tmp.flush().unwrap();
    tmp
}

// --- Behavior 1: Load a valid Wasm module ---

#[test]
fn load_minimal_wasm_module_succeeds() {
    let tmp = write_minimal_wasm();
    let runtime = ExtismRuntime::new();
    let result = runtime.load_module(tmp.path(), None);
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
}

#[test]
fn load_fixture_wasm_module() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }
    let runtime = ExtismRuntime::new();
    let result = runtime.load_module(&fixture_wasm_path(), None);
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
}

// --- Behavior 2: Call __handshake and get JSON response ---

#[test]
fn call_handshake_returns_json() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }
    let runtime = ExtismRuntime::new();
    runtime.load_module(&fixture_wasm_path(), None).unwrap();

    let input = serde_json::to_vec(&serde_json::json!({
        "host_version": "1.0.0",
        "supported_categories": ["entities"]
    }))
    .unwrap();

    let ext_name = "specforge_test_extension";
    let result = runtime.call_export(ext_name, "__handshake", &input);
    match result {
        WasmCallResult::Ok(output) => {
            let response: serde_json::Value = serde_json::from_slice(&output).unwrap();
            assert_eq!(response["protocol_version"], "1.0.0");
            assert_eq!(response["name"], "@test/hello");
            assert_eq!(response["version"], "0.1.0");
            assert_eq!(response["contribution_flags"]["entities"], true);
        }
        WasmCallResult::Trap(trap) => {
            panic!("Expected Ok, got trap: {:?}", trap);
        }
    }
}

// --- Behavior 3: Call export on unknown extension ---

#[test]
fn call_unknown_extension_returns_trap() {
    let runtime = ExtismRuntime::new();
    let result = runtime.call_export("nonexistent", "__handshake", b"{}");
    match result {
        WasmCallResult::Trap(trap) => {
            assert_eq!(trap.kind, "extension_not_found");
            assert!(trap.message.contains("nonexistent"));
        }
        WasmCallResult::Ok(_) => panic!("Expected trap for unknown extension"),
    }
}

// --- Behavior 4: Call __describe and get entity descriptors ---

#[test]
fn call_describe_returns_entities() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }
    let runtime = ExtismRuntime::new();
    runtime.load_module(&fixture_wasm_path(), None).unwrap();

    let input = serde_json::to_vec(&serde_json::json!({
        "category": "entities"
    }))
    .unwrap();

    let ext_name = "specforge_test_extension";
    let result = runtime.call_export(ext_name, "__describe", &input);
    match result {
        WasmCallResult::Ok(output) => {
            let response: serde_json::Value = serde_json::from_slice(&output).unwrap();
            assert_eq!(response["category"], "entities");
            let items = response["items"].as_array().unwrap();
            assert_eq!(items.len(), 1);
            assert_eq!(items[0]["name"], "widget");
        }
        WasmCallResult::Trap(trap) => {
            panic!("Expected Ok, got trap: {:?}", trap);
        }
    }
}

// --- Behavior 5: Multiple extensions loaded independently ---

#[test]
fn multiple_extensions_dispatch_independently() {
    let wasm1 = write_minimal_wasm();
    let wasm2 = write_minimal_wasm();

    let runtime = ExtismRuntime::new();
    runtime.load_module(wasm1.path(), None).unwrap();
    runtime.load_module(wasm2.path(), None).unwrap();

    // Unknown extension still traps even after loading others
    let result = runtime.call_export("other_extension", "__handshake", b"{}");
    assert!(matches!(result, WasmCallResult::Trap(_)));
}

// --- Behavior 6: Invalid Wasm binary returns error ---

#[test]
fn load_invalid_wasm_returns_error() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.write_all(b"not a wasm file").unwrap();
    tmp.flush().unwrap();

    let runtime = ExtismRuntime::new();
    let result = runtime.load_module(tmp.path(), None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Invalid Wasm binary"),
        "Expected 'Invalid Wasm binary', got: {}",
        err
    );
}

// --- Behavior 7: Missing file returns error ---

#[test]
fn load_missing_file_returns_error() {
    let runtime = ExtismRuntime::new();
    let result = runtime.load_module(Path::new("/nonexistent/path.wasm"), None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("not found"), "Got: {}", err);
}

// --- Behavior 8: AOT cache check ---

#[test]
fn has_cached_module_false_without_cache_dir() {
    let runtime = ExtismRuntime::new();
    assert!(!runtime.has_cached_module("abc123"));
}

#[test]
fn has_cached_module_false_when_file_missing() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = ExtismRuntime::new().with_aot_cache_dir(dir.path().to_path_buf());
    assert!(!runtime.has_cached_module("abc123"));
}

#[test]
fn has_cached_module_true_when_file_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("abc123.aot"), b"cached").unwrap();
    let runtime = ExtismRuntime::new().with_aot_cache_dir(dir.path().to_path_buf());
    assert!(runtime.has_cached_module("abc123"));
}

// --- Behavior: Call nonexistent export on loaded module returns trap ---

#[test]
fn call_nonexistent_export_returns_trap() {
    let tmp = write_minimal_wasm();
    let runtime = ExtismRuntime::new();
    runtime.load_module(tmp.path(), None).unwrap();

    let ext_name = tmp.path().file_stem().unwrap().to_str().unwrap();
    let result = runtime.call_export(ext_name, "nonexistent_fn", b"{}");
    match result {
        WasmCallResult::Trap(trap) => {
            assert_eq!(trap.kind, "call_failed");
            assert_eq!(trap.export_name, "nonexistent_fn");
        }
        WasmCallResult::Ok(_) => panic!("Expected trap for nonexistent export"),
    }
}

// --- Behavior: load_module_as allows custom extension names ---

#[test]
fn load_module_as_custom_name() {
    let tmp = write_minimal_wasm();
    let runtime = ExtismRuntime::new();
    runtime
        .load_module_as("@my/extension", tmp.path(), None)
        .unwrap();

    // Can dispatch by custom name
    let result = runtime.call_export("@my/extension", "nonexistent_fn", b"{}");
    // Should get call_failed (extension found, but no export), not extension_not_found
    match result {
        WasmCallResult::Trap(trap) => {
            assert_eq!(trap.kind, "call_failed");
        }
        WasmCallResult::Ok(_) => panic!("Expected trap"),
    }
}
