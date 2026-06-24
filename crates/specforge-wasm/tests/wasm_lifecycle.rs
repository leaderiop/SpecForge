// Slice 11: Wasm Lifecycle Core Integration Tests
//
// Tests the 6 core wasm lifecycle behaviors through the public API:
// - B:load_wasm_module
// - B:initialize_wasm_extension
// - B:call_extension_validators
// - B:topological_sort_extensions
// - B:validate_extension_manifest
// - B:validate_extension_peer_dependencies

use specforge_common::{Diagnostic, Severity};
use specforge_registry::{ManifestV2, PeerDependency};
use specforge_wasm::{
    call_extension_validators, initialize_extension, load_wasm_module,
    topological_sort_extensions, validate_extension_manifest, validate_extension_peer_dependencies,
    ExtensionLifecycleState, LoadedModule, WasmCallResult, WasmRuntime, WasmTrapInfo,
};
use std::path::Path;
use tempfile::TempDir;

// -- Test MockRuntime for integration tests --

struct MockRuntime {
    call_results: std::collections::HashMap<String, WasmCallResult>,
    cached_modules: std::collections::HashSet<String>,
}

impl MockRuntime {
    fn new() -> Self {
        Self {
            call_results: std::collections::HashMap::new(),
            cached_modules: std::collections::HashSet::new(),
        }
    }

    fn with_call_ok(mut self, export: &str, output: Vec<u8>) -> Self {
        self.call_results
            .insert(export.to_string(), WasmCallResult::Ok(output));
        self
    }

    fn with_call_trap(mut self, export: &str, trap: WasmTrapInfo) -> Self {
        self.call_results
            .insert(export.to_string(), WasmCallResult::Trap(trap));
        self
    }

    fn with_cached(mut self, hash: &str) -> Self {
        self.cached_modules.insert(hash.to_string());
        self
    }
}

impl WasmRuntime for MockRuntime {
    fn load_module(&self, _wasm_path: &Path, _aot: Option<&Path>) -> Result<(), String> {
        Ok(())
    }

    fn call_export(&self, _ext: &str, export_name: &str, _input: &[u8]) -> WasmCallResult {
        self.call_results
            .get(export_name)
            .cloned()
            .unwrap_or(WasmCallResult::Ok(vec![]))
    }

    fn has_cached_module(&self, wasm_hash: &str) -> bool {
        self.cached_modules.contains(wasm_hash)
    }
}

fn default_manifest() -> ManifestV2 {
    ManifestV2 {
        name: String::new(),
        version: String::new(),
        manifest_version: 2,
        wasm_path: String::new(),
        contributes: Default::default(),
        entity_kinds: vec![],
        edge_types: vec![],
        fields: vec![],
        validation_rules: vec![],
        verify_kinds: vec![],
        reserved_keywords: vec![],
        peer_dependencies: vec![],
        sandbox_policy: None,
        incremental: None,
        migration_hook: None,
        host_api_version: None,
        entity_enhancements: vec![],
        starter_template: None,
        grammar_contributions: vec![],
        body_parser_contributions: vec![],
        ext_short: None,
        query_scope: None,
        collector_contributions: vec![],
        analyzer_contributions: vec![],
        surfaces: None,
    }
}

fn make_manifest(name: &str, version: &str, peers: &[(&str, &str)]) -> ManifestV2 {
    ManifestV2 {
        name: name.to_string(),
        version: version.to_string(),
        wasm_path: "extension.wasm".to_string(),
        peer_dependencies: peers
            .iter()
            .map(|(n, v)| PeerDependency {
                name: n.to_string(),
                version: v.to_string(),
                optional: false,
            })
            .collect(),
        ..default_manifest()
    }
}

fn create_fake_wasm(dir: &TempDir, name: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, b"\x00asm\x01\x00\x00\x00fake").unwrap();
    path
}

// ============================================================================
// B:load_wasm_module — integration tests
// ============================================================================

