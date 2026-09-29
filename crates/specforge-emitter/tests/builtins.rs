use specforge_registry::{ManifestV2, validate_manifest, validate_manifest_consistency};
use specforge_wasm::WasmRuntime;
use specforge_wasm::protocol::{
    ProtocolHost, load_protocol_extension, protocol_extension_to_manifest,
};

/// Build a Wasm runtime for a temp project listing `ext_names` — the only
/// way extensions exist now (WASM-only migration, Phase 7: the native
/// mirror tier is gone).
fn wasm_runtime_for(ext_names: &[&str]) -> specforge_component::ComponentRuntime {
    let dir = tempfile::TempDir::new().unwrap();
    let config = serde_json::json!({
        "name": "test-project",
        "version": "0.1.0",
        "extensions": ext_names,
    });
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    specforge_component::project_runtime(dir.path())
}

/// Load an extension through the full protocol pipeline over its real Wasm
/// blob: project_runtime → ProtocolHost → load_protocol_extension → bridge
/// → ManifestV2.
fn load_via_protocol(ext_name: &str) -> ManifestV2 {
    let runtime = wasm_runtime_for(&[ext_name]);
    let host = ProtocolHost::new(&runtime);
    let proto_ext = load_protocol_extension(&host, ext_name).unwrap();
    protocol_extension_to_manifest(&proto_ext)
}

#[test]
fn product_extension_loads_via_protocol() {
    let manifest = load_via_protocol("@specforge/product");
    assert_eq!(manifest.name, "@specforge/product");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.entity_kinds.len(), 9);
    assert_eq!(manifest.edge_types.len(), 20);
    assert_eq!(manifest.validation_rules.len(), 30);
    assert_eq!(manifest.fields.len(), 1, "shared fields (tags)");
    assert!(manifest.contributes.entities);
    assert!(manifest.contributes.validators);

    let diags = validate_manifest(&manifest);
    assert!(diags.is_empty(), "schema validation errors: {:?}", diags);
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.is_empty(), "consistency errors: {:?}", diags);
}

#[test]
fn product_w093_semver_pattern_is_not_trivial() {
    let manifest = load_via_protocol("@specforge/product");
    let w093 = manifest
        .validation_rules
        .iter()
        .find(|r| r.code == "W093")
        .expect("W093 rule must exist");
    let pattern = w093.constraint.as_ref().unwrap().pattern.as_ref().unwrap();
    assert!(
        pattern.len() > 5,
        "W093 pattern '{}' is too trivial to validate semver",
        pattern
    );
    assert!(
        pattern.contains(r"\d") || pattern.contains("[0-9]"),
        "W093 pattern '{}' must match digits for semver validation",
        pattern
    );
}

#[test]
fn governance_extension_loads_via_protocol() {
    let manifest = load_via_protocol("@specforge/governance");
    assert_eq!(manifest.name, "@specforge/governance");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.entity_kinds.len(), 3);
    assert_eq!(manifest.edge_types.len(), 11);
    assert_eq!(manifest.validation_rules.len(), 7);
    assert_eq!(
        manifest.peer_dependencies.len(),
        2,
        "governance should depend on software AND product"
    );
    let dep_names: Vec<&str> = manifest
        .peer_dependencies
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert!(
        dep_names.contains(&"@specforge/software"),
        "must depend on software"
    );
    assert!(
        dep_names.contains(&"@specforge/product"),
        "must depend on product (declares edges to feature)"
    );
    assert!(manifest.contributes.entities);
    assert!(manifest.contributes.validators);

    let diags = validate_manifest(&manifest);
    assert!(diags.is_empty(), "schema validation errors: {:?}", diags);
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.is_empty(), "consistency errors: {:?}", diags);
}

#[specforge_test_macros::test(
    behavior = "vt_declare_vitest_collector",
    verify = "vitest declares its collector"
)]
fn vitest_declares_its_collector() {
    let manifest = load_via_protocol("@specforge/vitest");
    assert!(
        manifest
            .peer_dependencies
            .iter()
            .any(|p| p.name == "@specforge/testing" && !p.optional),
        "testing is a required peer"
    );
    let collectors = specforge_emitter::collect::collectors(std::slice::from_ref(&manifest));
    assert_eq!(collectors.len(), 1);
    let c = &collectors[0];
    assert_eq!(c.name, "vitest");
    assert_eq!(c.export, "collect__vitest");
    assert_eq!(c.detect, ["vitest.config.*", "vitest.workspace.*"]);
    assert_eq!(&c.run[..3], ["npx", "--no", "vitest"]);
    assert!(c.run.iter().any(|a| a == "--outputFile.json={report}"));
    assert_eq!(c.report, ".specforge/reports/vitest.json");
}

