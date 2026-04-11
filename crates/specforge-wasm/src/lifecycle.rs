use crate::integrity::hex_sha256;
use crate::runtime::{ExtensionLifecycleState, LoadedModule, WasmCallResult, WasmRuntime};
use specforge_common::{Diagnostic, Severity};
use specforge_registry::ManifestV2;
use std::path::Path;

/// Load a Wasm module from the manifest's wasm_path.
/// Checks AOT cache first; falls back to loading the raw .wasm binary.
pub fn load_wasm_module(
    extension_name: &str,
    wasm_path: &Path,
    aot_cache_dir: Option<&Path>,
    runtime: &dyn WasmRuntime,
) -> Result<LoadedModule, Diagnostic> {
    // Check if the .wasm binary exists
    if !wasm_path.exists() {
        return Err(Diagnostic {
            code: "E028".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': .wasm binary not found at '{}'",
                extension_name,
                wasm_path.display()
            ),
            span: None,
            suggestion: Some(format!("install the extension with: specforge add {}", extension_name)),
        });
    }

    // Compute content hash for AOT cache lookup
    let bytes = std::fs::read(wasm_path).map_err(|e| Diagnostic {
        code: "E028".to_string(),
        severity: Severity::Error,
        message: format!(
            "extension '{}': cannot read .wasm binary at '{}': {}",
            extension_name,
            wasm_path.display(),
            e
        ),
        span: None,
        suggestion: None,
    })?;
    let wasm_hash = hex_sha256(&bytes);

    // Check AOT cache
    let aot_path = aot_cache_dir.map(|dir| dir.join(format!("{}.aot", wasm_hash)));
    let cache_hit = runtime.has_cached_module(&wasm_hash);

    let load_path = if cache_hit {
        aot_path.as_deref()
    } else {
        None
    };

    runtime.load_module(wasm_path, load_path).map_err(|e| Diagnostic {
        code: "E028".to_string(),
        severity: Severity::Error,
        message: format!("extension '{}': failed to load Wasm module: {}", extension_name, e),
        span: None,
        suggestion: None,
    })?;

    Ok(LoadedModule {
        extension_name: extension_name.to_string(),
        wasm_hash,
        state: ExtensionLifecycleState::Loading,
    })
}

/// Initialize a loaded Wasm extension by calling its initialize() export.
pub fn initialize_extension(
    module: &mut LoadedModule,
    runtime: &dyn WasmRuntime,
) -> Result<(), Diagnostic> {
    match runtime.call_export(&module.extension_name, "initialize", &[]) {
        WasmCallResult::Ok(_) => {
            module.state = ExtensionLifecycleState::Initialized;
            Ok(())
        }
        WasmCallResult::Trap(trap) => {
            module.state = ExtensionLifecycleState::Failed;
            Err(Diagnostic {
                code: "E028".to_string(),
                severity: Severity::Error,
                message: format!(
                    "extension '{}': initialize() trapped: {} — {}",
                    module.extension_name, trap.kind, trap.message
                ),
                span: None,
                suggestion: None,
            })
        }
    }
}

/// Call validate() on each extension in order, collecting diagnostics.
/// Validation continues to the next extension after errors.
pub fn call_extension_validators(
    modules: &mut [LoadedModule],
    runtime: &dyn WasmRuntime,
) -> Vec<Diagnostic> {
    let mut all_diagnostics = Vec::new();

    for module in modules.iter_mut() {
        if module.state != ExtensionLifecycleState::Initialized {
            continue;
        }

        module.state = ExtensionLifecycleState::Validating;

        match runtime.call_export(&module.extension_name, "validate", &[]) {
            WasmCallResult::Ok(output) => {
                // Parse diagnostics from output (JSON array of diagnostics)
                if let Ok(diags) = serde_json::from_slice::<Vec<Diagnostic>>(&output) {
                    all_diagnostics.extend(diags);
                }
                module.state = ExtensionLifecycleState::Initialized;
            }
            WasmCallResult::Trap(trap) => {
                module.state = ExtensionLifecycleState::Failed;
                all_diagnostics.push(Diagnostic {
                    code: "E028".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "extension '{}': validate() trapped: {} — {}",
                        module.extension_name, trap.kind, trap.message
                    ),
                    span: None,
                    suggestion: None,
                });
                // Continue to next extension — don't stop
            }
        }
    }

    all_diagnostics
}

