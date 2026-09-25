use specforge_emitter::builtins;

#[test]
fn software_tests_field_is_registered() {
    let exts = vec![
        "@specforge/software".to_string(),
        "@specforge/formal".to_string(),
    ];
    let runtime = builtins::runtime_for_extensions(&exts);
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
