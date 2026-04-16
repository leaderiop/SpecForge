use std::collections::HashMap;
use std::path::Path;

use crate::protocol::{DescribeRequest, DescribeResponse, HandshakeResponse};
use crate::runtime::{WasmCallResult, WasmRuntime, WasmTrapInfo};

/// Trait for extensions implemented as native Rust instead of Wasm modules.
///
/// Each built-in extension returns typed protocol responses. The `BuiltinRuntime`
/// serializes them to JSON (same wire format a real Wasm extension would produce),
/// ensuring full round-trip validation of the protocol.
pub trait BuiltinExtension: Send + Sync {
    fn handshake(&self) -> HandshakeResponse;
    fn describe(&self, category: &str) -> Option<DescribeResponse>;
}

/// A `WasmRuntime` backed by native Rust extension implementations.
///
/// Dispatches `call_export()` to registered `BuiltinExtension` instances by
/// extension name. `load_module()` and `has_cached_module()` are no-ops since
/// the extensions are compiled into the binary.
pub struct BuiltinRuntime {
    extensions: HashMap<String, Box<dyn BuiltinExtension>>,
}

impl BuiltinRuntime {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
        }
    }

    pub fn with_extension(mut self, name: &str, ext: Box<dyn BuiltinExtension>) -> Self {
        self.extensions.insert(name.to_string(), ext);
        self
    }
}

impl Default for BuiltinRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmRuntime for BuiltinRuntime {
    fn load_module(&self, _wasm_path: &Path, _aot_cache_path: Option<&Path>) -> Result<(), String> {
        Ok(())
    }

    fn call_export(&self, extension_name: &str, export_name: &str, input: &[u8]) -> WasmCallResult {
        let ext = match self.extensions.get(extension_name) {
            Some(e) => e,
            None => {
                return WasmCallResult::Trap(WasmTrapInfo {
                    kind: "not_found".to_string(),
                    message: format!("no builtin extension registered for '{}'", extension_name),
                    export_name: export_name.to_string(),
                });
            }
        };

        match export_name {
            "__handshake" => {
                let response = ext.handshake();
                match serde_json::to_vec(&response) {
                    Ok(bytes) => WasmCallResult::Ok(bytes),
                    Err(e) => WasmCallResult::Trap(WasmTrapInfo {
                        kind: "serialization_error".to_string(),
                        message: format!("failed to serialize handshake response: {}", e),
                        export_name: export_name.to_string(),
                    }),
                }
            }
            "__describe" => {
                let request: DescribeRequest = match serde_json::from_slice(input) {
                    Ok(r) => r,
                    Err(e) => {
                        return WasmCallResult::Trap(WasmTrapInfo {
                            kind: "deserialization_error".to_string(),
                            message: format!("failed to deserialize describe request: {}", e),
                            export_name: export_name.to_string(),
                        });
                    }
                };

                match ext.describe(&request.category) {
                    Some(response) => match serde_json::to_vec(&response) {
                        Ok(bytes) => WasmCallResult::Ok(bytes),
                        Err(e) => WasmCallResult::Trap(WasmTrapInfo {
                            kind: "serialization_error".to_string(),
                            message: format!("failed to serialize describe response: {}", e),
                            export_name: export_name.to_string(),
                        }),
                    },
                    None => WasmCallResult::Trap(WasmTrapInfo {
                        kind: "unsupported_category".to_string(),
                        message: format!("extension does not support category '{}'", request.category),
                        export_name: export_name.to_string(),
                    }),
                }
            }
            _ => WasmCallResult::Trap(WasmTrapInfo {
                kind: "unknown_export".to_string(),
                message: format!("unknown export '{}'", export_name),
                export_name: export_name.to_string(),
            }),
        }
    }

