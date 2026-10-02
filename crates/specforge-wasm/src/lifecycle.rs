use crate::integrity::hex_sha256;
use crate::runtime::{ExtensionLifecycleState, LoadedModule, WasmCallResult, WasmRuntime};
use specforge_common::{Diagnostic, Severity};
use specforge_registry::ManifestV2;
use std::path::Path;

/// Load a Wasm component from `wasm_path` under `extension_name`.
///
/// `expected_hash` enforces the lockfile pin (spec #21, T4): when it carries
/// a hash (from `specforge.lock`), the on-disk binary must match it — a
/// mismatch refuses the load with a remediation hint. `None` or an empty
/// string (legacy lockfile entries from before hash pinning) load unchanged.
pub fn load_wasm_module(
    extension_name: &str,
    wasm_path: &Path,
    runtime: &dyn WasmRuntime,
    expected_hash: Option<&str>,
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
            suggestion: Some(format!(
                "install the extension with: specforge add {}",
                extension_name
            )),
        });
    }

    // Content hash recorded on the loaded module (lockfile verification)
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

    // Lockfile pin enforcement (spec #21, T4): a recorded hash that no
    // longer matches the on-disk binary means the installed extension was
    // tampered with or corrupted after install. Refuse before touching the
    // runtime (this also denies a tampered binary a cache-hit load path).
    if let Some(expected) = expected_hash.filter(|h| !h.is_empty() && h != &wasm_hash) {
        return Err(Diagnostic {
            code: "E033".to_string(),
            severity: Severity::Error,
            message: format!(
                "integrity mismatch for '{}': lockfile records hash {} but the installed binary is {}",
                extension_name, expected, wasm_hash
            ),
            span: None,
            suggestion: Some(format!(
                "the installed binary changed after install — re-install it: specforge remove \"{0}\" && specforge add \"{0}\"",
                extension_name
            )),
        });
    }

    runtime
        .load_module_named(extension_name, wasm_path)
        .map_err(|e| Diagnostic {
            code: "E028".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': failed to load Wasm module: {}",
                extension_name, e
            ),
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

/// Validate the peer dependencies one extension declares against the full
/// manifest set (delegates to `specforge_registry::validate_peer_dependencies_of`).
pub fn validate_extension_peer_dependencies(
    manifest: &ManifestV2,
    all_manifests: &[ManifestV2],
) -> Vec<Diagnostic> {
    specforge_registry::validate_peer_dependencies_of(manifest, all_manifests)
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

        let module = load_wasm_module("test-ext", &wasm_path, &runtime, None).unwrap();
        assert_eq!(module.extension_name, "test-ext");
        assert_eq!(module.state, ExtensionLifecycleState::Loading);
        assert!(!module.wasm_hash.is_empty());
    }

    // B:load_wasm_module — verify unit "missing .wasm produces ExtensionError"
    #[test]
    fn test_missing_wasm_produces_extension_error() {
        let runtime = MockRuntime::new();
        let missing = Path::new("/nonexistent/ext.wasm");

        let err = load_wasm_module("test-ext", missing, &runtime, None).unwrap_err();
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
        let module = load_wasm_module("test-ext", &wasm_path, &runtime, None).unwrap();
        assert_eq!(module.state, ExtensionLifecycleState::Loading);

        // ensures: missing_binary_diagnosed
        let missing = Path::new("/nonexistent.wasm");
        let err = load_wasm_module("missing", missing, &runtime, None).unwrap_err();
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
        let runtime = MockRuntime::new().with_call_trap(
            "initialize",
            WasmTrapInfo {
                kind: "unreachable".to_string(),
                message: "init failed".to_string(),
                export_name: "initialize".to_string(),
            },
        );
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
        let runtime_err = MockRuntime::new().with_call_trap(
            "initialize",
            WasmTrapInfo {
                kind: "trap".to_string(),
                message: "boom".to_string(),
                export_name: "initialize".to_string(),
            },
        );
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
        }])
        .unwrap();

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
        let runtime = MockRuntime::new().with_call_trap(
            "validate",
            WasmTrapInfo {
                kind: "unreachable".to_string(),
                message: "boom".to_string(),
                export_name: "validate".to_string(),
            },
        );

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
        assert!(
            diags.is_empty(),
            "expected no diagnostics, got: {:?}",
            diags
        );
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

    #[test]
    fn test_peer_deps_report_only_the_extensions_own_peers() {
        // `@x/a` is a prefix of `@x/ab`, and `@x/c` depends on `@x/a`: neither
        // of their diagnostics belongs to `@x/a`, which declares no peers.
        let a = make_manifest("@x/a", &[]);
        let ab = make_manifest("@x/ab", &[("@x/missing", ">=1.0.0")]);
        let c = make_manifest("@x/c", &[("@x/a", ">=9.0.0")]);
        let all = vec![a.clone(), ab.clone(), c];

        assert!(validate_extension_peer_dependencies(&a, &all).is_empty());
        let diags = validate_extension_peer_dependencies(&ab, &all);
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert!(diags[0].message.contains("@x/missing"));
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
}