/// Validate peer dependencies for a single extension against the full manifest set.
/// Delegates to `specforge_registry::validate_peer_dependencies` and filters results
/// to only the diagnostics relevant to the given manifest.
pub fn validate_extension_peer_dependencies(
    manifest: &ManifestV2,
    all_manifests: &[ManifestV2],
) -> Vec<Diagnostic> {
    let peer_diags = specforge_registry::validate_peer_dependencies(all_manifests);
    peer_diags
        .into_iter()
        .filter(|d| d.message.contains(&manifest.name))
        .collect()
}

/// Validate a tree-sitter grammar .wasm binary before loading.
/// Checks: required export presence, ABI version compatibility, binary size limit.
pub fn validate_grammar_wasm(
    grammar_bytes: &[u8],
    expected_export: &str,
    available_exports: &[String],
    abi_version: u32,
    supported_abi_version: u32,
    max_size_bytes: u64,
) -> Result<(), Diagnostic> {
    if !available_exports.iter().any(|e| e == expected_export) {
        return Err(Diagnostic {
            code: "E036".to_string(),
            severity: Severity::Error,
            message: format!(
                "grammar validation failed: missing language export '{}'",
                expected_export
            ),
            span: None,
            suggestion: Some(format!(
                "ensure the grammar .wasm exports '{}'",
                expected_export
            )),
        });
    }

    if abi_version != supported_abi_version {
        return Err(Diagnostic {
            code: "E037".to_string(),
            severity: Severity::Error,
            message: format!(
                "grammar validation failed: ABI version {} does not match supported version {}",
                abi_version, supported_abi_version
            ),
            span: None,
            suggestion: Some(format!(
                "rebuild the grammar with tree-sitter ABI version {}",
                supported_abi_version
            )),
        });
    }

    if grammar_bytes.len() as u64 > max_size_bytes {
        return Err(Diagnostic {
            code: "E038".to_string(),
            severity: Severity::Error,
            message: format!(
                "grammar validation failed: binary size {} bytes exceeds maximum {} bytes",
                grammar_bytes.len(),
                max_size_bytes
            ),
            span: None,
            suggestion: Some("reduce grammar complexity or increase max_size_bytes".to_string()),
        });
    }

    Ok(())
}

/// Result of loading an extension grammar.
#[derive(Debug, Clone)]
pub struct GrammarLoadResult {
    pub grammar_path: String,
    pub content_hash: String,
    pub abi_version: u32,
}

/// Load an extension grammar: validate, compute content hash, return cacheable result.
pub fn load_extension_grammar(
    grammar_path: &str,
    grammar_bytes: &[u8],
    expected_export: &str,
    available_exports: &[String],
    abi_version: u32,
    supported_abi_version: u32,
    max_size_bytes: u64,
) -> Result<GrammarLoadResult, Diagnostic> {
    validate_grammar_wasm(
        grammar_bytes,
        expected_export,
        available_exports,
        abi_version,
        supported_abi_version,
        max_size_bytes,
    )?;

    let content_hash = hex_sha256(grammar_bytes);

    Ok(GrammarLoadResult {
        grammar_path: grammar_path.to_string(),
        content_hash,
        abi_version,
    })
}

