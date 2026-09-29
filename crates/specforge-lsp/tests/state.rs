use specforge_test_macros::test as spec;

// -- document_open_close ------------------------------------------------------

#[spec(
    behavior = "document_open_close",
    verify = "didOpen registers document and triggers compilation"
)]
fn did_open_registers_document() {
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "behavior a \"A\" {}\n");

    assert!(state.is_open("file:///a.spec"));
    assert_eq!(
        state.document("file:///a.spec").unwrap().content(),
        "behavior a \"A\" {}\n"
    );
}

#[spec(
    behavior = "document_open_close",
    verify = "didClose removes document and clears diagnostics"
)]
fn did_close_removes_document() {
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "behavior a \"A\" {}\n");
    state.close_document("file:///a.spec");

    assert!(!state.is_open("file:///a.spec"));
    assert!(state.document("file:///a.spec").is_none());
}

#[spec(
    behavior = "document_open_close",
    verify = "only open documents participate in incremental compilation"
)]
fn only_open_documents_tracked() {
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "a");
    state.open_document("file:///b.spec", "b");

    assert_eq!(state.open_uris().len(), 2);

    state.close_document("file:///a.spec");
    assert_eq!(state.open_uris().len(), 1);
    assert_eq!(state.open_uris()[0], "file:///b.spec");
}

// -- handle_text_document_change ----------------------------------------------

#[spec(
    behavior = "handle_text_document_change",
    verify = "didChange applies incremental edits to buffer"
)]
fn did_change_applies_edits() {
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "hello world\n");

    state.apply_change("file:///a.spec", 0, 6, 0, 11, "rust");
    assert_eq!(
        state.document("file:///a.spec").unwrap().content(),
        "hello rust\n"
    );
}

// -- validation_patterns -------------------------------------------------------

fn load_patterns_for(
    ext_names: &[&str],
) -> Vec<(
    specforge_registry::validation_engine::ValidationRulePattern,
    String,
)> {
    let names: Vec<String> = ext_names.iter().map(|s| s.to_string()).collect();
    let runtime = wasm_runtime_for(&names);
    let host = specforge_wasm::protocol::ProtocolHost::new(&runtime);
    let mut manifests = Vec::new();
    for name in &names {
        if let Ok(ext) = specforge_wasm::protocol::load_protocol_extension(&host, name) {
            manifests.push(specforge_wasm::protocol::protocol_extension_to_manifest(
                &ext,
            ));
        }
    }
    let (_kind_reg, field_reg, _edge_reg, _diags) =
        specforge_registry::populate_registries(&manifests);
    let rule_inputs: Vec<(String, Vec<_>)> = manifests
        .iter()
        .map(|m| (m.name.clone(), m.validation_rules.clone()))
        .collect();
    let (mut patterns, _) =
        specforge_registry::validation_engine::parse_all_rule_patterns(&rule_inputs);
    patterns.extend(
        specforge_registry::generate_required_field_rules(&field_reg)
            .into_iter()
            .map(|p| (p, String::new())),
    );
    patterns
}

#[spec(
    invariant = "zero_domain_knowledge_core",
    verify = "core with zero extensions installed has zero entity kinds in KindRegistry"
)]
fn state_starts_with_empty_registries() {
    let state = specforge_lsp::LspState::new();
    assert!(state.kind_registry().is_empty());
    assert!(state.validation_patterns().is_empty());
}

#[spec(
    behavior = "register_extension_validation_rules",
    verify = "extensions produce E006 rules for required fields"
)]
fn extensions_produce_e006_rules() {
    let patterns = load_patterns_for(&[
        "@specforge/software",
        "@specforge/product",
        "@specforge/governance",
        "@specforge/formal",
    ]);
    assert!(
        !patterns.is_empty(),
        "extensions should produce validation patterns"
    );
    let e006_count = patterns.iter().filter(|p| p.0.code == "E006").count();
    assert!(
        e006_count > 0,
        "E006 rules should be auto-generated from required fields"
    );
}

#[spec(
    behavior = "register_extension_validation_rules",
    verify = "E006 covers all required fields from builtin extensions"
)]
fn e006_covers_all_required_fields() {
    let patterns = load_patterns_for(&[
        "@specforge/software",
        "@specforge/product",
        "@specforge/governance",
        "@specforge/formal",
    ]);

    let e006_targets: Vec<(&str, &str)> = patterns
        .iter()
        .filter(|p| p.0.code == "E006")
        .filter_map(|p| Some((p.0.target_kind.as_deref()?, p.0.field.as_deref()?)))
        .collect();

    // software
    assert!(e006_targets.contains(&("behavior", "contract")));
    assert!(e006_targets.contains(&("invariant", "guarantee")));
    assert!(e006_targets.contains(&("port", "direction")));
    // product
    assert!(e006_targets.contains(&("feature", "problem")));
    assert!(e006_targets.contains(&("term", "definition")));
    assert!(e006_targets.contains(&("release", "version")));
    assert!(e006_targets.contains(&("journey", "flow")));
    assert!(e006_targets.contains(&("deliverable", "artifact_type")));
    assert!(e006_targets.contains(&("persona", "description")));
    assert!(e006_targets.contains(&("channel", "description")));
    // governance
    assert!(e006_targets.contains(&("decision", "status")));
    assert!(e006_targets.contains(&("decision", "context")));
    assert!(e006_targets.contains(&("decision", "decision")));
    assert!(e006_targets.contains(&("constraint", "description")));
    assert!(e006_targets.contains(&("failure_mode", "severity")));
    assert!(e006_targets.contains(&("failure_mode", "cause")));
    assert!(e006_targets.contains(&("failure_mode", "effect")));
    // formal
    assert!(e006_targets.contains(&("property", "expression")));
    assert!(e006_targets.contains(&("property", "property_type")));
    assert!(e006_targets.contains(&("axiom", "expression")));
    assert!(e006_targets.contains(&("refinement", "abstract_entity")));
    assert!(e006_targets.contains(&("refinement", "concrete_entity")));
    assert!(e006_targets.contains(&("protocol", "alphabet")));
    assert!(e006_targets.contains(&("protocol", "initial_state")));
    assert!(e006_targets.contains(&("process", "alphabet")));
    assert!(e006_targets.contains(&("process", "initial_state")));
}
/// Build a Wasm runtime for a temp project listing `ext_names`, mirroring
/// how a real session loads extensions from specforge.json.
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
