//! End-to-end: one `ExtismRuntime` hosts BOTH an embedded builtin blob and a
//! third-party `.wasm` extension, and both load through the protocol pipeline
//! (WASM-only migration, Phase 7: the composite tier is gone — there is a
//! single runtime).

use std::path::{Path, PathBuf};

use specforge_extism::{ExtismRuntime, builtins};
use specforge_wasm::protocol::{
    ProtocolHost, load_protocol_extension, protocol_extension_to_manifest,
};

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

/// One runtime with the four embedded builtin blobs loaded.
fn runtime_with_builtins() -> ExtismRuntime {
    let runtime = ExtismRuntime::new();
    builtins::load_builtins(&runtime).expect("builtin blobs load");
    runtime
}

#[test]
fn protocol_host_loads_builtin_blob() {
    let runtime = runtime_with_builtins();
    let host = ProtocolHost::new(&runtime);
    let proto_ext = load_protocol_extension(&host, "@specforge/product").unwrap();
    let manifest = protocol_extension_to_manifest(&proto_ext);

    assert_eq!(manifest.name, "@specforge/product");
    assert!(!manifest.entity_kinds.is_empty());
}

#[test]
fn protocol_host_loads_wasm_alongside_builtin() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }

    let runtime = runtime_with_builtins();

    // Load the third-party Wasm extension under a canonical name
    runtime
        .load_module_as("@test/hello", &fixture_wasm_path(), None)
        .unwrap();

    let host = ProtocolHost::new(&runtime);

    // Both the builtin blob and the Wasm extension are accessible via protocol
    let builtin_ext = load_protocol_extension(&host, "@specforge/product").unwrap();
    assert_eq!(builtin_ext.handshake.name, "@specforge/product");

    let wasm_ext = load_protocol_extension(&host, "@test/hello").unwrap();
    assert_eq!(wasm_ext.handshake.name, "@test/hello");

    // The Wasm extension contributes entity kinds
    let wasm_manifest = protocol_extension_to_manifest(&wasm_ext);
    assert_eq!(wasm_manifest.entity_kinds.len(), 1);
    assert_eq!(wasm_manifest.entity_kinds[0].name, "widget");
}

#[test]
fn single_runtime_merges_registries_from_both_sources() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }

    use specforge_registry::populate_registries;

    let runtime = runtime_with_builtins();
    runtime
        .load_module_as("@test/hello", &fixture_wasm_path(), None)
        .unwrap();

    let host = ProtocolHost::new(&runtime);

    let ext1 = load_protocol_extension(&host, "@specforge/product").unwrap();
    let ext2 = load_protocol_extension(&host, "@test/hello").unwrap();

    let manifests = vec![
        protocol_extension_to_manifest(&ext1),
        protocol_extension_to_manifest(&ext2),
    ];

    let (kind_reg, _field_reg, _edge_reg, diags) = populate_registries(&manifests);

    assert!(
        diags
            .iter()
            .all(|d| d.severity != specforge_common::Severity::Error),
        "Unexpected errors: {:?}",
        diags
    );

    let keywords: Vec<String> = kind_reg.keywords().cloned().collect();
    assert!(!keywords.is_empty());
    assert!(
        keywords.contains(&"widget".to_string()),
        "Missing widget in {:?}",
        keywords
    );
}
