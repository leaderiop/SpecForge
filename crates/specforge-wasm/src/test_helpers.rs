use specforge_registry::{ManifestV2, PeerDependency};

pub fn default_manifest() -> ManifestV2 {
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

pub fn make_manifest(name: &str, peers: &[(&str, &str)]) -> ManifestV2 {
    ManifestV2 {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        manifest_version: 2,
        wasm_path: String::new(),
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
