use specforge_emitter::builtins::{self, RustExtension};
use specforge_emitter::scanner_dispatch;
use specforge_registry::{AnalyzerContribution, ManifestV2, ExtensionContributions};
use specforge_wasm::builtin::BuiltinRuntime;
use tempfile::TempDir;

fn rust_only_runtime() -> BuiltinRuntime {
    BuiltinRuntime::new().with_extension("@specforge/rust", Box::new(RustExtension))
}

fn rust_manifest() -> ManifestV2 {
    ManifestV2 {
        name: "@specforge/rust".into(),
        version: "1.0.0".into(),
        manifest_version: 2,
        wasm_path: String::new(),
        contributes: ExtensionContributions::default(),
        entity_kinds: vec![],
        edge_types: vec![],
        validation_rules: vec![],
        verify_kinds: vec![],
        fields: vec![],
        incremental: None,
        reserved_keywords: vec![],
        migration_hook: None,
        peer_dependencies: vec![],
        sandbox_policy: None,
        host_api_version: None,
        entity_enhancements: vec![],
        starter_template: None,
        grammar_contributions: vec![],
        body_parser_contributions: vec![],
        ext_short: None,
        query_scope: None,
        collector_contributions: vec![],
        analyzer_contributions: vec![AnalyzerContribution {
            language: "rust".into(),
            file_extensions: vec![".rs".into()],
            excluded_dirs: vec!["target".into()],
            scan_export: "scan__rust".into(),
            classify_export: "classify__rust".into(),
            map_export: "map__rust".into(),
            description: None,
        }],
        surfaces: None,
    }
}

#[test]
fn scan_only_matching_extensions() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("lib.rs"), "pub fn hello() {}").unwrap();
    std::fs::write(dir.path().join("readme.md"), "# Hello").unwrap();
    std::fs::write(dir.path().join("app.txt"), "text file").unwrap();

    let runtime = rust_only_runtime();
    let manifests = vec![rust_manifest()];
    let source_files = vec!["lib.rs".into(), "readme.md".into(), "app.txt".into()];

    let (items, scanners) = scanner_dispatch::scan_source_files(
        &runtime,
        &manifests,
        dir.path(),
        &source_files,
    );

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "hello");
    assert_eq!(items[0].item_kind, "function");
    assert_eq!(items[0].file, "lib.rs");
    assert_eq!(items[0].scanner.as_deref(), Some("@specforge/rust"));
    assert_eq!(scanners, vec!["@specforge/rust"]);
}

#[test]
fn scan_empty_source_list() {
    let dir = TempDir::new().unwrap();
    let runtime = rust_only_runtime();
    let manifests = vec![rust_manifest()];

    let (items, scanners) = scanner_dispatch::scan_source_files(
        &runtime,
        &manifests,
        dir.path(),
        &[],
    );

    assert!(items.is_empty());
    assert!(scanners.is_empty());
}

#[test]
fn scan_no_manifests_skips_all_files() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("lib.rs"), "pub fn hello() {}").unwrap();

    let runtime = rust_only_runtime();
    let source_files = vec!["lib.rs".into()];

    let (items, scanners) = scanner_dispatch::scan_source_files(
        &runtime,
        &[],
        dir.path(),
        &source_files,
    );

    assert!(items.is_empty());
    assert!(scanners.is_empty());
}

#[test]
fn scan_missing_file_skipped_gracefully() {
    let dir = TempDir::new().unwrap();

    let runtime = rust_only_runtime();
    let manifests = vec![rust_manifest()];
    let source_files = vec!["nonexistent.rs".into()];

    let (items, scanners) = scanner_dispatch::scan_source_files(
        &runtime,
        &manifests,
        dir.path(),
        &source_files,
    );

    assert!(items.is_empty());
    assert!(scanners.is_empty());
}

#[test]
fn default_runtime_scans_rust_files() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("main.rs"),
        "pub fn process_order() {}\npub struct Config {}",
    ).unwrap();

    let runtime = builtins::runtime_for_extensions(&["@specforge/rust".into()]);
    let manifests = vec![rust_manifest()];
    let source_files = vec!["main.rs".into()];

    let (items, scanners) = scanner_dispatch::scan_source_files(
        &runtime,
        &manifests,
        dir.path(),
        &source_files,
    );

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].name, "process_order");
    assert_eq!(items[1].name, "Config");
    assert_eq!(scanners, vec!["@specforge/rust"]);
}

fn typescript_manifest() -> ManifestV2 {
    ManifestV2 {
        name: "@specforge/typescript".into(),
        version: "1.0.0".into(),
        manifest_version: 2,
        wasm_path: String::new(),
        contributes: ExtensionContributions::default(),
        entity_kinds: vec![],
        edge_types: vec![],
        validation_rules: vec![],
        verify_kinds: vec![],
        fields: vec![],
        incremental: None,
        reserved_keywords: vec![],
        migration_hook: None,
        peer_dependencies: vec![],
        sandbox_policy: None,
        host_api_version: None,
        entity_enhancements: vec![],
        starter_template: None,
        grammar_contributions: vec![],
        body_parser_contributions: vec![],
        ext_short: None,
        query_scope: None,
        collector_contributions: vec![],
        analyzer_contributions: vec![AnalyzerContribution {
            language: "typescript".into(),
            file_extensions: vec![".ts".into(), ".tsx".into(), ".js".into(), ".jsx".into()],
            excluded_dirs: vec!["node_modules".into(), "dist".into()],
            scan_export: "scan__typescript".into(),
            classify_export: "classify__typescript".into(),
            map_export: "map__typescript".into(),
            description: None,
        }],
        surfaces: None,
    }
}

#[test]
fn multi_scanner_mixed_project() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("lib.rs"), "pub fn hello() {}").unwrap();
    std::fs::write(dir.path().join("app.ts"), "export function handleRequest() {}\nexport class UserService {}").unwrap();
    std::fs::write(dir.path().join("component.tsx"), "export function render() {}").unwrap();
    std::fs::write(dir.path().join("utils.js"), "export const MAX = 10;").unwrap();
    std::fs::write(dir.path().join("readme.md"), "# Hello").unwrap();

    let runtime = builtins::runtime_for_extensions(&[
        "@specforge/rust".into(),
        "@specforge/typescript".into(),
    ]);
    let manifests = vec![rust_manifest(), typescript_manifest()];
    let source_files = vec![
        "lib.rs".into(),
        "app.ts".into(),
        "component.tsx".into(),
        "utils.js".into(),
        "readme.md".into(),
    ];

    let (items, scanners) = scanner_dispatch::scan_source_files(
        &runtime,
        &manifests,
        dir.path(),
        &source_files,
    );

    assert_eq!(items.len(), 5);

    let rust_items: Vec<_> = items.iter().filter(|i| i.scanner.as_deref() == Some("@specforge/rust")).collect();
    assert_eq!(rust_items.len(), 1);
    assert_eq!(rust_items[0].name, "hello");

    let ts_items: Vec<_> = items.iter().filter(|i| i.scanner.as_deref() == Some("@specforge/typescript")).collect();
    assert_eq!(ts_items.len(), 4);
    assert!(ts_items.iter().any(|i| i.name == "handleRequest"));
    assert!(ts_items.iter().any(|i| i.name == "UserService"));
    assert!(ts_items.iter().any(|i| i.name == "render"));
    assert!(ts_items.iter().any(|i| i.name == "MAX"));

    assert_eq!(scanners.len(), 2);
    assert!(scanners.contains(&"@specforge/rust".to_string()));
    assert!(scanners.contains(&"@specforge/typescript".to_string()));
}
