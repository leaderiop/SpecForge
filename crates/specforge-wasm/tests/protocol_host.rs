use specforge_wasm::{
    protocol::*,
    WasmCallResult, WasmRuntime, WasmTrapInfo,
};
use std::path::Path;

// ── Mock Runtime for protocol tests ──
// Keys on "export_name" for __handshake, "export_name::category" for __describe.

struct MockRuntime {
    call_results: std::collections::HashMap<String, WasmCallResult>,
}

impl MockRuntime {
    fn new() -> Self {
        Self {
            call_results: std::collections::HashMap::new(),
        }
    }

    fn with_call_ok(mut self, key: &str, output: Vec<u8>) -> Self {
        self.call_results
            .insert(key.to_string(), WasmCallResult::Ok(output));
        self
    }

    fn with_call_trap(mut self, key: &str, trap: WasmTrapInfo) -> Self {
        self.call_results
            .insert(key.to_string(), WasmCallResult::Trap(trap));
        self
    }
}

impl WasmRuntime for MockRuntime {
    fn load_module(&self, _wasm_path: &Path, _aot_cache_path: Option<&Path>) -> Result<(), String> {
        Ok(())
    }

    fn call_export(&self, _extension_name: &str, export_name: &str, input: &[u8]) -> WasmCallResult {
        // For __describe, extract category from the input JSON to build a compound key
        if export_name == "__describe"
            && let Ok(req) = serde_json::from_slice::<DescribeRequest>(input) {
                let compound_key = format!("__describe::{}", req.category);
                if let Some(result) = self.call_results.get(&compound_key) {
                    return result.clone();
                }
            }
        // Fallback: look up by export name alone
        self.call_results
            .get(export_name)
            .cloned()
            .unwrap_or_else(|| {
                // Default: return an empty describe response
                let default_resp = serde_json::json!({"category": "unknown", "items": []});
                WasmCallResult::Ok(serde_json::to_vec(&default_resp).unwrap())
            })
    }

    fn has_cached_module(&self, _wasm_hash: &str) -> bool {
        false
    }
}

// ── Helper: build a valid handshake response JSON ──

fn handshake_response_json(name: &str, entities: bool, validators: bool) -> Vec<u8> {
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
    serde_json::to_vec(&resp).unwrap()
}

fn describe_response_json(category: &str, items_json: &str) -> Vec<u8> {
    let resp = serde_json::json!({
        "category": category,
        "items": serde_json::from_str::<serde_json::Value>(items_json).unwrap()
    });
    serde_json::to_vec(&resp).unwrap()
}

// ── Step 1: ProtocolHost::handshake tracer bullet ──

#[test]
fn handshake_returns_parsed_response() {
    let runtime = MockRuntime::new().with_call_ok(
        "__handshake",
        handshake_response_json("@specforge/software", true, true),
    );
    let host = ProtocolHost::new(&runtime);
    let resp = host.handshake("@specforge/software").unwrap();
    assert_eq!(resp.name, "@specforge/software");
    assert_eq!(resp.version, "1.0.0");
    assert_eq!(resp.protocol_version, "1.0.0");
    assert!(resp.contribution_flags.entities);
    assert!(resp.contribution_flags.validators);
}

// ── Step 2: Handshake error handling ──

#[test]
fn handshake_trap_returns_error() {
    let runtime = MockRuntime::new().with_call_trap(
        "__handshake",
        WasmTrapInfo {
            kind: "unreachable".to_string(),
            message: "module crashed".to_string(),
            export_name: "__handshake".to_string(),
        },
    );
    let host = ProtocolHost::new(&runtime);
    let err = host.handshake("@test/ext").unwrap_err();
    match err {
        ProtocolError::HandshakeFailed(msg) => {
            assert!(msg.contains("unreachable"));
            assert!(msg.contains("module crashed"));
        }
        other => panic!("expected HandshakeFailed, got {:?}", other),
    }
}