#[specforge_test_macros::test(
    behavior = "ct_declare_cargo_collector",
    verify = "cargo-test declares its collector"
)]
fn cargo_test_declares_its_collector() {
    let manifest = load_via_protocol("@specforge/cargo-test");
    assert!(
        manifest
            .peer_dependencies
            .iter()
            .any(|p| p.name == "@specforge/testing" && !p.optional),
        "testing is a required peer"
    );
    let collectors = specforge_emitter::collect::collectors(std::slice::from_ref(&manifest));
    assert_eq!(collectors.len(), 1);
    let c = &collectors[0];
    assert_eq!(c.name, "cargo-test");
    assert_eq!(c.export, "collect__cargo_test");
    assert_eq!(c.detect, ["Cargo.toml"]);
    assert_eq!(c.run, ["cargo", "test", "--workspace", "--no-fail-fast"]);
    assert_eq!(c.report, "target/specforge");
}

#[test]
fn testing_extension_makes_software_kinds_testable() {
    let manifest = load_via_protocol("@specforge/testing");
    assert_eq!(manifest.name, "@specforge/testing");
    assert!(manifest.entity_kinds.is_empty(), "testing owns no kinds");
    let testable: Vec<(&str, &str)> = manifest
        .entity_enhancements
        .iter()
        .filter(|e| e.verify_kinds.is_some())
        .map(|e| (e.target_kind.as_str(), e.source_extension.as_str()))
        .collect();
    assert_eq!(
        testable,
        [
            ("behavior", "@specforge/software"),
            ("invariant", "@specforge/software"),
            ("event", "@specforge/software"),
            ("type", "@specforge/software"),
            ("port", "@specforge/software"),
            ("constraint", "@specforge/governance"),
            ("failure_mode", "@specforge/governance"),
        ]
    );
    let codes: std::collections::BTreeSet<&str> = manifest
        .validation_rules
        .iter()
        .map(|r| r.code.as_str())
        .collect();
    assert_eq!(codes, ["W004", "W009"].into_iter().collect());
}

#[specforge_test_macros::test(
    behavior = "se_declare_manifest",
    verify = "the only peer is @specforge/product, and it is optional"
)]
fn software_extension_loads_via_protocol() {
    let manifest = load_via_protocol("@specforge/software");
    assert_eq!(manifest.name, "@specforge/software");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.entity_kinds.len(), 5);
    assert_eq!(manifest.edge_types.len(), 14);
    // W004/W009 moved to @specforge/testing (ADR 0002).
    assert_eq!(manifest.validation_rules.len(), 11);
    assert_eq!(manifest.entity_enhancements.len(), 2);
    assert!(
        manifest
            .entity_kinds
            .iter()
            .all(|k| !k.supports_verify && !k.testable),
        "software declares no test vocabulary"
    );
    assert_eq!(manifest.peer_dependencies.len(), 1);
    assert!(
        manifest.peer_dependencies[0].optional,
        "product is an optional peer: software works without it"
    );
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
    let manifest = load_via_protocol("@specforge/formal");
    assert_eq!(manifest.name, "@specforge/formal");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.entity_kinds.len(), 5);
    assert_eq!(manifest.edge_types.len(), 13);
    assert!(
        manifest.validation_rules.len() >= 6,
        "formal needs at least 6 validation rules, got {}",
        manifest.validation_rules.len()
    );
    assert_eq!(manifest.entity_enhancements.len(), 2);
    assert_eq!(manifest.peer_dependencies.len(), 1);
    assert!(manifest.contributes.entities);
    assert!(manifest.contributes.validators);

    let property = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "property")
        .unwrap();
    assert!(
        property.supports_verify,
        "property should support verify (model-checkable)"
    );
    let axiom = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "axiom")
        .unwrap();
    assert!(
        axiom.supports_verify,
        "axiom should support verify (proof-checkable)"
    );

    let refinement = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "refinement")
        .unwrap();
    let abstract_f = refinement
        .fields
        .iter()
        .find(|f| f.name == "abstract_entity")
        .unwrap();
    assert!(
        abstract_f.required,
        "refinement.abstract_entity should be required"
    );
    let concrete_f = refinement
        .fields
        .iter()
        .find(|f| f.name == "concrete_entity")
        .unwrap();
    assert!(
        concrete_f.required,
        "refinement.concrete_entity should be required"
    );

    let diags = validate_manifest(&manifest);
    assert!(diags.is_empty(), "schema validation errors: {:?}", diags);
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.is_empty(), "consistency errors: {:?}", diags);
}

// ── @specforge/rust ──

#[test]
fn rust_extension_loads_via_protocol() {
    let manifest = load_via_protocol("@specforge/rust");
    assert_eq!(manifest.name, "@specforge/rust");
    assert_eq!(manifest.version, "1.0.0");
    assert!(manifest.contributes.analyzers);
    assert!(!manifest.contributes.entities);
    assert_eq!(manifest.analyzer_contributions.len(), 1);
    let ac = &manifest.analyzer_contributions[0];
    assert_eq!(ac.language, "rust");
    assert_eq!(ac.file_extensions, vec![".rs"]);
    assert_eq!(ac.scan_export, "scan__rust");

    let diags = validate_manifest(&manifest);
    assert!(diags.is_empty(), "schema validation errors: {:?}", diags);
}

