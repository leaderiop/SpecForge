// Slice 14: Keyword Extension Index Tests
//
// Tests B:generate_keyword_extension_index through the public API.

use specforge_registry::{
    generate_keyword_extension_index, ManifestEntityKind, ManifestV2,
};

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
        surfaces: None,
    }
}

fn make_entity_kind(keyword: &str) -> ManifestEntityKind {
    ManifestEntityKind {
        name: keyword.to_string(),
        keyword: keyword.to_string(),
        testable: false,
        singleton: false,
        supports_verify: false,
        allowed_verify_kinds: vec![],
        semantic_token: None,
        lsp_icon: None,
        dot_shape: None,
        dot_color: None,
        dot_fillcolor: None,
        fields: vec![],
        incremental: None,
        has_body_parser: false,
        open_fields: false,
    }
}

fn manifest_with_kinds(name: &str, keywords: &[&str]) -> ManifestV2 {
    ManifestV2 {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        wasm_path: "ext.wasm".to_string(),
        entity_kinds: keywords.iter().map(|k| make_entity_kind(k)).collect(),
        ..default_manifest()
    }
}

// ============================================================================
// B:generate_keyword_extension_index — integration tests
// ============================================================================

// B:generate_keyword_extension_index — verify integration "single extension with 3 kinds → 3 keyword entries"
#[test]
fn test_single_extension_three_kinds() {
    let manifest = manifest_with_kinds("@specforge/software", &["behavior", "invariant", "event"]);

    let index = generate_keyword_extension_index(&[manifest]);
    assert_eq!(index.len(), 3);
    assert_eq!(
        index.extensions_for_keyword("behavior").unwrap(),
        &["@specforge/software"]
    );
    assert_eq!(
        index.extensions_for_keyword("invariant").unwrap(),
        &["@specforge/software"]
    );
    assert_eq!(
        index.extensions_for_keyword("event").unwrap(),
        &["@specforge/software"]
    );
}

// B:generate_keyword_extension_index — verify integration "two extensions with distinct kinds → all mapped"
#[test]
fn test_two_extensions_distinct_kinds() {
    let m1 = manifest_with_kinds("@specforge/software", &["behavior", "invariant"]);
    let m2 = manifest_with_kinds("@specforge/product", &["feature", "journey"]);

    let index = generate_keyword_extension_index(&[m1, m2]);
    assert_eq!(index.len(), 4);
    assert_eq!(
        index.extensions_for_keyword("behavior").unwrap(),
        &["@specforge/software"]
    );
    assert_eq!(
        index.extensions_for_keyword("feature").unwrap(),
        &["@specforge/product"]
    );
}

// B:generate_keyword_extension_index — verify integration "same keyword from two extensions → both listed"
#[test]
fn test_same_keyword_two_extensions() {
    let m1 = manifest_with_kinds("@ext/a", &["behavior"]);
    let m2 = manifest_with_kinds("@ext/b", &["behavior"]);

    let index = generate_keyword_extension_index(&[m1, m2]);
    assert_eq!(index.len(), 1);
    let exts = index.extensions_for_keyword("behavior").unwrap();
    assert_eq!(exts.len(), 2);
    assert!(exts.contains(&"@ext/a".to_string()));
    assert!(exts.contains(&"@ext/b".to_string()));
}

// B:generate_keyword_extension_index — verify integration "empty manifest list → empty index"
#[test]
fn test_empty_manifest_list() {
    let index = generate_keyword_extension_index(&[]);
    assert!(index.is_empty());
    assert_eq!(index.len(), 0);
}

// B:generate_keyword_extension_index — verify integration "index output is deterministic (sorted)"
#[test]
fn test_index_deterministic_sorted() {
    let m1 = manifest_with_kinds("@specforge/software", &["behavior", "invariant", "event"]);
    let m2 = manifest_with_kinds("@specforge/product", &["feature", "journey"]);

    let index1 = generate_keyword_extension_index(&[m1.clone(), m2.clone()]);
    let index2 = generate_keyword_extension_index(&[m1, m2]);

    // BTreeMap ensures deterministic key order
    let keys1: Vec<&String> = index1.entries.keys().collect();
    let keys2: Vec<&String> = index2.entries.keys().collect();
    assert_eq!(keys1, keys2);

    // Keys are sorted alphabetically
    assert_eq!(
        keys1,
        vec!["behavior", "event", "feature", "invariant", "journey"]
    );
}

// B:generate_keyword_extension_index — verify contract "requires manifests, ensures complete keyword mapping"
#[test]
fn test_keyword_index_contract() {
    let m1 = manifest_with_kinds("@specforge/software", &["behavior", "invariant"]);
    let m2 = manifest_with_kinds("@specforge/product", &["feature"]);
    let m3 = manifest_with_kinds("@specforge/governance", &["decision"]);

    let index = generate_keyword_extension_index(&[m1, m2, m3]);

    // ensures: all keywords present
    assert_eq!(index.len(), 4);
    assert!(index.extensions_for_keyword("behavior").is_some());
    assert!(index.extensions_for_keyword("invariant").is_some());
    assert!(index.extensions_for_keyword("feature").is_some());
    assert!(index.extensions_for_keyword("decision").is_some());

    // ensures: unknown keyword returns None
    assert!(index.extensions_for_keyword("nonexistent").is_none());

    // ensures: each keyword maps to correct extension
    assert_eq!(
        index.extensions_for_keyword("decision").unwrap(),
        &["@specforge/governance"]
    );
}
