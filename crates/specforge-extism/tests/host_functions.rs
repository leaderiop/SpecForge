use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use specforge_common::{Diagnostic, Severity};
use specforge_extism::{ExtismRuntime, HostContext};
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};

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

#[test]
fn wasm_plugin_can_emit_diagnostic_via_host_function() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }

    let diagnostics: Arc<Mutex<Vec<Diagnostic>>> = Arc::new(Mutex::new(Vec::new()));
    let ctx = HostContext::new(diagnostics.clone());

    let runtime = ExtismRuntime::with_host_context(ctx);
    runtime
        .load_module_as("@test/hello", &fixture_wasm_path(), None)
        .unwrap();

    let result = runtime.call_export("@test/hello", "test_emit_diagnostic", b"");
    assert!(
        matches!(&result, WasmCallResult::Ok(_)),
        "Expected Ok, got: {:?}",
        result
    );

    let diags = diagnostics.lock().unwrap();
    assert_eq!(diags.len(), 1, "Expected 1 diagnostic, got {}", diags.len());
    assert_eq!(diags[0].code, "W100");
    assert_eq!(diags[0].severity, Severity::Warning);
    assert_eq!(diags[0].message, "test warning from wasm");
}

#[test]
fn wasm_plugin_emit_diagnostic_malformed_json_produces_error_diagnostic() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }

    let diagnostics: Arc<Mutex<Vec<Diagnostic>>> = Arc::new(Mutex::new(Vec::new()));
    let ctx = HostContext::new(diagnostics.clone());

    let runtime = ExtismRuntime::with_host_context(ctx);
    runtime
        .load_module_as("@test/hello", &fixture_wasm_path(), None)
        .unwrap();

    let result = runtime.call_export("@test/hello", "test_emit_malformed_diagnostic", b"");
    assert!(
        matches!(&result, WasmCallResult::Ok(_)),
        "Expected Ok (host handles error gracefully), got: {:?}",
        result
    );

    let diags = diagnostics.lock().unwrap();
    assert_eq!(diags.len(), 1, "Expected 1 error diagnostic, got {}", diags.len());
    assert_eq!(diags[0].code, "E028");
    assert_eq!(diags[0].severity, Severity::Error);
    assert!(diags[0].message.contains("malformed JSON"));
}

#[test]
fn wasm_plugin_can_read_file_under_spec_root() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let spec_root = tmp.path().to_path_buf();
    std::fs::write(spec_root.join("test.spec"), "hello from spec file").unwrap();

    let diagnostics: Arc<Mutex<Vec<Diagnostic>>> = Arc::new(Mutex::new(Vec::new()));
    let ctx = HostContext::new(diagnostics.clone())
        .with_spec_root(spec_root.clone());

    let runtime = ExtismRuntime::with_host_context(ctx);
    runtime
        .load_module_as("@test/hello", &fixture_wasm_path(), None)
        .unwrap();

    let request = serde_json::json!({
        "path": spec_root.join("test.spec").to_string_lossy()
    });
    let result = runtime.call_export(
        "@test/hello",
        "test_read_file",
        &serde_json::to_vec(&request).unwrap(),
    );

    match &result {
        WasmCallResult::Ok(data) => {
            let response: serde_json::Value = serde_json::from_slice(data).unwrap();
            assert_eq!(response["contents"], "hello from spec file");
        }
        WasmCallResult::Trap(t) => panic!("Expected Ok, got trap: {:?}", t),
    }

    let diags = diagnostics.lock().unwrap();
    assert!(diags.is_empty(), "Expected no diagnostics, got {:?}", diags);
}

#[test]
fn wasm_plugin_read_file_denied_outside_spec_root() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let spec_root = tmp.path().join("spec");
    std::fs::create_dir_all(&spec_root).unwrap();
    let outside = tmp.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), "sensitive data").unwrap();

    let diagnostics: Arc<Mutex<Vec<Diagnostic>>> = Arc::new(Mutex::new(Vec::new()));
    let ctx = HostContext::new(diagnostics.clone())
        .with_spec_root(spec_root.clone());

    let runtime = ExtismRuntime::with_host_context(ctx);
    runtime
        .load_module_as("@test/hello", &fixture_wasm_path(), None)
        .unwrap();

    let request = serde_json::json!({
        "path": outside.join("secret.txt").to_string_lossy()
    });
    let result = runtime.call_export(
        "@test/hello",
        "test_read_file",
        &serde_json::to_vec(&request).unwrap(),
    );

    match &result {
        WasmCallResult::Ok(data) => {
            let response: serde_json::Value = serde_json::from_slice(data).unwrap();
            assert!(
                response.get("error").is_some(),
                "Expected error response, got: {}",
                response
            );
        }
        WasmCallResult::Trap(t) => panic!("Expected Ok with error payload, got trap: {:?}", t),
    }

    let diags = diagnostics.lock().unwrap();
    assert_eq!(diags.len(), 1, "Expected 1 sandbox violation diagnostic");
    assert_eq!(diags[0].code, "E031");
    assert!(diags[0].message.contains("not under spec_root"));
}

#[test]
fn wasm_plugin_can_query_graph() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }

    let graph = serde_json::json!({
        "entities": [
            {"id": "b1", "kind": "behavior", "name": "login"},
            {"id": "e1", "kind": "event", "name": "user_logged_in"}
        ],
        "edges": [
            {"source": "b1", "target": "e1", "label": "emits"}
        ]
    });

    let diagnostics: Arc<Mutex<Vec<Diagnostic>>> = Arc::new(Mutex::new(Vec::new()));
    let ctx = HostContext::new(diagnostics.clone())
        .with_graph(graph.clone());

    let runtime = ExtismRuntime::with_host_context(ctx);
    runtime
        .load_module_as("@test/hello", &fixture_wasm_path(), None)
        .unwrap();

    let result = runtime.call_export("@test/hello", "test_query_graph", b"");

    match &result {
        WasmCallResult::Ok(data) => {
            let response: serde_json::Value = serde_json::from_slice(data).unwrap();
            let entities = response["entities"].as_array().unwrap();
            assert_eq!(entities.len(), 2);
            let edges = response["edges"].as_array().unwrap();
            assert_eq!(edges.len(), 1);
        }
        WasmCallResult::Trap(t) => panic!("Expected Ok, got trap: {:?}", t),
    }
}

#[test]
fn multiple_extensions_share_same_host_context() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }

    let diagnostics: Arc<Mutex<Vec<Diagnostic>>> = Arc::new(Mutex::new(Vec::new()));
    let ctx = HostContext::new(diagnostics.clone());

    let runtime = ExtismRuntime::with_host_context(ctx);
    runtime
        .load_module_as("@test/ext-a", &fixture_wasm_path(), None)
        .unwrap();
    runtime
        .load_module_as("@test/ext-b", &fixture_wasm_path(), None)
        .unwrap();

    runtime.call_export("@test/ext-a", "test_emit_diagnostic", b"");
    runtime.call_export("@test/ext-b", "test_emit_diagnostic", b"");

    let diags = diagnostics.lock().unwrap();
    assert_eq!(
        diags.len(),
        2,
        "Both extensions should write to the same diagnostics collector, got {}",
        diags.len()
    );
}
