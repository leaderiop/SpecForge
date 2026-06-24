//! Protocol extension loading: extensions are loaded via the Wasm protocol
//! (__handshake / __describe) through a WasmRuntime implementation.

use specforge_test::prelude::*;
use specforge_wasm::{WasmCallResult, WasmRuntime, WasmTrapInfo};
use specforge_wasm::protocol::*;
use tempfile::TempDir;
use std::collections::HashMap;
use std::path::Path;
use std::fs;

/// Helper: create a temp project dir with specforge.json and optional extensions.
fn setup_project(extensions: &[&str], spec_content: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let ext_json: Vec<String> = extensions.iter().map(|e| format!("\"{}\"", e)).collect();
    let config = format!(
        r#"{{"name":"test","version":"0.1.0","extensions":[{}]}}"#,
        ext_json.join(",")
    );
    fs::write(dir.path().join("specforge.json"), config).unwrap();
    fs::write(dir.path().join("core.spec"), spec_content).unwrap();
    dir
}

// (Removed: manifest loading tests — manifest.json is no longer supported.
// Extensions are loaded via the BuiltinRuntime protocol pipeline.)

// ── MockRuntime ──
// Category-aware mock: keys on "__handshake" for handshake, "__describe::{category}" for describe.

struct MockRuntime {
    call_results: HashMap<String, WasmCallResult>,
}

impl MockRuntime {
    fn new() -> Self {
        Self {
            call_results: HashMap::new(),
        }
    }

    fn with_handshake(mut self, name: &str, entities: bool, validators: bool) -> Self {
        let resp = HandshakeResponse {
            protocol_version: PROTOCOL_VERSION.to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            contribution_flags: ContributionFlags {
                entities,
                validators,
                ..Default::default()
            },
            peer_dependencies: vec![],
            sandbox_policy: None,
        };
        self.call_results.insert(
            "__handshake".to_string(),
            WasmCallResult::Ok(serde_json::to_vec(&resp).unwrap()),
        );
        self
    }

    fn with_describe(mut self, category: &str, items: serde_json::Value) -> Self {
        let resp = DescribeResponse {
            category: category.to_string(),
            items,
        };
        self.call_results.insert(
            format!("__describe::{}", category),
            WasmCallResult::Ok(serde_json::to_vec(&resp).unwrap()),
        );
        self
    }

    fn with_call_trap(mut self, key: &str, trap: WasmTrapInfo) -> Self {
        self.call_results.insert(key.to_string(), WasmCallResult::Trap(trap));
        self
    }
}

impl WasmRuntime for MockRuntime {
    fn load_module(&self, _wasm_path: &Path, _aot_cache_path: Option<&Path>) -> Result<(), String> {
        Ok(())
    }

    fn call_export(&self, _extension_name: &str, export_name: &str, input: &[u8]) -> WasmCallResult {
        if export_name == "__describe"
            && let Ok(req) = serde_json::from_slice::<DescribeRequest>(input) {
                let compound_key = format!("__describe::{}", req.category);
                if let Some(result) = self.call_results.get(&compound_key) {
                    return result.clone();
                }
            }
        self.call_results
            .get(export_name)
            .cloned()
            .unwrap_or_else(|| {
                let default_resp = serde_json::json!({"category": "unknown", "items": []});
                WasmCallResult::Ok(serde_json::to_vec(&default_resp).unwrap())
            })
    }

    fn has_cached_module(&self, _wasm_hash: &str) -> bool {
        false
    }
}

// --- Step 6: Protocol extension loaded with MockRuntime ---

// B:dual_mode_loading — verify unit "protocol extension loaded via runtime"
#[test]
#[specforge_test(behavior = "dual_mode_loading", verify = "protocol extension loaded via runtime")]
fn protocol_extension_loaded_with_runtime() {
    let dir = setup_project(&["./ext-proto"], "behavior hello \"Hello\" {\n    status planned\n}\n");

    let ext_dir = dir.path().join("ext-proto");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(ext_dir.join("extension.wasm"), [0x00, 0x61, 0x73, 0x6d]).unwrap();

    let runtime = MockRuntime::new()
        .with_handshake("@test/proto", true, false)
        .with_describe("entities", serde_json::json!([{
            "name": "gadget",
            "description": "A test gadget"
        }]))
        .with_describe("edges", serde_json::json!([]))
        .with_describe("fields", serde_json::json!([]))
        .with_describe("shared_fields", serde_json::json!([]))
        .with_describe("enhancements", serde_json::json!([]))
        .with_describe("surfaces", serde_json::json!([]))
        .with_describe("passes", serde_json::json!([]))
        .with_describe("feature_flags", serde_json::json!([]));

    let ctx = specforge_emitter::compile_with_runtime(dir.path(), Some(&runtime));

    // No W031 — runtime was provided
    let w031: Vec<_> = ctx.diagnostics.iter().filter(|d| d.code == "W031").collect();
    assert!(w031.is_empty(), "should not have W031 when runtime is provided: {:?}", w031);

    // No E031 — protocol loading should succeed
    let e031: Vec<_> = ctx.diagnostics.iter().filter(|d| d.code == "E031").collect();
    assert!(e031.is_empty(), "should not have E031: {:?}", e031);

    // ManifestV2 should appear in ctx.manifests
    assert_eq!(ctx.manifests.len(), 1, "expected 1 manifest from protocol extension");
    assert_eq!(ctx.manifests[0].name, "@test/proto");

    // KindRegistry should have "gadget"
    assert!(ctx.kind_registry.contains("gadget"), "expected gadget kind in registry");
}

