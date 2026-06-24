use std::path::{Path, PathBuf};

use specforge_extism::{CompositeRuntime, ExtismRuntime};
use specforge_wasm::builtin::{BuiltinExtension, BuiltinRuntime};
use specforge_wasm::protocol::{
    ContributionFlags, DescribeResponse, HandshakeResponse, ProtocolHost, load_protocol_extension,
    protocol_extension_to_manifest,
};

struct MinimalBuiltin;

impl BuiltinExtension for MinimalBuiltin {
    fn handshake(&self) -> HandshakeResponse {
        HandshakeResponse {
            protocol_version: "1.0.0".to_string(),
            name: "@specforge/minimal".to_string(),
            version: "1.0.0".to_string(),
            contribution_flags: ContributionFlags {
                entities: true,
                ..Default::default()
            },
            peer_dependencies: vec![],
            sandbox_policy: None,
        }
    }

    fn describe(&self, category: &str) -> Option<DescribeResponse> {
        let items = match category {
            "entities" => serde_json::json!([{
                "name": "builtin_kind",
                "fields": [],
                "testable": false,
                "singleton": false,
                "supports_verify": false
            }]),
            _ => serde_json::json!([]),
        };
        Some(DescribeResponse {
            category: category.to_string(),
            items,
        })
    }
}

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

// --- End-to-end: CompositeRuntime loads both builtin and Wasm extensions via protocol ---

#[test]
fn protocol_host_loads_builtin_through_composite() {
    let builtin = BuiltinRuntime::new()
        .with_extension("@specforge/minimal", Box::new(MinimalBuiltin));
    let extism = ExtismRuntime::new();
    let runtime = CompositeRuntime::new(builtin, extism);

    let host = ProtocolHost::new(&runtime);
    let proto_ext = load_protocol_extension(&host, "@specforge/minimal").unwrap();
    let manifest = protocol_extension_to_manifest(&proto_ext);

    assert_eq!(manifest.name, "@specforge/minimal");
    assert_eq!(manifest.entity_kinds.len(), 1);
    assert_eq!(manifest.entity_kinds[0].name, "builtin_kind");
}

#[test]
fn protocol_host_loads_wasm_through_composite() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }

    let builtin = BuiltinRuntime::new()
        .with_extension("@specforge/minimal", Box::new(MinimalBuiltin));
    let extism = ExtismRuntime::new();
    let runtime = CompositeRuntime::new(builtin, extism);

    // Load the Wasm extension under a canonical name
    runtime
        .load_wasm_extension("@test/hello", &fixture_wasm_path())
        .unwrap();

    let host = ProtocolHost::new(&runtime);

    // Both builtin and Wasm extensions are accessible via protocol
    let builtin_ext = load_protocol_extension(&host, "@specforge/minimal").unwrap();
    assert_eq!(builtin_ext.handshake.name, "@specforge/minimal");

    let wasm_ext = load_protocol_extension(&host, "@test/hello").unwrap();
    assert_eq!(wasm_ext.handshake.name, "@test/hello");

    // Wasm extension contributes entity kinds
    let wasm_manifest = protocol_extension_to_manifest(&wasm_ext);
    assert_eq!(wasm_manifest.entity_kinds.len(), 1);
    assert_eq!(wasm_manifest.entity_kinds[0].name, "widget");
}

#[test]
fn composite_runtime_merges_registries_from_both_sources() {
    if !has_fixture() {
        eprintln!("SKIP: test fixture not built");
        return;
    }

    use specforge_registry::populate_registries;

    let builtin = BuiltinRuntime::new()
        .with_extension("@specforge/minimal", Box::new(MinimalBuiltin));
    let extism = ExtismRuntime::new();
    let runtime = CompositeRuntime::new(builtin, extism);

    runtime
        .load_wasm_extension("@test/hello", &fixture_wasm_path())
        .unwrap();

    let host = ProtocolHost::new(&runtime);

    // Load both extensions via protocol
    let ext1 = load_protocol_extension(&host, "@specforge/minimal").unwrap();
    let ext2 = load_protocol_extension(&host, "@test/hello").unwrap();

    let manifests = vec![
        protocol_extension_to_manifest(&ext1),
        protocol_extension_to_manifest(&ext2),
    ];

    let (kind_reg, _field_reg, _edge_reg, diags) = populate_registries(&manifests);

    // No errors in registry population
    assert!(
        diags.iter().all(|d| d.severity != specforge_common::Severity::Error),
        "Unexpected errors: {:?}",
        diags
    );

    // Both entity kinds are registered
    let keywords: Vec<String> = kind_reg.keywords().cloned().collect();
    assert!(keywords.contains(&"builtin_kind".to_string()), "Missing builtin_kind in {:?}", keywords);
    assert!(keywords.contains(&"widget".to_string()), "Missing widget in {:?}", keywords);
}
