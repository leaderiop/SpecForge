/// Tests link themselves to entities by annotation (ADR 0002): no extension
/// registers a spec-side `tests` field any more.
#[test]
fn no_spec_side_tests_field_is_registered() {
    let exts = vec![
        "@specforge/software".to_string(),
        "@specforge/testing".to_string(),
        "@specforge/formal".to_string(),
    ];
    let runtime = wasm_runtime_for(&exts);
    let mut diags = Vec::new();
    let manifests = specforge_emitter::compile::load_extensions(&exts, &runtime, &mut diags);
    let (_kind_reg, field_reg, _edge, _d) = specforge_registry::populate_registries(&manifests);
    assert!(field_reg.contains("behavior", "invariants"));
    assert!(!field_reg.contains("behavior", "tests"));
    assert!(!field_reg.contains("invariant", "tests"));
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
