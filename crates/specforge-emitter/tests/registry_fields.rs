#[test]
fn software_tests_field_is_registered() {
    let exts = vec![
        "@specforge/software".to_string(),
        "@specforge/formal".to_string(),
    ];
    let runtime = wasm_runtime_for(&exts);
    let mut diags = Vec::new();
    let manifests = specforge_emitter::compile::load_extensions(&exts, &runtime, &mut diags);
    let (_kind_reg, field_reg, _edge, _d) = specforge_registry::populate_registries(&manifests);
    println!(
        "behavior/tests: {:?}",
        field_reg.contains("behavior", "tests")
    );
    println!(
        "invariant/tests: {:?}",
        field_reg.contains("invariant", "tests")
    );
    println!(
        "behavior/invariants: {:?}",
        field_reg.contains("behavior", "invariants")
    );
    let m = manifests
        .iter()
        .find(|m| m.name == "@specforge/software")
        .unwrap();
    let b = m
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "behavior")
        .unwrap();
    println!(
        "manifest behavior fields: {:?}",
        b.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>()
    );
    assert!(field_reg.contains("behavior", "tests"));
}

/// Build a Wasm runtime for a temp project listing `ext_names`.
fn wasm_runtime_for(ext_names: &[String]) -> specforge_component::ComponentRuntime {
    let dir = tempfile::TempDir::new().unwrap();
    let config = serde_json::json!({
        "name": "test-project",
        "version": "0.1.0",
        "extensions": ext_names,
    });
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    specforge_component::project_runtime(dir.path())
}