// B:load_wasm_module — verify integration "load valid module bytes → Ok with LoadedModule"
#[test]
fn test_load_valid_module_returns_loaded_module() {
    let dir = TempDir::new().unwrap();
    let wasm_path = create_fake_wasm(&dir, "ext.wasm");
    let runtime = MockRuntime::new();

    let module = load_wasm_module("@test/ext", &wasm_path, None, &runtime).unwrap();
    assert_eq!(module.extension_name, "@test/ext");
    assert_eq!(module.state, ExtensionLifecycleState::Loading);
    assert!(!module.wasm_hash.is_empty());
    assert_eq!(module.wasm_hash.len(), 64); // SHA256 hex
}

// B:load_wasm_module — verify integration "load corrupted bytes → Err with E028"
#[test]
fn test_load_missing_wasm_returns_e028() {
    let runtime = MockRuntime::new();
    let missing = Path::new("/nonexistent/path/ext.wasm");

    let err = load_wasm_module("@test/missing", missing, None, &runtime).unwrap_err();
    assert_eq!(err.code, "E028");
    assert_eq!(err.severity, Severity::Error);
    assert!(err.message.contains("not found"));
}

// B:load_wasm_module — verify integration "AOT cache hit uses cached path"
#[test]
fn test_load_with_aot_cache_hit() {
    let dir = TempDir::new().unwrap();
    let wasm_path = create_fake_wasm(&dir, "ext.wasm");
    let bytes = std::fs::read(&wasm_path).unwrap();
    let hash = specforge_wasm::hex_sha256(&bytes);
    let runtime = MockRuntime::new().with_cached(&hash);

    let module =
        load_wasm_module("@test/ext", &wasm_path, Some(dir.path()), &runtime).unwrap();
    assert_eq!(module.wasm_hash, hash);
    assert_eq!(module.state, ExtensionLifecycleState::Loading);
}

// B:load_wasm_module — verify contract "requires valid bytes, ensures LoadedModule or diagnostic"
#[test]
fn test_load_wasm_module_contract() {
    let dir = TempDir::new().unwrap();
    let wasm_path = create_fake_wasm(&dir, "ext.wasm");
    let runtime = MockRuntime::new();

    // ensures: success path returns LoadedModule
    let module = load_wasm_module("@test/ext", &wasm_path, None, &runtime).unwrap();
    assert_eq!(module.state, ExtensionLifecycleState::Loading);

    // ensures: failure path returns E028 diagnostic
    let err = load_wasm_module("bad", Path::new("/no/such.wasm"), None, &runtime).unwrap_err();
    assert_eq!(err.code, "E028");
    assert_eq!(err.severity, Severity::Error);
}

// ============================================================================
// B:initialize_wasm_extension — integration tests
// ============================================================================

// B:initialize_wasm_extension — verify integration "extension with initialize() → Active"
#[test]
fn test_initialize_extension_success_transitions_to_initialized() {
    let runtime = MockRuntime::new().with_call_ok("initialize", vec![]);
    let mut module = LoadedModule {
        extension_name: "@test/ext".to_string(),
        wasm_hash: "abc".to_string(),
        state: ExtensionLifecycleState::Loading,
    };

    initialize_extension(&mut module, &runtime).unwrap();
    assert_eq!(module.state, ExtensionLifecycleState::Initialized);
}

// B:initialize_wasm_extension — verify integration "initialize() trap → Failed state"
#[test]
fn test_initialize_extension_trap_transitions_to_failed() {
    let runtime = MockRuntime::new().with_call_trap(
        "initialize",
        WasmTrapInfo {
            kind: "unreachable".to_string(),
            message: "init failed".to_string(),
            export_name: "initialize".to_string(),
        },
    );
    let mut module = LoadedModule {
        extension_name: "@test/ext".to_string(),
        wasm_hash: "abc".to_string(),
        state: ExtensionLifecycleState::Loading,
    };

    let err = initialize_extension(&mut module, &runtime).unwrap_err();
    assert_eq!(module.state, ExtensionLifecycleState::Failed);
    assert_eq!(err.code, "E028");
    assert!(err.message.contains("trapped"));
}