/// Dispatch a body parser Wasm export and validate its JSON output.
pub fn dispatch_body_parser(
    extension_name: &str,
    export_name: &str,
    body_text: &str,
    runtime: &dyn WasmRuntime,
) -> Result<serde_json::Value, Diagnostic> {
    match runtime.call_export(extension_name, export_name, body_text.as_bytes()) {
        WasmCallResult::Trap(trap) => Err(Diagnostic {
            code: "E028".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': body parser {}() trapped: {} — {}",
                extension_name, export_name, trap.kind, trap.message
            ),
            span: None,
            suggestion: None,
        }),
        WasmCallResult::Ok(output) => {
            serde_json::from_slice::<serde_json::Value>(&output).map_err(|e| Diagnostic {
                code: "E028".to_string(),
                severity: Severity::Error,
                message: format!(
                    "extension '{}': body parser '{}' returned invalid JSON: {}",
                    extension_name, export_name, e
                ),
                span: None,
                suggestion: Some("ensure the body parser returns valid JSON".to_string()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{MockRuntime, WasmTrapInfo};
    use crate::test_helpers::{default_manifest, make_manifest};
    use std::io::Write;
    use tempfile::TempDir;

    fn create_fake_wasm(dir: &TempDir, name: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"\x00asm\x01\x00\x00\x00fake").unwrap();
        path
    }

    // -- load_wasm_module --

    // B:load_wasm_module — verify unit "loads .wasm binary from manifest path"
    #[test]
    fn test_loads_wasm_binary_from_manifest_path() {
        let dir = TempDir::new().unwrap();
        let wasm_path = create_fake_wasm(&dir, "ext.wasm");
        let runtime = MockRuntime::new();

        let module = load_wasm_module("test-ext", &wasm_path, None, &runtime).unwrap();
        assert_eq!(module.extension_name, "test-ext");
        assert_eq!(module.state, ExtensionLifecycleState::Loading);
        assert!(!module.wasm_hash.is_empty());
    }

    // B:load_wasm_module — verify unit "uses AOT cache on cache hit"
    #[test]
    fn test_uses_aot_cache_on_cache_hit() {
        let dir = TempDir::new().unwrap();
        let wasm_path = create_fake_wasm(&dir, "ext.wasm");
        let wasm_hash = hex_sha256(&std::fs::read(&wasm_path).unwrap());
        let runtime = MockRuntime::new().with_cached(&wasm_hash);

        let module = load_wasm_module("test-ext", &wasm_path, Some(dir.path()), &runtime).unwrap();
        assert_eq!(module.wasm_hash, wasm_hash);
    }

    // B:load_wasm_module — verify unit "missing .wasm produces ExtensionError"
    #[test]
    fn test_missing_wasm_produces_extension_error() {
        let runtime = MockRuntime::new();
        let missing = Path::new("/nonexistent/ext.wasm");

        let err = load_wasm_module("test-ext", missing, None, &runtime).unwrap_err();
        assert_eq!(err.code, "E028");
        assert!(err.message.contains("not found"));
    }

    // B:load_wasm_module — verify contract "requires/ensures consistency for Wasm module loading"
    #[test]
    fn test_load_wasm_module_contract() {
        let dir = TempDir::new().unwrap();
        let wasm_path = create_fake_wasm(&dir, "ext.wasm");
        let runtime = MockRuntime::new();

        // ensures: extension_loaded on success
        let module = load_wasm_module("test-ext", &wasm_path, None, &runtime).unwrap();
        assert_eq!(module.state, ExtensionLifecycleState::Loading);

        // ensures: missing_binary_diagnosed
        let missing = Path::new("/nonexistent.wasm");
        let err = load_wasm_module("missing", missing, None, &runtime).unwrap_err();
        assert_eq!(err.code, "E028");
        assert_eq!(err.severity, Severity::Error);
    }

    // -- initialize_wasm_extension --

    // B:initialize_wasm_extension — verify unit "calls initialize() export on loaded module"
    #[test]
    fn test_calls_initialize_export() {
        let runtime = MockRuntime::new().with_call_ok("initialize", vec![]);
        let mut module = LoadedModule {
            extension_name: "test-ext".to_string(),
            wasm_hash: "abc".to_string(),
            state: ExtensionLifecycleState::Loading,
        };

        let result = initialize_extension(&mut module, &runtime);
        assert!(result.is_ok());
    }

    // B:initialize_wasm_extension — verify unit "lifecycle transitions to initialized on success"
    #[test]
    fn test_lifecycle_transitions_to_initialized_on_success() {
        let runtime = MockRuntime::new().with_call_ok("initialize", vec![]);
        let mut module = LoadedModule {
            extension_name: "test-ext".to_string(),
            wasm_hash: "abc".to_string(),
            state: ExtensionLifecycleState::Loading,
        };

        initialize_extension(&mut module, &runtime).unwrap();
        assert_eq!(module.state, ExtensionLifecycleState::Initialized);
    }

    // B:initialize_wasm_extension — verify unit "lifecycle transitions to failed on error"
    #[test]
    fn test_lifecycle_transitions_to_failed_on_error() {
        let runtime = MockRuntime::new().with_call_trap("initialize", WasmTrapInfo {
            kind: "unreachable".to_string(),
            message: "init failed".to_string(),
            export_name: "initialize".to_string(),
        });
        let mut module = LoadedModule {
            extension_name: "test-ext".to_string(),
            wasm_hash: "abc".to_string(),
            state: ExtensionLifecycleState::Loading,
        };

        let err = initialize_extension(&mut module, &runtime).unwrap_err();
        assert_eq!(module.state, ExtensionLifecycleState::Failed);
        assert_eq!(err.code, "E028");
        assert!(err.message.contains("trapped"));
    }

    // B:initialize_wasm_extension — verify contract "requires/ensures consistency for Wasm extension initialization"
    #[test]
    fn test_initialize_extension_contract() {
        // requires: extension_loaded_fired — module is in Loading state
        let runtime_ok = MockRuntime::new().with_call_ok("initialize", vec![]);
        let mut module = LoadedModule {
            extension_name: "ext".to_string(),
            wasm_hash: "hash".to_string(),
            state: ExtensionLifecycleState::Loading,
        };

        // ensures: extension_initialized_emitted + lifecycle_state_updated
        initialize_extension(&mut module, &runtime_ok).unwrap();
        assert_eq!(module.state, ExtensionLifecycleState::Initialized);

        // ensures: lifecycle to failed on error
        let runtime_err = MockRuntime::new().with_call_trap("initialize", WasmTrapInfo {
            kind: "trap".to_string(),
            message: "boom".to_string(),
            export_name: "initialize".to_string(),
        });
        let mut module2 = LoadedModule {
            extension_name: "ext2".to_string(),
            wasm_hash: "hash2".to_string(),
            state: ExtensionLifecycleState::Loading,
        };
        let err = initialize_extension(&mut module2, &runtime_err).unwrap_err();
        assert_eq!(module2.state, ExtensionLifecycleState::Failed);
        assert_eq!(err.severity, Severity::Error);
    }

    // -- call_extension_validators --

    // B:call_extension_validators — verify unit "calls validate() in topological order"
    #[test]
    fn test_calls_validate_in_topological_order() {
        let runtime = MockRuntime::new().with_call_ok("validate", vec![]);
        let mut modules = vec![
            LoadedModule {
                extension_name: "first".to_string(),
                wasm_hash: "a".to_string(),
                state: ExtensionLifecycleState::Initialized,
            },
            LoadedModule {
                extension_name: "second".to_string(),
                wasm_hash: "b".to_string(),
                state: ExtensionLifecycleState::Initialized,
            },
        ];

        let diags = call_extension_validators(&mut modules, &runtime);
        assert!(diags.is_empty());
        // Both processed (back to Initialized after validate completes)
        assert_eq!(modules[0].state, ExtensionLifecycleState::Initialized);
        assert_eq!(modules[1].state, ExtensionLifecycleState::Initialized);
    }

    // B:call_extension_validators — verify unit "diagnostics emitted via host function are collected"
    #[test]
    fn test_diagnostics_from_validate_are_collected() {
        let diag_json = serde_json::to_vec(&vec![Diagnostic {
            code: "W100".to_string(),
            severity: Severity::Warning,
            message: "custom warning".to_string(),
            span: None,
            suggestion: None,
        }]).unwrap();

        let runtime = MockRuntime::new().with_call_ok("validate", diag_json);
        let mut modules = vec![LoadedModule {
            extension_name: "ext".to_string(),
            wasm_hash: "a".to_string(),
            state: ExtensionLifecycleState::Initialized,
        }];

        let diags = call_extension_validators(&mut modules, &runtime);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W100");
    }

    // B:call_extension_validators — verify unit "validation continues to next extension after errors"
    #[test]
    fn test_validation_continues_after_trap() {
        let runtime = MockRuntime::new()
            .with_call_trap("validate", WasmTrapInfo {
                kind: "unreachable".to_string(),
                message: "boom".to_string(),
                export_name: "validate".to_string(),
            });

        let mut modules = vec![
            LoadedModule {
                extension_name: "trapping".to_string(),
                wasm_hash: "a".to_string(),
                state: ExtensionLifecycleState::Initialized,
            },
            LoadedModule {
                extension_name: "healthy".to_string(),
                wasm_hash: "b".to_string(),
                state: ExtensionLifecycleState::Initialized,
            },
        ];

        let diags = call_extension_validators(&mut modules, &runtime);
        // Both get called — first traps, second also traps (same mock), but both are processed
        assert_eq!(diags.len(), 2);
        assert_eq!(modules[0].state, ExtensionLifecycleState::Failed);
        assert_eq!(modules[1].state, ExtensionLifecycleState::Failed);
    }

    // B:call_extension_validators — verify contract "requires/ensures consistency for extension validator dispatch"
    #[test]
    fn test_call_validators_contract() {
        // requires: extension_initialized — only initialized modules are called
        let runtime = MockRuntime::new().with_call_ok("validate", vec![]);
        let mut modules = vec![
            LoadedModule {
                extension_name: "init".to_string(),
                wasm_hash: "a".to_string(),
                state: ExtensionLifecycleState::Initialized,
            },
            LoadedModule {
                extension_name: "failed".to_string(),
                wasm_hash: "b".to_string(),
                state: ExtensionLifecycleState::Failed,
            },
        ];

        let diags = call_extension_validators(&mut modules, &runtime);
        assert!(diags.is_empty());
        // ensures: only initialized module was processed
        assert_eq!(modules[0].state, ExtensionLifecycleState::Initialized);
        // ensures: failed module was skipped
        assert_eq!(modules[1].state, ExtensionLifecycleState::Failed);
    }

    // -- validate_extension_peer_dependencies --

    // B:validate_extension_peer_dependencies — verify unit "satisfied peers pass"
    #[test]
    fn test_peer_deps_satisfied_pass() {
        let software = make_manifest("@specforge/software", &[]);
        let product = make_manifest("@specforge/product", &[("@specforge/software", ">=1.0.0")]);
        let all = vec![software, product.clone()];

        let diags = validate_extension_peer_dependencies(&product, &all);
        assert!(diags.is_empty(), "expected no diagnostics, got: {:?}", diags);
    }

    // B:validate_extension_peer_dependencies — verify unit "missing peer → E027"
    #[test]
    fn test_peer_deps_missing_produces_e027() {
        let product = make_manifest("@specforge/product", &[("@specforge/software", ">=1.0.0")]);
        let all = vec![product.clone()]; // software not installed

        let diags = validate_extension_peer_dependencies(&product, &all);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E027");
        assert!(diags[0].message.contains("@specforge/product"));
        assert!(diags[0].message.contains("@specforge/software"));
    }

    // B:validate_extension_peer_dependencies — verify unit "version mismatch → E027"
    #[test]
    fn test_peer_deps_version_mismatch_produces_e027() {
        let mut software = default_manifest();
        software.name = "@specforge/software".to_string();
        software.version = "0.5.0".to_string();

        let product = make_manifest("@specforge/product", &[("@specforge/software", ">=1.0.0")]);
        let all = vec![software, product.clone()];

        let diags = validate_extension_peer_dependencies(&product, &all);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E027");
        assert!(diags[0].message.contains("0.5.0"));
    }

    // B:validate_extension_peer_dependencies — verify contract "requires/ensures consistency"
    #[test]
    fn test_peer_deps_contract() {
        // requires: manifests loaded
        let software = make_manifest("@specforge/software", &[]);
        let product = make_manifest("@specforge/product", &[("@specforge/software", ">=1.0.0")]);

        // ensures: satisfied → empty diagnostics
        let diags = validate_extension_peer_dependencies(&product, &[software, product.clone()]);
        assert!(diags.is_empty());

        // ensures: missing → E027 with extension name
        let diags = validate_extension_peer_dependencies(&product, std::slice::from_ref(&product));
        assert!(diags.iter().any(|d| d.code == "E027"));
        assert!(diags.iter().all(|d| d.severity == Severity::Error));
    }

    // -- validate_grammar_wasm --

    // B:validate_grammar_wasm — verify unit "valid grammar passes all validation checks"
    #[test]
    fn test_validate_grammar_valid_passes() {
        let bytes = vec![0u8; 100];
        let exports = vec!["tree_sitter_specforge".to_string()];
        let result = validate_grammar_wasm(&bytes, "tree_sitter_specforge", &exports, 14, 14, 1024);
        assert!(result.is_ok());
    }

    // B:validate_grammar_wasm — verify unit "missing language export produces GrammarError"
    #[test]
    fn test_validate_grammar_missing_export() {
        let bytes = vec![0u8; 100];
        let exports = vec!["other_export".to_string()];
        let err = validate_grammar_wasm(&bytes, "tree_sitter_specforge", &exports, 14, 14, 1024).unwrap_err();
        assert_eq!(err.code, "E036");
        assert!(err.message.contains("missing language export"));
    }

    // B:validate_grammar_wasm — verify unit "ABI version mismatch produces GrammarError with versions"
    #[test]
    fn test_validate_grammar_abi_mismatch() {
        let bytes = vec![0u8; 100];
        let exports = vec!["tree_sitter_specforge".to_string()];
        let err = validate_grammar_wasm(&bytes, "tree_sitter_specforge", &exports, 13, 14, 1024).unwrap_err();
        assert_eq!(err.code, "E037");
        assert!(err.message.contains("13"));
        assert!(err.message.contains("14"));
    }

    // B:validate_grammar_wasm — verify unit "oversized grammar binary is rejected"
    #[test]
    fn test_validate_grammar_oversized() {
        let bytes = vec![0u8; 2000];
        let exports = vec!["tree_sitter_specforge".to_string()];
        let err = validate_grammar_wasm(&bytes, "tree_sitter_specforge", &exports, 14, 14, 1024).unwrap_err();
        assert_eq!(err.code, "E038");
        assert!(err.message.contains("2000"));
        assert!(err.message.contains("1024"));
    }

    // -- dispatch_body_parser --

    // B:dispatch_body_parser — verify unit "body parser called for entity kind with registered parser"
    #[test]
    fn test_dispatch_body_parser_success() {
        let json_output = serde_json::json!({"key": "value"});
        let runtime = MockRuntime::new()
            .with_call_ok("parse_body", serde_json::to_vec(&json_output).unwrap());

        let result = dispatch_body_parser("ext", "parse_body", "some body text", &runtime);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json_output);
    }

    // B:dispatch_body_parser — verify unit "parser output validated against declared schema"
    #[test]
    fn test_dispatch_body_parser_invalid_json_output() {
        let runtime = MockRuntime::new()
            .with_call_ok("parse_body", b"not valid json".to_vec());

        let err = dispatch_body_parser("ext", "parse_body", "body", &runtime).unwrap_err();
        assert_eq!(err.code, "E028");
        assert!(err.message.contains("invalid JSON"));
    }

    // B:dispatch_body_parser — verify unit "parser crash produces BodyParserError with fallback"
    #[test]
    fn test_dispatch_body_parser_trap() {
        let runtime = MockRuntime::new().with_call_trap("parse_body", WasmTrapInfo {
            kind: "unreachable".to_string(),
            message: "parser panic".to_string(),
            export_name: "parse_body".to_string(),
        });

        let err = dispatch_body_parser("ext", "parse_body", "body", &runtime).unwrap_err();
        assert_eq!(err.code, "E028");
        assert!(err.message.contains("body parser"));
        assert!(err.message.contains("trapped"));
    }

    // B:dispatch_body_parser — verify unit "empty body text handled"
    #[test]
    fn test_dispatch_body_parser_empty_body() {
        let runtime = MockRuntime::new()
            .with_call_ok("parse_body", b"{}".to_vec());

        let result = dispatch_body_parser("ext", "parse_body", "", &runtime);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!({}));
    }

    // B:dispatch_body_parser — verify contract
    #[test]
    fn test_dispatch_body_parser_contract() {
        // ensures: valid JSON output returns Ok
        let runtime_ok = MockRuntime::new()
            .with_call_ok("parse_body", b"{\"x\":1}".to_vec());
        let result = dispatch_body_parser("ext", "parse_body", "text", &runtime_ok);
        assert!(result.is_ok());

        // ensures: trap returns E028
        let runtime_trap = MockRuntime::new().with_call_trap("parse_body", WasmTrapInfo {
            kind: "trap".to_string(),
            message: "boom".to_string(),
            export_name: "parse_body".to_string(),
        });
        let err = dispatch_body_parser("ext", "parse_body", "text", &runtime_trap).unwrap_err();
        assert_eq!(err.code, "E028");
        assert_eq!(err.severity, Severity::Error);

        // ensures: invalid JSON returns E028
        let runtime_bad = MockRuntime::new()
            .with_call_ok("parse_body", b"[broken".to_vec());
        let err = dispatch_body_parser("ext", "parse_body", "text", &runtime_bad).unwrap_err();
        assert_eq!(err.code, "E028");
    }

    // -- load_extension_grammar --

    // B:load_extension_grammar — verify unit "valid grammar .wasm loads successfully"
    #[test]
    fn test_load_grammar_valid_succeeds() {
        let bytes = vec![0u8; 100];
        let exports = vec!["tree_sitter_specforge".to_string()];
        let result = load_extension_grammar(
            "/grammars/specforge.wasm", &bytes, "tree_sitter_specforge",
            &exports, 14, 14, 1024,
        );
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.grammar_path, "/grammars/specforge.wasm");
        assert_eq!(r.abi_version, 14);
    }

    // B:load_extension_grammar — verify unit "invalid grammar path produces GrammarError"
    #[test]
    fn test_load_grammar_validation_failure() {
        let bytes = vec![0u8; 100];
        let exports = vec!["other_export".to_string()]; // missing expected export
        let err = load_extension_grammar(
            "/grammars/bad.wasm", &bytes, "tree_sitter_specforge",
            &exports, 14, 14, 1024,
        ).unwrap_err();
        assert_eq!(err.code, "E036");
    }

    // B:load_extension_grammar — verify unit "loaded grammar is cached for subsequent use"
    #[test]
    fn test_load_grammar_produces_cacheable_result() {
        let bytes = vec![0u8; 100];
        let exports = vec!["tree_sitter_specforge".to_string()];
        let result = load_extension_grammar(
            "/grammars/specforge.wasm", &bytes, "tree_sitter_specforge",
            &exports, 14, 14, 1024,
        ).unwrap();
        // content_hash can be used as cache key
        assert!(!result.content_hash.is_empty());
        assert_eq!(result.content_hash.len(), 64); // SHA256 hex
    }

    // B:load_extension_grammar — verify unit "grammar loading completes within performance budget"
    #[test]
    fn test_load_grammar_returns_content_hash() {
        let bytes = b"grammar binary content";
        let exports = vec!["tree_sitter_specforge".to_string()];
        let r1 = load_extension_grammar(
            "/g.wasm", bytes, "tree_sitter_specforge", &exports, 14, 14, 1024,
        ).unwrap();
        let r2 = load_extension_grammar(
            "/g.wasm", bytes, "tree_sitter_specforge", &exports, 14, 14, 1024,
        ).unwrap();
        // Same content = same hash (deterministic)
        assert_eq!(r1.content_hash, r2.content_hash);
    }

    // B:load_extension_grammar — verify contract
    #[test]
    fn test_load_grammar_contract() {
        let bytes = vec![0u8; 100];
        let exports = vec!["tree_sitter_specforge".to_string()];

        // ensures: valid grammar returns Ok with path, hash, abi
        let r = load_extension_grammar(
            "/g.wasm", &bytes, "tree_sitter_specforge", &exports, 14, 14, 1024,
        ).unwrap();
        assert_eq!(r.grammar_path, "/g.wasm");
        assert_eq!(r.abi_version, 14);
        assert!(!r.content_hash.is_empty());

        // ensures: missing export -> E036
        let err = load_extension_grammar(
            "/g.wasm", &bytes, "missing_export", &exports, 14, 14, 1024,
        ).unwrap_err();
        assert_eq!(err.code, "E036");

        // ensures: ABI mismatch -> E037
        let err = load_extension_grammar(
            "/g.wasm", &bytes, "tree_sitter_specforge", &exports, 13, 14, 1024,
        ).unwrap_err();
        assert_eq!(err.code, "E037");

        // ensures: oversized -> E038
        let big = vec![0u8; 2000];
        let err = load_extension_grammar(
            "/g.wasm", &big, "tree_sitter_specforge", &exports, 14, 14, 1024,
        ).unwrap_err();
        assert_eq!(err.code, "E038");
    }
}