#[test]
fn handshake_invalid_json_returns_deserialization_error() {
    let runtime = MockRuntime::new().with_call_ok(
        "__handshake",
        b"not valid json at all".to_vec(),
    );
    let host = ProtocolHost::new(&runtime);
    let err = host.handshake("@test/ext").unwrap_err();
    match err {
        ProtocolError::DeserializationError(_) => {}
        other => panic!("expected DeserializationError, got {:?}", other),
    }
}

// ── Step 3: validate_protocol_version ──

#[test]
fn validate_protocol_version_compatible() {
    let runtime = MockRuntime::new();
    let host = ProtocolHost::new(&runtime);
    let resp = HandshakeResponse {
        protocol_version: "1.0.0".to_string(),
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        contribution_flags: ContributionFlags::default(),
        peer_dependencies: vec![],
        sandbox_policy: None,
    };
    assert!(host.validate_protocol_version(&resp).is_ok());
}

#[test]
fn validate_protocol_version_incompatible() {
    let runtime = MockRuntime::new();
    let host = ProtocolHost::new(&runtime);
    let resp = HandshakeResponse {
        protocol_version: "2.0.0".to_string(),
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        contribution_flags: ContributionFlags::default(),
        peer_dependencies: vec![],
        sandbox_policy: None,
    };
    let err = host.validate_protocol_version(&resp).unwrap_err();
    match err {
        ProtocolError::IncompatibleVersion {
            host_version,
            extension_version,
        } => {
            assert_eq!(host_version, "1.0.0");
            assert_eq!(extension_version, "2.0.0");
        }
        other => panic!("expected IncompatibleVersion, got {:?}", other),
    }
}

// ── Step 4: ProtocolHost::describe ──

#[test]
fn describe_returns_parsed_response() {
    let runtime = MockRuntime::new().with_call_ok(
        "__describe",
        describe_response_json("entities", r#"[{"name": "behavior", "testable": true}]"#),
    );
    let host = ProtocolHost::new(&runtime);
    let resp = host.describe("@test/ext", "entities").unwrap();
    assert_eq!(resp.category, "entities");
    let entities: Vec<EntityKindDescriptor> = resp.parse_items().unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].name, "behavior");
}

// ── Step 5: Describe error handling ──

#[test]
fn describe_trap_returns_error() {
    let runtime = MockRuntime::new().with_call_trap(
        "__describe",
        WasmTrapInfo {
            kind: "trap".to_string(),
            message: "out of memory".to_string(),
            export_name: "__describe".to_string(),
        },
    );
    let host = ProtocolHost::new(&runtime);
    let err = host.describe("@test/ext", "entities").unwrap_err();
    match err {
        ProtocolError::DescribeFailed { category, reason } => {
            assert_eq!(category, "entities");
            assert!(reason.contains("out of memory"));
        }
        other => panic!("expected DescribeFailed, got {:?}", other),
    }
}

#[test]
fn describe_unsupported_category_returns_error() {
    let runtime = MockRuntime::new();
    let host = ProtocolHost::new(&runtime);
    let err = host.describe("@test/ext", "widgets").unwrap_err();
    match err {
        ProtocolError::UnsupportedCategory(cat) => assert_eq!(cat, "widgets"),
        other => panic!("expected UnsupportedCategory, got {:?}", other),
    }
}

// ── Step 6: describe_all ──