#[test]
fn rust_scanner_produces_same_items_as_fallback() {
    use specforge_wasm::protocol::{ScanRequest, ScanResponse};

    let source = r#"pub fn hello() {}
pub struct MyConfig {}
pub enum Status {}
pub trait Drawable {}
pub async fn fetch_data() {}
fn private() {}
// pub fn commented() {}
pub const MAX_SIZE: usize = 100;
"#;

    let runtime = wasm_runtime_for(&["@specforge/rust"]);
    let input = serde_json::to_vec(&ScanRequest {
        file_path: "src/lib.rs".into(),
        content: source.into(),
    })
    .unwrap();

    let result = runtime.call_export("@specforge/rust", "scan__rust", &input);
    let bytes = match result {
        specforge_wasm::runtime::WasmCallResult::Ok(b) => b,
        specforge_wasm::runtime::WasmCallResult::Trap(t) => panic!("scan trapped: {:?}", t),
    };
    let resp: ScanResponse = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(resp.items.len(), 6);
    assert_eq!(resp.items[0].name, "hello");
    assert_eq!(resp.items[0].item_kind, "function");
    assert_eq!(resp.items[1].name, "MyConfig");
    assert_eq!(resp.items[1].item_kind, "struct");
    assert_eq!(resp.items[2].name, "Status");
    assert_eq!(resp.items[2].item_kind, "enum");
    assert_eq!(resp.items[3].name, "Drawable");
    assert_eq!(resp.items[3].item_kind, "trait");
    assert_eq!(resp.items[4].name, "fetch_data");
    assert_eq!(resp.items[4].item_kind, "function");
    assert_eq!(resp.items[5].name, "MAX_SIZE");
    assert_eq!(resp.items[5].item_kind, "constant");
}

// ── @specforge/typescript ──

#[test]
fn typescript_extension_loads_via_protocol() {
    let manifest = load_via_protocol("@specforge/typescript");
    assert_eq!(manifest.name, "@specforge/typescript");
    assert_eq!(manifest.version, "1.0.0");
    assert!(manifest.contributes.analyzers);
    assert!(!manifest.contributes.entities);
    assert_eq!(manifest.analyzer_contributions.len(), 1);
    let ac = &manifest.analyzer_contributions[0];
    assert_eq!(ac.language, "typescript");
    assert_eq!(ac.file_extensions, vec![".ts", ".tsx", ".js", ".jsx"]);
    assert_eq!(ac.scan_export, "scan__typescript");

    let diags = validate_manifest(&manifest);
    assert!(diags.is_empty(), "schema validation errors: {:?}", diags);
}

#[test]
fn typescript_scanner_finds_exported_symbols() {
    use specforge_wasm::protocol::{ScanRequest, ScanResponse};

    let source = r#"export function handleRequest() {}
export async function fetchData() {}
export class UserService {}
export interface IRepository {}
export type Config = Record<string, unknown>;
export enum Status { Active, Inactive }
export const MAX_RETRIES = 3;
export let counter = 0;
export abstract class Base {}
export default function main() {}
function privateHelper() {}
// export function commented() {}
export { something } from './other';
export * from './barrel';
"#;

    let runtime = wasm_runtime_for(&["@specforge/typescript"]);
    let input = serde_json::to_vec(&ScanRequest {
        file_path: "src/app.ts".into(),
        content: source.into(),
    })
    .unwrap();

    let result = runtime.call_export("@specforge/typescript", "scan__typescript", &input);
    let bytes = match result {
        specforge_wasm::runtime::WasmCallResult::Ok(b) => b,
        specforge_wasm::runtime::WasmCallResult::Trap(t) => panic!("scan trapped: {:?}", t),
    };
    let resp: ScanResponse = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(resp.items.len(), 10);
    assert_eq!(resp.items[0].name, "handleRequest");
    assert_eq!(resp.items[0].item_kind, "function");
    assert_eq!(resp.items[1].name, "fetchData");
    assert_eq!(resp.items[1].item_kind, "function");
    assert_eq!(resp.items[2].name, "UserService");
    assert_eq!(resp.items[2].item_kind, "class");
    assert_eq!(resp.items[3].name, "IRepository");
    assert_eq!(resp.items[3].item_kind, "interface");
    assert_eq!(resp.items[4].name, "Config");
    assert_eq!(resp.items[4].item_kind, "type_alias");
    assert_eq!(resp.items[5].name, "Status");
    assert_eq!(resp.items[5].item_kind, "enum");
    assert_eq!(resp.items[6].name, "MAX_RETRIES");
    assert_eq!(resp.items[6].item_kind, "constant");
    assert_eq!(resp.items[7].name, "counter");
    assert_eq!(resp.items[7].item_kind, "variable");
    assert_eq!(resp.items[8].name, "Base");
    assert_eq!(resp.items[8].item_kind, "class");
    assert_eq!(resp.items[9].name, "main");
    assert_eq!(resp.items[9].item_kind, "function");
    assert_eq!(resp.items[9].visibility.as_deref(), Some("default"));
}
