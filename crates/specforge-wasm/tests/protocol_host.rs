use specforge_wasm::{WasmCallResult, WasmRuntime, WasmTrapInfo, protocol::*};
use std::path::Path;

// ── Mock Runtime for protocol tests ──
// Keys on "export_name" for __handshake, "export_name::category" for __describe.

struct MockRuntime {
    call_results: std::collections::HashMap<String, WasmCallResult>,
    /// Records `set_execution_deadline_ms` calls (extension, ms).
    deadlines: std::sync::Mutex<Vec<(String, u64)>>,
}

impl MockRuntime {
    fn new() -> Self {
        Self {
            call_results: std::collections::HashMap::new(),
            deadlines: std::sync::Mutex::new(Vec::new()),
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
    fn load_module(&self, _wasm_path: &Path) -> Result<(), String> {
        Ok(())
    }

    fn call_export(
        &self,
        _extension_name: &str,
        export_name: &str,
        input: &[u8],
    ) -> WasmCallResult {
        // For __describe, extract category from the input JSON to build a compound key
        if export_name == "__describe"
            && let Ok(req) = serde_json::from_slice::<DescribeRequest>(input)
        {
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

    fn set_execution_deadline_ms(&self, extension_name: &str, max_execution_ms: u64) {
        self.deadlines
            .lock()
            .unwrap()
            .push((extension_name.to_string(), max_execution_ms));
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
        starter_template: None,
        theme_color: None,
        migration_hook: None,
        ..Default::default()
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

// The transport under `load_declaration`: the handshake, its protocol
// version check and its execution budget, then every describe category.

fn handshake_with(protocol_version: &str) -> Vec<u8> {
    serde_json::to_vec(&HandshakeResponse {
        protocol_version: protocol_version.to_string(),
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        ..Default::default()
    })
    .unwrap()
}

#[test]
fn handshake_returns_parsed_response() {
    let runtime = MockRuntime::new().with_call_ok(
        "__handshake",
        handshake_response_json("@specforge/software", true, true),
    );
    let loaded = load_declaration(&runtime, "@specforge/software").unwrap();
    let resp = &loaded.declaration.handshake;
    assert_eq!(resp.name, "@specforge/software");
    assert_eq!(resp.version, "1.0.0");
    assert_eq!(resp.protocol_version, "1.0.0");
    assert!(resp.contribution_flags.entities);
    assert!(resp.contribution_flags.validators);
}

// ── Handshake applies the plugin's declared execution budget (C7-10) ──

#[test]
fn handshake_applies_declared_max_execution_ms() {
    let policy = SandboxPolicy {
        max_execution_ms: Some(5000),
        ..Default::default()
    };
    let resp = HandshakeResponse {
        protocol_version: PROTOCOL_VERSION.to_string(),
        name: "@specforge/software".to_string(),
        version: "1.0.0".to_string(),
        sandbox_policy: Some(policy),
        ..Default::default()
    };
    let runtime =
        MockRuntime::new().with_call_ok("__handshake", serde_json::to_vec(&resp).unwrap());
    load_declaration(&runtime, "@specforge/software").unwrap();

    assert_eq!(
        *runtime.deadlines.lock().unwrap_or_else(|p| p.into_inner()),
        vec![("@specforge/software".to_string(), 5000)]
    );
}

#[test]
fn handshake_without_execution_budget_sets_no_deadline() {
    let runtime = MockRuntime::new().with_call_ok(
        "__handshake",
        handshake_response_json("@specforge/formal", true, false),
    );
    load_declaration(&runtime, "@specforge/formal").unwrap();
    assert!(
        runtime
            .deadlines
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .is_empty()
    );
}

// ── Handshake error handling ──

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
    let err = load_declaration(&runtime, "@test/ext").unwrap_err();
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
    let runtime = MockRuntime::new().with_call_ok("__handshake", b"not valid json at all".to_vec());
    let err = load_declaration(&runtime, "@test/ext").unwrap_err();
    match err {
        ProtocolError::DeserializationError(_) => {}
        other => panic!("expected DeserializationError, got {:?}", other),
    }
}

// ── The protocol version: same major is compatible (M11) ──

#[test]
fn compatible_protocol_versions_load() {
    for version in ["1.0.0", "1.0.1", "1.1.0", "1.9.3"] {
        let runtime = MockRuntime::new().with_call_ok("__handshake", handshake_with(version));
        assert!(
            load_declaration(&runtime, "@test/ext").is_ok(),
            "{version} is compatible with the host's 1.0.0"
        );
    }
}

#[test]
fn an_incompatible_protocol_version_fails_the_load() {
    for version in ["2.0.0", "0.9.0", "99.0"] {
        let runtime = MockRuntime::new().with_call_ok("__handshake", handshake_with(version));
        match load_declaration(&runtime, "@test/ext").unwrap_err() {
            ProtocolError::IncompatibleVersion {
                host_version,
                extension_version,
            } => {
                assert_eq!(host_version, "1.0.0");
                assert_eq!(extension_version, version);
            }
            other => panic!("{version}: expected IncompatibleVersion, got {:?}", other),
        }
    }
}

// ── Describe ──

#[test]
fn describe_answers_become_the_declaration() {
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
    let declaration = load_declaration(&runtime, "@test/ext").unwrap().declaration;
    assert_eq!(declaration.entities.len(), 1);
    assert_eq!(declaration.entities[0].name, "behavior");
    assert!(declaration.entities[0].testable);
    assert_eq!(declaration.edges[0].label, "Implements");
    assert_eq!(declaration.validation_rules[0].code, "W001");
    assert!(declaration.collectors.is_empty());
}

#[test]
fn describe_trap_returns_error() {
    let runtime = MockRuntime::new()
        .with_call_ok(
            "__handshake",
            handshake_response_json("@test/ext", true, false),
        )
        .with_call_trap(
            "__describe",
            WasmTrapInfo {
                kind: "trap".to_string(),
                message: "out of memory".to_string(),
                export_name: "__describe".to_string(),
            },
        );
    let err = load_declaration(&runtime, "@test/ext").unwrap_err();
    match err {
        ProtocolError::DescribeFailed { category, reason } => {
            assert_eq!(category, "entities");
            assert!(reason.contains("out of memory"));
        }
        other => panic!("expected DescribeFailed, got {:?}", other),
    }
}

/// An answer that is not a describe response fails naming its category.
#[test]
fn a_describe_answer_that_is_not_json_names_its_category() {
    let runtime = MockRuntime::new()
        .with_call_ok(
            "__handshake",
            handshake_response_json("@test/ext", true, false),
        )
        .with_call_ok("__describe::edges", b"not json".to_vec());
    match load_declaration(&runtime, "@test/ext").unwrap_err() {
        ProtocolError::DescribeFailed { category, .. } => assert_eq!(category, "edges"),
        other => panic!("expected DescribeFailed, got {:?}", other),
    }
}

/// Load an extension whose surfaces declare an arg type outside
/// CommandArgType ("list"), which cannot be represented.
fn load_with_unknown_arg_type() -> Result<Loaded, ProtocolError> {
    let surfaces = r#"[{"commands": [{"id": "c", "title": "C", "description": "d",
        "export": "cmd__c", "args": [{"name": "tags", "arg_type": "list"}]}]}]"#;
    let runtime = MockRuntime::new()
        .with_call_ok(
            "__handshake",
            handshake_response_json("@test/ext", true, false),
        )
        .with_call_ok(
            "__describe::surfaces",
            describe_response_json("surfaces", surfaces),
        );
    load_declaration(&runtime, "@test/ext")
}

#[specforge_test_macros::test(
    behavior = "register_surface_contributions",
    verify = "a surfaces description that does not parse fails the extension's load"
)]
fn a_surfaces_description_that_does_not_parse_fails_the_load() {
    let err = load_with_unknown_arg_type().unwrap_err();
    assert!(format!("{err}").contains("surfaces"), "{err}");
}

#[specforge_test_macros::test(
    invariant = "surface_schema_validity",
    verify = "a surfaces description with an unknown arg type fails the extension's load"
)]
fn an_unknown_arg_type_fails_the_load() {
    assert!(load_with_unknown_arg_type().is_err());
}