// --- Step 7: Mixed manifest + protocol extensions ---
// (Removed: dual-mode coexistence is no longer supported. When a runtime is provided,
// all extensions are loaded via the protocol path. Manifest-only extensions are being removed.)

// --- Step 8: Error handling — protocol failures become diagnostics ---

// B:dual_mode_loading — verify unit "protocol handshake trap produces E031 diagnostic"
#[test]
#[specforge_test(behavior = "dual_mode_loading", verify = "protocol handshake trap produces E031 diagnostic")]
fn protocol_handshake_trap_produces_e031() {
    let dir = setup_project(&["./ext-broken"], "behavior hello \"Hello\" {\n    status planned\n}\n");

    let ext_dir = dir.path().join("ext-broken");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(ext_dir.join("extension.wasm"), [0x00, 0x61, 0x73, 0x6d]).unwrap();

    let runtime = MockRuntime::new()
        .with_call_trap("__handshake", WasmTrapInfo {
            kind: "unreachable".to_string(),
            message: "extension panicked during handshake".to_string(),
            export_name: "__handshake".to_string(),
        });

    let ctx = specforge_emitter::compile_with_runtime(dir.path(), Some(&runtime));

    // E031 diagnostic should be emitted
    let e031: Vec<_> = ctx.diagnostics.iter().filter(|d| d.code == "E031").collect();
    assert_eq!(e031.len(), 1, "expected exactly 1 E031 diagnostic, got: {:?}", e031);
    assert!(e031[0].message.contains("ext-broken"), "E031 should mention extension name");
    assert!(e031[0].message.contains("protocol loading failed"), "E031 should describe the error");

    // No manifests from the broken extension
    assert!(ctx.manifests.is_empty(), "broken extension should not produce a manifest");
}

// B:dual_mode_loading — verify unit "protocol version mismatch produces E031"
#[test]
#[specforge_test(behavior = "dual_mode_loading", verify = "protocol version mismatch produces E031")]
fn protocol_version_mismatch_produces_e031() {
    let dir = setup_project(&["./ext-badver"], "behavior hello \"Hello\" {\n    status planned\n}\n");

    let ext_dir = dir.path().join("ext-badver");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(ext_dir.join("extension.wasm"), [0x00, 0x61, 0x73, 0x6d]).unwrap();

    // Return a handshake with wrong protocol version
    let bad_handshake = HandshakeResponse {
        protocol_version: "99.0".to_string(),
        name: "@test/badver".to_string(),
        version: "1.0.0".to_string(),
        contribution_flags: ContributionFlags::default(),
        peer_dependencies: vec![],
        sandbox_policy: None,
    };
    let runtime = MockRuntime {
        call_results: {
            let mut m = HashMap::new();
            m.insert(
                "__handshake".to_string(),
                WasmCallResult::Ok(serde_json::to_vec(&bad_handshake).unwrap()),
            );
            m
        },
    };

    let ctx = specforge_emitter::compile_with_runtime(dir.path(), Some(&runtime));

    let e031: Vec<_> = ctx.diagnostics.iter().filter(|d| d.code == "E031").collect();
    assert_eq!(e031.len(), 1, "expected exactly 1 E031 for version mismatch");
    assert!(e031[0].message.contains("protocol loading failed"), "E031 should describe error");
    assert!(e031[0].message.contains("version mismatch"), "E031 should mention version mismatch");
}

// (Removed: "protocol error does not prevent manifest extensions from loading" tested dual-mode
// coexistence which is no longer supported. Error isolation for protocol extensions is covered
// by protocol_handshake_trap_produces_e031.)