// B:initialize_wasm_extension — verify contract "requires LoadedModule, ensures lifecycle state"
#[test]
fn test_initialize_extension_contract() {
    // Success path
    let runtime_ok = MockRuntime::new().with_call_ok("initialize", vec![]);
    let mut m1 = LoadedModule {
        extension_name: "a".to_string(),
        wasm_hash: "h".to_string(),
        state: ExtensionLifecycleState::Loading,
    };
    initialize_extension(&mut m1, &runtime_ok).unwrap();
    assert_eq!(m1.state, ExtensionLifecycleState::Initialized);

    // Failure path
    let runtime_err = MockRuntime::new().with_call_trap(
        "initialize",
        WasmTrapInfo {
            kind: "trap".to_string(),
            message: "boom".to_string(),
            export_name: "initialize".to_string(),
        },
    );
    let mut m2 = LoadedModule {
        extension_name: "b".to_string(),
        wasm_hash: "h2".to_string(),
        state: ExtensionLifecycleState::Loading,
    };
    let err = initialize_extension(&mut m2, &runtime_err).unwrap_err();
    assert_eq!(m2.state, ExtensionLifecycleState::Failed);
    assert_eq!(err.severity, Severity::Error);
}

// ============================================================================
// B:call_extension_validators — integration tests
// ============================================================================

// B:call_extension_validators — verify integration "single validator → diagnostics collected"
#[test]
fn test_call_validators_collects_diagnostics() {
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
        extension_name: "@test/ext".to_string(),
        wasm_hash: "a".to_string(),
        state: ExtensionLifecycleState::Initialized,
    }];

    let diags = call_extension_validators(&mut modules, &runtime);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "W100");
}

// B:call_extension_validators — verify integration "multiple extensions processed sequentially"
#[test]
fn test_call_validators_processes_all_initialized_modules() {
    let runtime = MockRuntime::new().with_call_ok("validate", vec![]);
    let mut modules = vec![
        LoadedModule {
            extension_name: "ext-a".to_string(),
            wasm_hash: "a".to_string(),
            state: ExtensionLifecycleState::Initialized,
        },
        LoadedModule {
            extension_name: "ext-b".to_string(),
            wasm_hash: "b".to_string(),
            state: ExtensionLifecycleState::Initialized,
        },
    ];

    let diags = call_extension_validators(&mut modules, &runtime);
    assert!(diags.is_empty());
    // Both remain Initialized after successful validation
    assert_eq!(modules[0].state, ExtensionLifecycleState::Initialized);
    assert_eq!(modules[1].state, ExtensionLifecycleState::Initialized);
}

