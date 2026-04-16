use specforge_emitter::builtins::{
    FormalExtension, GovernanceExtension, ProductExtension, SoftwareExtension,
};
use specforge_registry::{validate_manifest, validate_manifest_consistency, ManifestV2};
use specforge_wasm::builtin::BuiltinExtension;
use specforge_wasm::protocol::{
    load_protocol_extension, protocol_extension_to_manifest, ProtocolHost,
};
use specforge_wasm::BuiltinRuntime;

/// Load an extension through the full protocol pipeline:
/// BuiltinRuntime → ProtocolHost → load_protocol_extension → bridge → ManifestV2
fn load_via_protocol(ext_name: &str, ext: Box<dyn BuiltinExtension>) -> ManifestV2 {
    let runtime = BuiltinRuntime::new().with_extension(ext_name, ext);
    let host = ProtocolHost::new(&runtime);
    let proto_ext = load_protocol_extension(&host, ext_name).unwrap();
    protocol_extension_to_manifest(&proto_ext)
}

#[test]
fn product_extension_loads_via_protocol() {
    let manifest = load_via_protocol("@specforge/product", Box::new(ProductExtension));
    assert_eq!(manifest.name, "@specforge/product");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.entity_kinds.len(), 9);
    assert_eq!(manifest.edge_types.len(), 20);
    assert_eq!(manifest.validation_rules.len(), 24);
    assert_eq!(manifest.fields.len(), 1, "shared fields (tags)");
    assert!(manifest.contributes.entities);
    assert!(manifest.contributes.validators);

    let diags = validate_manifest(&manifest);
    assert!(diags.is_empty(), "schema validation errors: {:?}", diags);
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.is_empty(), "consistency errors: {:?}", diags);
}

#[test]
fn governance_extension_loads_via_protocol() {
    let manifest = load_via_protocol("@specforge/governance", Box::new(GovernanceExtension));
    assert_eq!(manifest.name, "@specforge/governance");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.entity_kinds.len(), 3);
    assert_eq!(manifest.edge_types.len(), 11);
    assert_eq!(manifest.validation_rules.len(), 7);
    assert_eq!(manifest.peer_dependencies.len(), 1);
    assert!(manifest.contributes.entities);
    assert!(manifest.contributes.validators);

    let diags = validate_manifest(&manifest);
    assert!(diags.is_empty(), "schema validation errors: {:?}", diags);
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.is_empty(), "consistency errors: {:?}", diags);
}

#[test]
fn software_extension_loads_via_protocol() {
    let manifest = load_via_protocol("@specforge/software", Box::new(SoftwareExtension));
    assert_eq!(manifest.name, "@specforge/software");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.entity_kinds.len(), 5);
    assert_eq!(manifest.edge_types.len(), 14);
    assert_eq!(manifest.validation_rules.len(), 12);
    assert_eq!(manifest.entity_enhancements.len(), 2);
    assert_eq!(manifest.peer_dependencies.len(), 1);
    assert!(manifest.sandbox_policy.is_some());
    assert!(manifest.contributes.entities);
    assert!(manifest.contributes.validators);

    let diags = validate_manifest(&manifest);
    assert!(diags.is_empty(), "schema validation errors: {:?}", diags);
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.is_empty(), "consistency errors: {:?}", diags);
}

#[test]
fn formal_extension_loads_via_protocol() {
    let manifest = load_via_protocol("@specforge/formal", Box::new(FormalExtension));
    assert_eq!(manifest.name, "@specforge/formal");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.entity_kinds.len(), 5);
    assert_eq!(manifest.edge_types.len(), 12);
    assert_eq!(manifest.validation_rules.len(), 1);
    assert_eq!(manifest.entity_enhancements.len(), 2);
    assert_eq!(manifest.peer_dependencies.len(), 1);
    assert!(manifest.contributes.entities);
    assert!(manifest.contributes.validators);

    let diags = validate_manifest(&manifest);
    assert!(diags.is_empty(), "schema validation errors: {:?}", diags);
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.is_empty(), "consistency errors: {:?}", diags);
}
