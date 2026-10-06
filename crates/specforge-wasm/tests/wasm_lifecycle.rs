// Wasm lifecycle integration tests through the public API:
// - B:load_wasm_module (with the lock file's hash pin)
// - B:topological_sort_extensions

use specforge_common::Severity;
use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta};
use specforge_protocol_types::PeerDependency;
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{LockFile, load_wasm_module, topological_sort_extensions};
use std::path::Path;
use tempfile::TempDir;

/// A runtime serving the extensions these tests load, so loading their
/// binaries under those names succeeds.
fn runtime() -> InProcessRuntime {
    let serve =
        |name: &'static str| move || ContributionsBuilder::new(ExtensionMeta::new(name, "1.0.0"));
    InProcessRuntime::new()
        .with(serve("@test/ext"))
        .with(serve("@test/legacy"))
        .with(serve("@test/local"))
}

fn make_declaration(
    name: &str,
    version: &str,
    peers: &[(&str, &str)],
) -> specforge_protocol_types::ExtensionDeclaration {
    specforge_protocol_types::ExtensionDeclaration {
        handshake: specforge_protocol_types::HandshakeResponse {
            name: name.to_string(),
            version: version.to_string(),
            peer_dependencies: peers
                .iter()
                .map(|(n, v)| PeerDependency {
                    name: n.to_string(),
                    version: v.to_string(),
                    optional: false,
                })
                .collect(),
            ..Default::default()
        },
        ..Default::default()
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

// B:load_wasm_module — verify integration "load valid module bytes → Ok"
#[test]
fn test_load_valid_module_loads() {
    let dir = TempDir::new().unwrap();
    let wasm_path = create_fake_wasm(&dir, "ext.wasm");
    let runtime = runtime();

    load_wasm_module("@test/ext", &wasm_path, &runtime, None).unwrap();
}

// B:load_wasm_module — verify integration "load corrupted bytes → Err with E028"
#[test]
fn test_load_missing_wasm_returns_e028() {
    let runtime = runtime();
    let missing = Path::new("/nonexistent/path/ext.wasm");

    let err = load_wasm_module("@test/missing", missing, &runtime, None).unwrap_err();
    assert_eq!(err.severity, Severity::Error);
    assert!(err.message.contains("not found"));
}

// B:load_wasm_module — verify contract "requires valid bytes, ensures a load or a diagnostic"
#[test]
fn test_load_wasm_module_contract() {
    let dir = TempDir::new().unwrap();
    let wasm_path = create_fake_wasm(&dir, "ext.wasm");
    let runtime = runtime();

    // ensures: success path loads it
    load_wasm_module("@test/ext", &wasm_path, &runtime, None).unwrap();

    // ensures: failure path returns E028 diagnostic
    let err = load_wasm_module("bad", Path::new("/no/such.wasm"), &runtime, None).unwrap_err();
    assert_eq!(err.code, "E028");
    assert_eq!(err.severity, Severity::Error);
}

// ============================================================================
// B:topological_sort_extensions — integration tests
// ============================================================================

// B:topological_sort_extensions — verify integration "linear dependency chain → correct order"
#[test]
fn test_toposort_linear_chain() {
    let manifests = vec![
        make_declaration(
            "@specforge/governance",
            "1.0.0",
            &[("@specforge/software", ">=1.0.0")],
        ),
        make_declaration("@specforge/software", "1.0.0", &[]),
    ];

    let order = topological_sort_extensions(&manifests).unwrap();
    assert_eq!(order, vec!["@specforge/software", "@specforge/governance"]);
}

// B:topological_sort_extensions — verify integration "diamond dependency → both paths respected"
#[test]
fn test_toposort_diamond_dependency() {
    let manifests = vec![
        make_declaration("@specforge/software", "1.0.0", &[]),
        make_declaration(
            "@specforge/product",
            "1.0.0",
            &[("@specforge/software", ">=1.0.0")],
        ),
        make_declaration(
            "@specforge/governance",
            "1.0.0",
            &[("@specforge/software", ">=1.0.0")],
        ),
        make_declaration(
            "@specforge/dashboard",
            "1.0.0",
            &[
                ("@specforge/product", ">=1.0.0"),
                ("@specforge/governance", ">=1.0.0"),
            ],
        ),
    ];

    let order = topological_sort_extensions(&manifests).unwrap();
    // software must be first, dashboard must be last
    assert_eq!(order[0], "@specforge/software");
    assert_eq!(order[order.len() - 1], "@specforge/dashboard");
    assert_eq!(order.len(), 4);
}

// B:topological_sort_extensions — verify integration "cycle detected → E031 diagnostic"
#[test]
fn test_toposort_cycle_produces_e027() {
    let manifests = vec![
        make_declaration("A", "1.0.0", &[("B", ">=1.0.0")]),
        make_declaration("B", "1.0.0", &[("C", ">=1.0.0")]),
        make_declaration("C", "1.0.0", &[("A", ">=1.0.0")]),
    ];

    let err = topological_sort_extensions(&manifests).unwrap_err();
    assert_eq!(err.len(), 1);
    assert_eq!(err[0].code, "E027");
    assert_eq!(err[0].severity, Severity::Error);
    assert!(err[0].message.contains("cycle"));
}

// B:topological_sort_extensions — verify contract "requires manifests, ensures sorted or cycle error"
#[test]
fn test_toposort_contract() {
    // ensures: deterministic ordering on ties (alphabetical)
    let manifests = vec![
        make_declaration("Z-ext", "1.0.0", &[]),
        make_declaration("A-ext", "1.0.0", &[]),
        make_declaration("M-ext", "1.0.0", &[]),
    ];
    let order = topological_sort_extensions(&manifests).unwrap();
    assert_eq!(order, vec!["A-ext", "M-ext", "Z-ext"]);

    // ensures: empty input → empty output
    let empty = topological_sort_extensions(&[]).unwrap();
    assert!(empty.is_empty());
}

// B:load_wasm_module — verify unit "lockfile hash pin refuses tampered binary"
#[test]
fn load_refuses_binary_that_differs_from_lockfile_hash() {
    use specforge_wasm::install_extension;

    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    let wasm_bytes = b"\0asm-original";
    let mut lock = LockFile::new();
    install_extension(
        "@test/ext",
        "1.0.0",
        wasm_bytes,
        &specforge_wasm::hex_sha256(wasm_bytes),
        &extensions_dir,
        &mut lock,
        None,
        Vec::new(),
    )
    .unwrap();

    let wasm_path = extensions_dir.join("@test/ext").join("extension.wasm");

    // Load with the recorded hash: succeeds.
    let runtime = runtime();
    load_wasm_module(
        "@test/ext",
        &wasm_path,
        &runtime,
        Some(lock.entries[0].wasm_hash.as_str()),
    )
    .unwrap();

    // Tamper with the installed binary, then load: refused with E033.
    std::fs::write(&wasm_path, b"\0asm-swapped-after-install").unwrap();
    let err = load_wasm_module(
        "@test/ext",
        &wasm_path,
        &runtime,
        Some(lock.entries[0].wasm_hash.as_str()),
    )
    .unwrap_err();
    assert_eq!(err.code, "E033");
    assert!(err.message.contains("integrity mismatch"));
    assert!(err.suggestion.unwrap_or_default().contains("re-install"));
}

// B:load_wasm_module — verify unit "legacy entries without hash load unchanged"
#[test]
fn load_with_empty_or_absent_hash_does_not_fail() {
    let dir = TempDir::new().unwrap();
    let wasm_path = dir.path().join("extension.wasm");
    std::fs::write(&wasm_path, b"\0asm-legacy").unwrap();
    let runtime = runtime();

    // Legacy lockfile entry: empty hash string — warn-and-load, not fail.
    load_wasm_module("@test/legacy", &wasm_path, &runtime, Some("")).unwrap();

    // No hash context at all (local dev load): unchanged behavior.
    load_wasm_module("@test/local", &wasm_path, &runtime, None).unwrap();
}