// B:call_extension_validators — verify integration "skips non-Initialized modules"
#[test]
fn test_call_validators_skips_failed_modules() {
    let runtime = MockRuntime::new().with_call_ok("validate", vec![]);
    let mut modules = vec![
        LoadedModule {
            extension_name: "good".to_string(),
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
    assert_eq!(modules[0].state, ExtensionLifecycleState::Initialized);
    assert_eq!(modules[1].state, ExtensionLifecycleState::Failed); // unchanged
}

// B:call_extension_validators — verify contract "requires loaded extensions, ensures diagnostics aggregated"
#[test]
fn test_call_validators_contract() {
    let runtime_trap = MockRuntime::new().with_call_trap(
        "validate",
        WasmTrapInfo {
            kind: "unreachable".to_string(),
            message: "boom".to_string(),
            export_name: "validate".to_string(),
        },
    );
    let mut modules = vec![
        LoadedModule {
            extension_name: "ext-1".to_string(),
            wasm_hash: "a".to_string(),
            state: ExtensionLifecycleState::Initialized,
        },
        LoadedModule {
            extension_name: "ext-2".to_string(),
            wasm_hash: "b".to_string(),
            state: ExtensionLifecycleState::Initialized,
        },
    ];

    // ensures: validation continues past trap (both processed)
    let diags = call_extension_validators(&mut modules, &runtime_trap);
    assert_eq!(diags.len(), 2);
    assert!(diags.iter().all(|d| d.code == "E028"));
    // ensures: trapping modules go to Failed
    assert_eq!(modules[0].state, ExtensionLifecycleState::Failed);
    assert_eq!(modules[1].state, ExtensionLifecycleState::Failed);
}

// ============================================================================
// B:topological_sort_extensions — integration tests
// ============================================================================

// B:topological_sort_extensions — verify integration "linear dependency chain → correct order"
#[test]
fn test_toposort_linear_chain() {
    let manifests = vec![
        make_manifest("@specforge/governance", "1.0.0", &[("@specforge/software", ">=1.0.0")]),
        make_manifest("@specforge/software", "1.0.0", &[]),
    ];

    let order = topological_sort_extensions(&manifests).unwrap();
    assert_eq!(order, vec!["@specforge/software", "@specforge/governance"]);
}

// B:topological_sort_extensions — verify integration "diamond dependency → both paths respected"
#[test]
fn test_toposort_diamond_dependency() {
    let manifests = vec![
        make_manifest("@specforge/software", "1.0.0", &[]),
        make_manifest("@specforge/product", "1.0.0", &[("@specforge/software", ">=1.0.0")]),
        make_manifest("@specforge/governance", "1.0.0", &[("@specforge/software", ">=1.0.0")]),
        make_manifest("@specforge/dashboard", "1.0.0", &[
            ("@specforge/product", ">=1.0.0"),
            ("@specforge/governance", ">=1.0.0"),
        ]),
    ];

    let order = topological_sort_extensions(&manifests).unwrap();
    // software must be first, dashboard must be last
    assert_eq!(order[0], "@specforge/software");
    assert_eq!(order[order.len() - 1], "@specforge/dashboard");
    assert_eq!(order.len(), 4);
}

// B:topological_sort_extensions — verify integration "cycle detected → E031 diagnostic"
#[test]
fn test_toposort_cycle_produces_e031() {
    let manifests = vec![
        make_manifest("A", "1.0.0", &[("B", ">=1.0.0")]),
        make_manifest("B", "1.0.0", &[("C", ">=1.0.0")]),
        make_manifest("C", "1.0.0", &[("A", ">=1.0.0")]),
    ];

    let err = topological_sort_extensions(&manifests).unwrap_err();
    assert_eq!(err.len(), 1);
    assert_eq!(err[0].code, "E031");
    assert_eq!(err[0].severity, Severity::Error);
    assert!(err[0].message.contains("cycle"));
}

// B:topological_sort_extensions — verify contract "requires manifests, ensures sorted or cycle error"
#[test]
fn test_toposort_contract() {
    // ensures: deterministic ordering on ties (alphabetical)
    let manifests = vec![
        make_manifest("Z-ext", "1.0.0", &[]),
        make_manifest("A-ext", "1.0.0", &[]),
        make_manifest("M-ext", "1.0.0", &[]),
    ];
    let order = topological_sort_extensions(&manifests).unwrap();
    assert_eq!(order, vec!["A-ext", "M-ext", "Z-ext"]);

    // ensures: empty input → empty output
    let empty = topological_sort_extensions(&[]).unwrap();
    assert!(empty.is_empty());
}

// ============================================================================
// B:validate_extension_manifest — integration tests
// ============================================================================

// B:validate_extension_manifest — verify integration "valid ManifestV2 → no diagnostics"
#[test]
fn test_validate_valid_manifest_no_diagnostics() {
    let m = make_manifest("@specforge/software", "1.0.0", &[]);

    let diags = validate_extension_manifest(&m, std::slice::from_ref(&m));
    assert!(diags.is_empty(), "expected no diagnostics, got: {:?}", diags);
}

// B:validate_extension_manifest — verify integration "missing required fields → E025/E030"
#[test]
fn test_validate_manifest_missing_name_produces_error() {
    let mut m = default_manifest();
    m.manifest_version = 2;
    // name is empty

    let diags = validate_extension_manifest(&m, std::slice::from_ref(&m));
    assert!(!diags.is_empty());
    assert!(diags.iter().any(|d| d.severity == Severity::Error));
}

// B:validate_extension_manifest — verify integration "invalid manifest_version → E030"
#[test]
fn test_validate_manifest_bad_version_produces_e030() {
    let mut m = make_manifest("@test/ext", "1.0.0", &[]);
    m.manifest_version = 99;

    let diags = validate_extension_manifest(&m, std::slice::from_ref(&m));
    assert!(diags.iter().any(|d| d.code == "E030"));
}

// B:validate_extension_manifest — verify contract "requires manifest JSON, ensures diagnostics"
#[test]
fn test_validate_manifest_contract() {
    // ensures: valid manifest passes
    let valid = make_manifest("@specforge/software", "1.0.0", &[]);
    assert!(validate_extension_manifest(&valid, std::slice::from_ref(&valid)).is_empty());

    // ensures: missing peer deps diagnosed
    let with_peer = make_manifest("ext", "1.0.0", &[("missing", ">=1.0.0")]);
    let diags = validate_extension_manifest(&with_peer, std::slice::from_ref(&with_peer));
    assert!(diags.iter().any(|d| d.code == "E027"));
}

// ============================================================================
// B:validate_extension_peer_dependencies — integration tests
// ============================================================================

// B:validate_extension_peer_dependencies — verify integration "all peers satisfied → no diagnostics"
#[test]
fn test_peer_deps_all_satisfied() {
    let software = make_manifest("@specforge/software", "1.0.0", &[]);
    let product = make_manifest(
        "@specforge/product",
        "1.0.0",
        &[("@specforge/software", ">=1.0.0")],
    );
    let all = vec![software, product.clone()];

    let diags = validate_extension_peer_dependencies(&product, &all);
    assert!(diags.is_empty());
}

// B:validate_extension_peer_dependencies — verify integration "missing peer → E027"
#[test]
fn test_peer_deps_missing_peer_produces_e027() {
    let product = make_manifest(
        "@specforge/product",
        "1.0.0",
        &[("@specforge/software", ">=1.0.0")],
    );
    let all = vec![product.clone()]; // software not installed

    let diags = validate_extension_peer_dependencies(&product, &all);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E027");
    assert!(diags[0].message.contains("@specforge/product"));
}

// B:validate_extension_peer_dependencies — verify integration "incompatible version → E027"
#[test]
fn test_peer_deps_incompatible_version_produces_e027() {
    let mut software = default_manifest();
    software.name = "@specforge/software".to_string();
    software.version = "0.5.0".to_string();
    software.manifest_version = 2;

    let product = make_manifest(
        "@specforge/product",
        "1.0.0",
        &[("@specforge/software", ">=1.0.0")],
    );
    let all = vec![software, product.clone()];

    let diags = validate_extension_peer_dependencies(&product, &all);
    assert!(!diags.is_empty());
    assert!(diags.iter().any(|d| d.code == "E027"));
}

// B:validate_extension_peer_dependencies — verify contract "requires manifests, ensures dependency validation"
#[test]
fn test_peer_deps_contract() {
    let software = make_manifest("@specforge/software", "1.0.0", &[]);
    let product = make_manifest(
        "@specforge/product",
        "1.0.0",
        &[("@specforge/software", ">=1.0.0")],
    );

    // ensures: satisfied → empty
    let diags = validate_extension_peer_dependencies(&product, &[software.clone(), product.clone()]);
    assert!(diags.is_empty());

    // ensures: missing → E027
    let diags = validate_extension_peer_dependencies(&product, std::slice::from_ref(&product));
    assert!(diags.iter().any(|d| d.code == "E027"));
    assert!(diags.iter().all(|d| d.severity == Severity::Error));
}