#[test]
fn describe_all_populates_enabled_categories() {
    let runtime = MockRuntime::new()
        .with_call_ok(
            "__handshake",
            handshake_response_json("@test/ext", true, true),
        )
        .with_call_ok(
            "__describe::entities",
            describe_response_json("entities", r#"[{"name": "behavior", "testable": true}]"#),
        )
        .with_call_ok(
            "__describe::edges",
            describe_response_json("edges", r#"[{"label": "Implements"}]"#),
        )
        .with_call_ok(
            "__describe::validation_rules",
            describe_response_json("validation_rules", r#"[{"code": "W001", "severity": "warning", "message_template": "test", "check": "no_incoming_edges"}]"#),
        );

    let host = ProtocolHost::new(&runtime);
    let flags = ContributionFlags {
        entities: true,
        validators: true,
        ..Default::default()
    };
    let descs = host.describe_all("@test/ext", &flags).unwrap();
    assert_eq!(descs.entity_kinds.len(), 1);
    assert_eq!(descs.entity_kinds[0].name, "behavior");
    assert_eq!(descs.edge_types.len(), 1);
    assert_eq!(descs.edge_types[0].label, "Implements");
    assert_eq!(descs.validation_rules.len(), 1);
    assert_eq!(descs.validation_rules[0].code, "W001");
    // Collectors, grammars etc. not enabled → empty
    assert!(descs.collectors.is_empty());
    assert!(descs.grammars.is_empty());
}

#[test]
fn describe_all_skips_disabled_categories() {
    // With all flags false, describe_all should make no __describe calls
    let runtime = MockRuntime::new().with_call_trap(
        "__describe",
        WasmTrapInfo {
            kind: "trap".to_string(),
            message: "should not be called".to_string(),
            export_name: "__describe".to_string(),
        },
    );
    let host = ProtocolHost::new(&runtime);
    let flags = ContributionFlags::default(); // all false
    let descs = host.describe_all("@test/ext", &flags).unwrap();
    assert!(descs.entity_kinds.is_empty());
    assert!(descs.validation_rules.is_empty());
    assert!(descs.surfaces.is_none());
}

// ── Step 7: validate_peer_dependencies ──

#[test]
fn validate_peer_deps_missing_required() {
    let runtime = MockRuntime::new();
    let host = ProtocolHost::new(&runtime);
    let resp = HandshakeResponse {
        protocol_version: "1.0.0".to_string(),
        name: "@specforge/formal".to_string(),
        version: "1.0.0".to_string(),
        contribution_flags: ContributionFlags::default(),
        peer_dependencies: vec![PeerDependency {
            name: "@specforge/software".to_string(),
            version: ">=1.0.0".to_string(),
            optional: false,
        }],
        sandbox_policy: None,
    };
    let diags = host.validate_peer_dependencies(&resp, &[]);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E028");
    assert!(diags[0].message.contains("@specforge/software"));
}

#[test]
fn validate_peer_deps_optional_missing_no_error() {
    let runtime = MockRuntime::new();
    let host = ProtocolHost::new(&runtime);
    let resp = HandshakeResponse {
        protocol_version: "1.0.0".to_string(),
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        contribution_flags: ContributionFlags::default(),
        peer_dependencies: vec![PeerDependency {
            name: "@specforge/optional".to_string(),
            version: ">=1.0.0".to_string(),
            optional: true,
        }],
        sandbox_policy: None,
    };
    let diags = host.validate_peer_dependencies(&resp, &[]);
    assert!(diags.is_empty());
}

#[test]
fn validate_peer_deps_all_satisfied() {
    let runtime = MockRuntime::new();
    let host = ProtocolHost::new(&runtime);
    let resp = HandshakeResponse {
        protocol_version: "1.0.0".to_string(),
        name: "@specforge/formal".to_string(),
        version: "1.0.0".to_string(),
        contribution_flags: ContributionFlags::default(),
        peer_dependencies: vec![PeerDependency {
            name: "@specforge/software".to_string(),
            version: ">=1.0.0".to_string(),
            optional: false,
        }],
        sandbox_policy: None,
    };
    let diags = host.validate_peer_dependencies(&resp, &["@specforge/software"]);
    assert!(diags.is_empty());
}

// ── Step 8: load_protocol_extension ──

#[test]
fn load_protocol_extension_full_flow() {
    let runtime = MockRuntime::new()
        .with_call_ok(
            "__handshake",
            handshake_response_json("@specforge/governance", true, true),
        )
        .with_call_ok(
            "__describe",
            describe_response_json("entities", "[]"),
        );

    let host = ProtocolHost::new(&runtime);
    let ext = load_protocol_extension(&host, "@specforge/governance").unwrap();
    assert_eq!(ext.name, "@specforge/governance");
    assert_eq!(ext.version, "1.0.0");
    assert_eq!(ext.handshake.protocol_version, "1.0.0");
}

#[test]
fn load_protocol_extension_handshake_failure_propagated() {
    let runtime = MockRuntime::new().with_call_trap(
        "__handshake",
        WasmTrapInfo {
            kind: "trap".to_string(),
            message: "extension panicked".to_string(),
            export_name: "__handshake".to_string(),
        },
    );
    let host = ProtocolHost::new(&runtime);
    let err = load_protocol_extension(&host, "@test/ext").unwrap_err();
    match err {
        ProtocolError::HandshakeFailed(_) => {}
        other => panic!("expected HandshakeFailed, got {:?}", other),
    }
}

// ── M11: semver-compatible protocol version matching ──

#[test]
fn validate_protocol_version_compatible_patch_bump() {
    let runtime = MockRuntime::new();
    let host = ProtocolHost::new(&runtime);
    // Extension reports "1.0.1" — same major as host "1.0.0" => compatible
    let resp = HandshakeResponse {
        protocol_version: "1.0.1".to_string(),
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        contribution_flags: ContributionFlags::default(),
        peer_dependencies: vec![],
        sandbox_policy: None,
    };
    assert!(
        host.validate_protocol_version(&resp).is_ok(),
        "patch version bump should be compatible"
    );
}

#[test]
fn validate_protocol_version_compatible_minor_bump() {
    let runtime = MockRuntime::new();
    let host = ProtocolHost::new(&runtime);
    // Extension reports "1.1.0" — same major as host "1.0.0" => compatible
    let resp = HandshakeResponse {
        protocol_version: "1.1.0".to_string(),
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        contribution_flags: ContributionFlags::default(),
        peer_dependencies: vec![],
        sandbox_policy: None,
    };
    assert!(
        host.validate_protocol_version(&resp).is_ok(),
        "minor version bump should be compatible"
    );
}

#[test]
fn validate_protocol_version_incompatible_major_bump() {
    let runtime = MockRuntime::new();
    let host = ProtocolHost::new(&runtime);
    // Extension reports "2.0.0" — different major => incompatible
    let resp = HandshakeResponse {
        protocol_version: "2.0.0".to_string(),
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        contribution_flags: ContributionFlags::default(),
        peer_dependencies: vec![],
        sandbox_policy: None,
    };
    let err = host.validate_protocol_version(&resp).unwrap_err();
    match err {
        ProtocolError::IncompatibleVersion {
            host_version,
            extension_version,
        } => {
            assert!(host_version.starts_with("1."));
            assert_eq!(extension_version, "2.0.0");
        }
        other => panic!("expected IncompatibleVersion, got {:?}", other),
    }
}

#[test]
fn validate_protocol_version_exact_match_still_works() {
    let runtime = MockRuntime::new();
    let host = ProtocolHost::new(&runtime);
    // Same version as PROTOCOL_VERSION => compatible
    let resp = HandshakeResponse {
        protocol_version: PROTOCOL_VERSION.to_string(),
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        contribution_flags: ContributionFlags::default(),
        peer_dependencies: vec![],
        sandbox_policy: None,
    };
    assert!(host.validate_protocol_version(&resp).is_ok());
}

#[test]
fn load_protocol_extension_version_mismatch_propagated() {
    let mut resp = HandshakeResponse {
        protocol_version: "99.0".to_string(),
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        contribution_flags: ContributionFlags::default(),
        peer_dependencies: vec![],
        sandbox_policy: None,
    };
    // Patch protocol_version to something incompatible
    resp.protocol_version = "99.0".to_string();
    let runtime = MockRuntime::new().with_call_ok(
        "__handshake",
        serde_json::to_vec(&resp).unwrap(),
    );
    let host = ProtocolHost::new(&runtime);
    let err = load_protocol_extension(&host, "@test/ext").unwrap_err();
    match err {
        ProtocolError::IncompatibleVersion { .. } => {}
        other => panic!("expected IncompatibleVersion, got {:?}", other),
    }
}