    fn has_cached_module(&self, _wasm_hash: &str) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ContributionFlags, EntityKindDescriptor, FieldDescriptor};

    struct TestExtension;

    impl BuiltinExtension for TestExtension {
        fn handshake(&self) -> HandshakeResponse {
            HandshakeResponse {
                protocol_version: "1.0".to_string(),
                name: "@test/ext".to_string(),
                version: "0.1.0".to_string(),
                contribution_flags: ContributionFlags {
                    entities: true,
                    ..Default::default()
                },
                peer_dependencies: vec![],
                sandbox_policy: None,
            }
        }

        fn describe(&self, category: &str) -> Option<DescribeResponse> {
            match category {
                "entities" => Some(DescribeResponse {
                    category: "entities".to_string(),
                    items: serde_json::to_value(vec![EntityKindDescriptor {
                        name: "widget".to_string(),
                        keyword: Some("widget".to_string()),
                        fields: vec![FieldDescriptor {
                            name: "label".to_string(),
                            field_type: "string".to_string(),
                            required: true,
                            ..Default::default()
                        }],
                        ..Default::default()
                    }])
                    .unwrap(),
                }),
                _ => None,
            }
        }
    }

    impl Default for FieldDescriptor {
        fn default() -> Self {
            Self {
                name: String::new(),
                field_type: String::new(),
                required: false,
                description: None,
                edge: None,
                target_kind: None,
                file_reference: false,
                default_value: None,
                enum_values: vec![],
            }
        }
    }

    impl Default for EntityKindDescriptor {
        fn default() -> Self {
            Self {
                name: String::new(),
                keyword: None,
                description: None,
                fields: vec![],
                testable: false,
                singleton: false,
                supports_verify: false,
                incremental: None,
                has_body_parser: false,
                open_fields: false,
                semantic_token: None,
                lsp_icon: None,
                dot_shape: None,
                dot_color: None,
                dot_fillcolor: None,
                verify_kinds: vec![],
            }
        }
    }

    #[test]
    fn handshake_returns_correct_response() {
        let runtime = BuiltinRuntime::new()
            .with_extension("@test/ext", Box::new(TestExtension));

        let input = serde_json::to_vec(&serde_json::json!({
            "host_version": "1.0",
            "supported_categories": ["entities"]
        }))
        .unwrap();

        let result = runtime.call_export("@test/ext", "__handshake", &input);
        match result {
            WasmCallResult::Ok(bytes) => {
                let response: HandshakeResponse = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(response.protocol_version, "1.0");
                assert_eq!(response.name, "@test/ext");
                assert_eq!(response.version, "0.1.0");
                assert!(response.contribution_flags.entities);
                assert!(!response.contribution_flags.validators);
            }
            WasmCallResult::Trap(t) => panic!("expected Ok, got Trap: {:?}", t),
        }
    }

    #[test]
    fn describe_returns_entities() {
        let runtime = BuiltinRuntime::new()
            .with_extension("@test/ext", Box::new(TestExtension));

        let input = serde_json::to_vec(&serde_json::json!({ "category": "entities" })).unwrap();

        let result = runtime.call_export("@test/ext", "__describe", &input);
        match result {
            WasmCallResult::Ok(bytes) => {
                let response: DescribeResponse = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(response.category, "entities");
                let kinds: Vec<EntityKindDescriptor> = response.parse_items().unwrap();
                assert_eq!(kinds.len(), 1);
                assert_eq!(kinds[0].name, "widget");
                assert_eq!(kinds[0].fields.len(), 1);
                assert_eq!(kinds[0].fields[0].name, "label");
                assert!(kinds[0].fields[0].required);
            }
            WasmCallResult::Trap(t) => panic!("expected Ok, got Trap: {:?}", t),
        }
    }

    #[test]
    fn describe_unsupported_category_returns_trap() {
        let runtime = BuiltinRuntime::new()
            .with_extension("@test/ext", Box::new(TestExtension));

        let input = serde_json::to_vec(&serde_json::json!({ "category": "surfaces" })).unwrap();

        let result = runtime.call_export("@test/ext", "__describe", &input);
        match result {
            WasmCallResult::Trap(t) => {
                assert_eq!(t.kind, "unsupported_category");
                assert!(t.message.contains("surfaces"));
            }
            WasmCallResult::Ok(_) => panic!("expected Trap for unsupported category"),
        }
    }

    #[test]
    fn unknown_extension_returns_trap() {
        let runtime = BuiltinRuntime::new();

        let result = runtime.call_export("@nonexistent/ext", "__handshake", &[]);
        match result {
            WasmCallResult::Trap(t) => {
                assert_eq!(t.kind, "not_found");
                assert!(t.message.contains("@nonexistent/ext"));
            }
            WasmCallResult::Ok(_) => panic!("expected Trap for unknown extension"),
        }
    }

    #[test]
    fn unknown_export_returns_trap() {
        let runtime = BuiltinRuntime::new()
            .with_extension("@test/ext", Box::new(TestExtension));

        let result = runtime.call_export("@test/ext", "initialize", &[]);
        match result {
            WasmCallResult::Trap(t) => {
                assert_eq!(t.kind, "unknown_export");
                assert!(t.message.contains("initialize"));
            }
            WasmCallResult::Ok(_) => panic!("expected Trap for unknown export"),
        }
    }

    #[test]
    fn load_module_is_noop() {
        let runtime = BuiltinRuntime::new();
        let result = runtime.load_module(Path::new("/fake/path.wasm"), None);
        assert!(result.is_ok());
    }

    #[test]
    fn has_cached_module_always_true() {
        let runtime = BuiltinRuntime::new();
        assert!(runtime.has_cached_module("abc123"));
    }
}
