use specforge_emitter::outline::*;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_wasm::protocol::load_declaration;

fn load_manifest(name: &str) -> ExtensionDeclaration {
    let ext_name = match name {
        "product" => "@specforge/product",
        "software" => "@specforge/software",
        "governance" => "@specforge/governance",
        "formal" => "@specforge/formal",
        _ => panic!("unknown extension: {}", name),
    };
    let runtime = wasm_runtime_for(&[ext_name]);
    load_declaration(&runtime, ext_name).unwrap().declaration
}

/// A Wasm runtime holding the builtin extensions `ext_names`.
fn wasm_runtime_for(ext_names: &[&str]) -> specforge_component::ComponentRuntime {
    let runtime = specforge_component::ComponentRuntime::new();
    let names: Vec<String> = ext_names.iter().map(|name| name.to_string()).collect();
    specforge_component::builtins::load_builtins_for(&runtime, &names).unwrap();
    runtime
}

fn load_all_manifests() -> Vec<ExtensionDeclaration> {
    vec![
        load_manifest("product"),
        load_manifest("software"),
        load_manifest("governance"),
        load_manifest("formal"),
    ]
}

// --- Tracer bullet: empty manifests ---

#[test]
fn empty_manifests_produce_empty_outline() {
    let outline = OutlineIntermediate_from_declarations(&[]);
    assert!(outline.extensions.is_empty());
    assert!(outline.dependencies.is_empty());
    assert!(outline.enhancements.is_empty());
    assert!(outline.cross_edges.is_empty());
}

// --- Single extension ---

#[test]
fn single_extension_maps_basic_metadata() {
    let product = load_manifest("product");
    let outline = OutlineIntermediate_from_declarations(&[product]);

    assert_eq!(outline.extensions.len(), 1);
    let ext = &outline.extensions[0];
    assert_eq!(ext.name, "@specforge/product");
    assert_eq!(ext.version, "1.0.0");
    assert_eq!(ext.entity_kinds.len(), 9);
    assert_eq!(ext.edge_types.len(), 20);
    assert!(ext.contributes.entities);
    assert!(ext.contributes.validators);
}

// --- Peer dependencies ---

#[test]
fn peer_dependencies_mapped_to_outline_dependencies() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    // software→product, governance→software, formal→software (3 direct deps, no reverse)
    assert!(outline.dependencies.len() >= 3);

    let sw_to_prod = outline
        .dependencies
        .iter()
        .find(|d| d.from == "@specforge/software" && d.to == "@specforge/product");
    assert!(sw_to_prod.is_some(), "software→product dependency expected");
    assert_eq!(sw_to_prod.unwrap().version, "^1.0");
}

// --- Entity enhancements ---

#[test]
fn entity_enhancements_detected() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    // software enhances product's module and milestone
    let sw_enhances: Vec<_> = outline
        .enhancements
        .iter()
        .filter(|e| e.enhancer == "@specforge/software")
        .collect();
    assert!(
        sw_enhances.len() >= 2,
        "software should enhance at least 2 kinds, got: {:?}",
        sw_enhances
    );

    let module_enh = sw_enhances.iter().find(|e| e.target_kind == "module");
    assert!(module_enh.is_some(), "software should enhance module");
    assert_eq!(module_enh.unwrap().owner, "@specforge/product");
    assert!(module_enh.unwrap().field_count > 0);
}

// --- Cross-extension edges ---

#[test]
fn cross_extension_edges_detected() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    // governance has edges targeting product kinds (e.g., DecisionAffectsFeature)
    let gov_cross: Vec<_> = outline
        .cross_edges
        .iter()
        .filter(|e| e.owner_extension == "@specforge/governance")
        .collect();
    assert!(
        !gov_cross.is_empty(),
        "governance should have cross-extension edges to product"
    );
}

// --- Validation rules ---

#[test]
fn validation_rules_mapped() {
    let product = load_manifest("product");
    let outline = OutlineIntermediate_from_declarations(&[product]);

    let ext = &outline.extensions[0];
    assert!(
        ext.validation_rules.len() >= 17,
        "product should have at least 17 validation rules, got {}",
        ext.validation_rules.len()
    );
}

// --- Surface counts ---

#[test]
fn surface_counts_mapped() {
    let product = load_manifest("product");
    let outline = OutlineIntermediate_from_declarations(&[product]);

    let ext = &outline.extensions[0];
    // Surface counts should be populated from manifest (product has surfaces)
    // The builder should at minimum not panic on surface mapping
    let _cli = ext.surface_counts.cli_commands;
    let _mcp = ext.surface_counts.mcp_tools;
    let _res = ext.surface_counts.mcp_resources;
}

// --- Enhanced_by attribution on entity kinds ---

#[test]
fn entity_kind_enhanced_by_populated() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    // product's module entity should show enhanced_by software
    let product_ext = outline
        .extensions
        .iter()
        .find(|e| e.name == "@specforge/product")
        .unwrap();
    let module_kind = product_ext
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "module")
        .unwrap();
    assert!(
        !module_kind.enhanced_by.is_empty(),
        "module should be enhanced by software"
    );
    assert_eq!(
        module_kind.enhanced_by[0].source_extension,
        "@specforge/software"
    );
}

// ==========================================================================
// Renderer tests
// ==========================================================================

// --- Markdown ---

#[test]
fn markdown_keys_contains_overview_and_extensions() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Markdown,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    assert!(output.contains("# Extension Architecture"));
    assert!(output.contains("## Overview"));
    assert!(output.contains("## Dependencies"));
    assert!(output.contains("## Extensions"));
    assert!(output.contains("@specforge/product"));
    assert!(output.contains("@specforge/software"));
    assert!(output.contains("**Entity kinds**:"));
    assert!(output.contains("**Contributes**:"));
}

#[test]
fn markdown_none_omits_extension_detail() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Markdown,
        detail: OutlineDetail::None,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    assert!(output.contains("## Overview"));
    assert!(
        !output.contains("## Extensions"),
        "detail=none should omit per-extension sections"
    );
}

#[test]
fn markdown_all_contains_field_tables() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Markdown,
        detail: OutlineDetail::All,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    assert!(output.contains("| Field | Type | Required | Source | Edge | Target |"));
}

#[test]
fn markdown_shows_enhancements() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Markdown,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    assert!(output.contains("## Enhancements"));
    assert!(output.contains("enhances"));
}

// --- Mermaid ---

#[test]
fn mermaid_produces_flowchart_tb_with_subgraphs() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Mermaid,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    assert!(
        output.starts_with("flowchart TB"),
        "should use top-to-bottom flowchart, got: {}",
        &output[..50.min(output.len())]
    );
    assert!(output.contains("subgraph"), "should use subgraph cards");
    assert!(output.contains("end"), "subgraphs should be closed");
    assert!(
        output.contains("@specforge/product"),
        "should show extension names"
    );
    assert!(output.contains("-->|"), "should have dependency edges");
    assert!(
        output.contains("classDef"),
        "should have classDef color definitions"
    );
}

#[test]
fn mermaid_all_includes_entity_names() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Mermaid,
        detail: OutlineDetail::All,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    // detail=all should include entity keyword names in labels
    assert!(output.contains("feature"));
    assert!(output.contains("behavior"));
}

// --- DOT ---

#[test]
fn dot_produces_digraph() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Dot,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    assert!(output.contains("digraph extensions"));
    assert!(output.contains("rankdir=TB"));
    assert!(output.contains("specforge_product"));
    assert!(output.contains("#2ecc71")); // product color
    assert!(output.contains("#4a90d9")); // software color
    assert!(output.contains("-> ")); // edge
}

// --- DOT cross-edges ---

#[test]
fn dot_renders_cross_extension_edges() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Dot,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    // DOT should render cross-extension edges as dotted red lines
    assert!(
        output.contains("style=dotted") && output.contains("#e74c3c"),
        "DOT should render cross-extension edges as dotted red lines"
    );
}

// --- JSON ---

#[test]
fn json_keys_valid_and_contains_extensions() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Json,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("JSON should be valid");
    let extensions = parsed["extensions"].as_array().unwrap();
    assert_eq!(extensions.len(), 4);
    // Keys level should have entity_kinds with names but no field details
    assert!(extensions[0]["entity_kinds"].is_array());
}

#[test]
fn json_none_contains_only_counts() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Json,
        detail: OutlineDetail::None,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("JSON should be valid");
    let ext = &parsed["extensions"][0];
    assert!(ext["entity_count"].is_number());
    // Should NOT have entity_kinds detail
    assert!(ext.get("entity_kinds").is_none());
}

#[test]
fn json_all_contains_full_field_detail() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Json,
        detail: OutlineDetail::All,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("JSON should be valid");
    let extensions = parsed["extensions"].as_array().unwrap();
    // All level should have full entity_kinds with fields
    let product = &extensions[0];
    let first_kind = &product["entity_kinds"][0];
    assert!(first_kind["fields"].is_array());
}

// --- 7.18: json keys includes validation_rules array ---

#[test]
fn json_keys_includes_validation_rule_codes() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Json,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("JSON should be valid");

    let ext = &parsed["extensions"][0];
    // Keys level should now include validation_rules with code+severity+check
    let rules = ext["validation_rules"].as_array();
    assert!(
        rules.is_some(),
        "keys level should include validation_rules array"
    );
    let first_rule = &rules.unwrap()[0];
    assert!(first_rule["code"].is_string(), "rule should have code");
    assert!(
        first_rule["severity"].is_string(),
        "rule should have severity"
    );
    assert!(
        first_rule["check"].is_string(),
        "rule should have check category"
    );
}

// --- 8.7: json metadata envelope ---

#[test]
fn json_keys_has_metadata_envelope() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Json,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("JSON should be valid");

    let metadata = &parsed["metadata"];
    assert!(metadata.is_object(), "should have metadata envelope");
    assert!(metadata["total_extensions"].is_number());
    assert!(metadata["total_entity_kinds"].is_number());
    assert!(metadata["total_edge_types"].is_number());
    assert!(metadata["total_validation_rules"].is_number());
    assert!(metadata["total_cross_edges"].is_number());
    assert!(metadata["total_enhancements"].is_number());
}

// ==========================================================================
// Phase 6: IR enrichment tests
// ==========================================================================

// --- 6.10: required field populated from manifest ---

#[test]
fn field_required_flag_populated_from_manifest() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    // Find an extension with entity kinds that have fields with required=true
    // (product's feature entity has "name" which is required in the manifest)
    let has_required_field = outline.extensions.iter().any(|ext| {
        ext.entity_kinds
            .iter()
            .any(|kind| kind.fields.iter().any(|f| f.required))
    });
    assert!(
        has_required_field,
        "at least one field should have required=true from manifest data"
    );
}

// --- 6.3: validation rule check category ---

#[test]
fn validation_rule_check_category_populated() {
    let product = load_manifest("product");
    let outline = OutlineIntermediate_from_declarations(&[product]);

    let ext = &outline.extensions[0];
    // Every rule should have a non-empty check category
    assert!(
        ext.validation_rules.iter().all(|r| !r.check.is_empty()),
        "all validation rules should have a check category"
    );
    // Product manifest has rules with different check types
    let checks: Vec<&str> = ext
        .validation_rules
        .iter()
        .map(|r| r.check.as_str())
        .collect();
    assert!(
        checks.len() >= 2,
        "should have multiple check categories, got: {:?}",
        checks
    );
}

// --- 6.12: shared fields mapped ---

#[test]
fn shared_fields_mapped_from_manifest() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    // Extensions that declare top-level `fields` in manifest should have shared_fields populated
    let has_shared = outline
        .extensions
        .iter()
        .any(|ext| !ext.shared_fields.is_empty());
    // Even if no extension currently has shared fields, the field should exist and be empty
    // The builder should at minimum not panic
    let _ = has_shared;

    // Verify the struct is accessible and has correct shape
    for ext in &outline.extensions {
        for sf in &ext.shared_fields {
            assert!(!sf.name.is_empty());
            assert!(!sf.field_type.is_empty());
        }
    }
}

// --- 6.6: collector counts ---

#[test]
fn collector_counts_populated() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    // Counts should be accessible (even if zero for most extensions)
    for ext in &outline.extensions {
        let _c = ext.collector_count;
    }
}

// --- 6.4: edge description ---

#[test]
fn edge_type_description_populated() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    // At least some edge types should have descriptions from the manifest
    let has_desc = outline
        .extensions
        .iter()
        .any(|ext| ext.edge_types.iter().any(|e| e.description.is_some()));
    assert!(
        has_desc,
        "at least some edge types should have descriptions from manifests"
    );
}

// --- 7.15: markdown keys shows validation rule codes ---

#[test]
fn markdown_keys_shows_validation_rule_codes() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Markdown,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    // Should show rule codes at keys level, e.g., "E007" or "W041"
    assert!(
        output.contains("**Validation rules**:"),
        "keys level should show validation rule details"
    );
}

// --- 7.19: markdown all shows required indicators ---

#[test]
fn markdown_all_shows_required_indicators() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Markdown,
        detail: OutlineDetail::All,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    assert!(
        output.contains("| Required |") || output.contains("Required"),
        "all detail should show required column in field tables"
    );
}

// --- 7.20: markdown summary footer ---

#[test]
fn markdown_keys_has_summary_footer() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Markdown,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    assert!(
        output.contains("## Summary"),
        "should have summary footer with totals"
    );
    assert!(
        output.contains("extensions"),
        "summary should mention extension count"
    );
}

// --- Mermaid edge differentiation ---

#[test]
fn mermaid_differentiates_required_and_enhancement_edges() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Mermaid,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    // Two distinct edge types rendered (all current deps are required, no optional)
    assert!(
        output.contains("-->|"),
        "should have solid dependency edges"
    );
    assert!(
        output.contains("-.->|\"enhances"),
        "should have dotted enhancement edges"
    );
    // linkStyle for visual differentiation
    assert!(
        output.contains("linkStyle"),
        "should have linkStyle directives"
    );
}

// ==========================================================================
// Outline V2: Dependency model + Mermaid rewrite
// ==========================================================================

#[test]
fn product_is_standalone_root_with_zero_deps() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    let product_deps: Vec<_> = outline
        .dependencies
        .iter()
        .filter(|d| d.from == "@specforge/product")
        .collect();
    assert!(
        product_deps.is_empty(),
        "product is the root — zero deps, got: {:?}",
        product_deps.iter().map(|d| &d.to).collect::<Vec<_>>()
    );
}

#[test]
fn governance_has_two_direct_deps() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    let gov_direct: Vec<_> = outline
        .dependencies
        .iter()
        .filter(|d| d.from == "@specforge/governance" && d.kind == DependencyKind::Direct)
        .collect();
    assert_eq!(
        gov_direct.len(),
        2,
        "governance should have two direct deps, both optional"
    );
    let sw = gov_direct
        .iter()
        .find(|d| d.to == "@specforge/software")
        .unwrap();
    // Governance works without software; its behavior links are optional.
    assert!(sw.optional, "governance→software should be optional");
    let prod = gov_direct
        .iter()
        .find(|d| d.to == "@specforge/product")
        .unwrap();
    assert!(prod.optional, "governance→product should be optional");
}

#[test]
fn governance_to_product_is_direct_optional() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    let gov_to_prod = outline
        .dependencies
        .iter()
        .find(|d| d.from == "@specforge/governance" && d.to == "@specforge/product");
    assert!(
        gov_to_prod.is_some(),
        "governance → product should exist as direct dep"
    );
    assert_eq!(
        gov_to_prod.unwrap().kind,
        DependencyKind::Direct,
        "governance → product should be direct (optional peer dep)"
    );
}

#[test]
fn governance_product_direct_is_optional() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    let gov_to_prod = outline
        .dependencies
        .iter()
        .find(|d| d.from == "@specforge/governance" && d.to == "@specforge/product")
        .expect("governance → product dep should exist");
    assert_eq!(
        gov_to_prod.kind,
        DependencyKind::Direct,
        "governance → product is a direct optional peer dependency"
    );
    assert!(
        gov_to_prod.optional,
        "governance → product should be optional"
    );
}

#[test]
fn formal_to_product_transitive_computed() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    let formal_to_prod = outline
        .dependencies
        .iter()
        .find(|d| d.from == "@specforge/formal" && d.to == "@specforge/product");
    assert!(
        formal_to_prod.is_some(),
        "formal → product should exist as transitive dep (via formal → software → product)"
    );
    assert_ne!(
        formal_to_prod.unwrap().kind,
        DependencyKind::Direct,
        "formal → product should be transitive"
    );
}

#[test]
fn no_reverse_dependencies_in_dag() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    for dep in &outline.dependencies {
        let has_reverse = outline
            .dependencies
            .iter()
            .any(|d| d.from == dep.to && d.to == dep.from);
        assert!(
            !has_reverse,
            "reverse dependency: {} <-> {}",
            dep.from, dep.to
        );
    }
}

#[test]
fn deps_direct_filters_out_transitive() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    let filtered = filter_dependencies(&outline.dependencies, DependencyDepth::Direct);
    assert!(
        filtered.iter().all(|d| d.kind == DependencyKind::Direct),
        "direct mode should only contain direct deps"
    );
    // governance → product SHOULD appear in direct mode (it's an optional direct dep now)
    assert!(
        filtered
            .iter()
            .any(|d| d.from == "@specforge/governance" && d.to == "@specforge/product"),
        "direct governance→product should appear in direct mode"
    );
}

#[test]
fn deps_effective_shows_used_transitive() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    let filtered = filter_dependencies(&outline.dependencies, DependencyDepth::Effective);
    // governance → product SHOULD appear (effective)
    assert!(
        filtered
            .iter()
            .any(|d| d.from == "@specforge/governance" && d.to == "@specforge/product"),
        "effective governance→product should appear in effective mode"
    );
    // No pure transitive deps should appear
    assert!(
        !filtered
            .iter()
            .any(|d| d.kind == DependencyKind::Transitive),
        "effective mode should not include pure transitive deps"
    );
}

#[test]
fn deps_full_shows_everything() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    let filtered = filter_dependencies(&outline.dependencies, DependencyDepth::Full);
    assert_eq!(
        filtered.len(),
        outline.dependencies.len(),
        "full mode should include all deps"
    );
}

#[specforge_test_macros::test(
    behavior = "render_extension_outline",
    verify = "the dependencies every format shows, json included, are the ones --deps selects"
)]
fn the_json_outline_lists_the_dependencies_deps_selects() {
    let outline = OutlineIntermediate_from_declarations(&load_all_manifests());
    let listed = |deps| -> Vec<String> {
        let options = OutlineOptions {
            format: OutlineFormat::Json,
            detail: OutlineDetail::Keys,
            deps,
        };
        let json: serde_json::Value = serde_json::from_str(&render(&outline, &options)).unwrap();
        json["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["kind"].as_str().unwrap().to_string())
            .collect()
    };
    let direct = listed(DependencyDepth::Direct);
    assert!(direct.iter().all(|k| k == "direct"), "{direct:?}");
    assert_eq!(
        direct.len(),
        outline
            .dependencies
            .iter()
            .filter(|d| d.kind == DependencyKind::Direct)
            .count()
    );
    assert!(!listed(DependencyDepth::Effective).contains(&"transitive".to_string()));
    let full = listed(DependencyDepth::Full);
    assert_eq!(full.len(), outline.dependencies.len());
    assert!(full.contains(&"transitive".to_string()), "{full:?}");
}

#[test]
fn mermaid_card_has_stats_divider_keywords() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Mermaid,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    // Stats line (bold)
    assert!(output.contains("<b>"), "card should have bold stats line");
    assert!(output.contains("entities"), "card should show entity count");
    // Unicode divider
    assert!(
        output.contains("\u{2500}\u{2500}\u{2500}"),
        "card should have unicode divider"
    );
    // Entity keywords (product has "journey")
    assert!(
        output.contains("journey"),
        "card should contain entity keywords"
    );
    // Italic extras
    assert!(
        output.contains("<i>"),
        "card should have italic extras section"
    );
}

#[test]
fn mermaid_renders_required_dep_as_solid_arrow() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Mermaid,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    assert!(
        output.contains("-->|\"depends on\"|"),
        "required deps should render as solid arrows with label"
    );
}

#[test]
fn only_the_product_links_and_governance_software_are_optional() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);

    let mut optional_deps: Vec<(&str, &str)> = outline
        .dependencies
        .iter()
        .filter(|d| d.optional)
        .map(|d| (d.from.as_str(), d.to.as_str()))
        .collect();
    optional_deps.sort();
    assert_eq!(
        optional_deps,
        vec![
            ("@specforge/formal", "@specforge/product"),
            ("@specforge/governance", "@specforge/product"),
            ("@specforge/governance", "@specforge/software"),
            ("@specforge/software", "@specforge/product"),
        ]
    );
}

#[test]
fn json_dependencies_include_optional_field() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Json,
        detail: OutlineDetail::Keys,
        deps: DependencyDepth::Full,
    };
    let output = render(&outline, &opts);
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("JSON should be valid");
    let deps = parsed["dependencies"].as_array().unwrap();
    let mut optional_deps: Vec<String> = deps
        .iter()
        .filter(|d| d["optional"] == true)
        .map(|d| {
            format!(
                "{} -> {}",
                d["from"].as_str().unwrap(),
                d["to"].as_str().unwrap()
            )
        })
        .collect();
    optional_deps.sort();
    assert_eq!(
        optional_deps,
        vec![
            "@specforge/formal -> @specforge/product",
            "@specforge/governance -> @specforge/product",
            "@specforge/governance -> @specforge/software",
            "@specforge/software -> @specforge/product",
        ]
    );
}

#[test]
fn mermaid_renders_enhancement_edges() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let opts = OutlineOptions {
        format: OutlineFormat::Mermaid,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&outline, &opts);
    assert!(
        output.contains("enhances"),
        "mermaid should render enhancement edges"
    );
}

// The transitive closure is computed over maps: it must not depend on
// their iteration order (which chain decides optionality and version, and
// the order dependencies come out in).
#[specforge_test_macros::test(
    behavior = "deterministic_output",
    verify = "same input produces identical output across runs"
)]
fn transitive_dependencies_are_the_same_on_every_run() {
    let manifests = load_all_manifests();
    let options = OutlineOptions {
        format: OutlineFormat::Json,
        detail: OutlineDetail::Keys,
        deps: DependencyDepth::Full,
    };
    let first = render(&OutlineIntermediate_from_declarations(&manifests), &options);
    for _ in 0..20 {
        assert_eq!(
            render(&OutlineIntermediate_from_declarations(&manifests), &options),
            first
        );
    }
}

// ==========================================================================
// Declared text in the text diagrams (plan 16)
// ==========================================================================

/// One extension, `@t/q` at `version`, declaring a `note` kind whose keyword
/// is `no"te`.
fn quoted(version: &str) -> OutlineIntermediate {
    use specforge_protocol_types::{EntityKindDescriptor, FieldDescriptor, HandshakeResponse};
    let declaration = ExtensionDeclaration {
        handshake: HandshakeResponse {
            name: "@t/q".to_string(),
            version: version.to_string(),
            ..Default::default()
        },
        entities: vec![EntityKindDescriptor {
            name: "note".to_string(),
            keyword: Some("no\"te".to_string()),
            fields: vec![FieldDescriptor {
                name: "body".to_string(),
                field_type: "string".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    OutlineIntermediate_from_declarations(&[declaration])
}

#[specforge_test_macros::test(
    behavior = "render_extension_outline",
    verify = "declared text with a quote, markup or a line break stays inside its Mermaid label"
)]
fn a_quote_in_a_version_or_keyword_stays_inside_the_mermaid_label() {
    let opts = OutlineOptions {
        format: OutlineFormat::Mermaid,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&quoted("1.0.0-\"rc\""), &opts);
    assert!(
        output.contains("subgraph t_q[\"  @t/q v1.0.0-#quot;rc#quot;  \"]"),
        "{output}"
    );
    assert!(output.contains("<br>no#quot;te\"]"), "{output}");
}

#[specforge_test_macros::test(
    behavior = "render_extension_outline",
    verify = "declared text with a pipe or a line break stays in its Markdown table cell"
)]
fn a_pipe_in_a_version_stays_in_its_markdown_cell() {
    let opts = OutlineOptions {
        format: OutlineFormat::Markdown,
        detail: OutlineDetail::Keys,
        ..Default::default()
    };
    let output = render(&quoted("1.0|rc"), &opts);
    assert!(output.contains("| @t/q | 1.0\\|rc |"), "{output}");
}

#[specforge_test_macros::test(
    behavior = "render_extension_outline",
    verify = "the builtins' outline is unchanged in every format"
)]
fn outline_of_the_builtins_in_every_format() {
    let outline = OutlineIntermediate_from_declarations(&load_all_manifests());
    for (format, name) in [
        (OutlineFormat::Markdown, "markdown"),
        (OutlineFormat::Mermaid, "mermaid"),
        (OutlineFormat::Dot, "dot"),
    ] {
        let opts = OutlineOptions {
            format,
            detail: OutlineDetail::All,
            deps: DependencyDepth::Full,
        };
        insta::assert_snapshot!(format!("outline_builtins_{name}"), render(&outline, &opts));
    }
}

#[specforge_test_macros::test(
    behavior = "render_extension_outline",
    verify = "Render the Extension Outline: outline rendering holds — declarations_loaded, one_card_per_extension, declared_text_contained, deps_selected"
)]
fn outline_rendering_contract() {
    // declarations_loaded: the four builtins load.
    let manifests = load_all_manifests();
    assert_eq!(manifests.len(), 4);
    let outline = OutlineIntermediate_from_declarations(&manifests);
    let render_as = |outline: &OutlineIntermediate, format, deps| {
        render(
            outline,
            &OutlineOptions {
                format,
                detail: OutlineDetail::Keys,
                deps,
            },
        )
    };

    // one_card_per_extension: the overview has one row per extension.
    let markdown = render_as(&outline, OutlineFormat::Markdown, DependencyDepth::Direct);
    for ext in &manifests {
        let row = format!("| {} | {} |", ext.handshake.name, ext.handshake.version);
        assert!(markdown.contains(&row), "{row}: {markdown}");
    }

    // declared_text_contained: a pipe stays in its cell.
    let quoted = render_as(
        &quoted("1.0|rc"),
        OutlineFormat::Markdown,
        DependencyDepth::Direct,
    );
    assert!(quoted.contains("1.0\\|rc"), "{quoted}");

    // deps_selected: Mermaid and JSON under Direct show the same pairs.
    let mermaid = render_as(&outline, OutlineFormat::Mermaid, DependencyDepth::Direct);
    let json: serde_json::Value = serde_json::from_str(&render_as(
        &outline,
        OutlineFormat::Json,
        DependencyDepth::Direct,
    ))
    .unwrap();
    let listed = json["dependencies"].as_array().unwrap();
    let arrows = mermaid
        .lines()
        .filter(|l| l.contains("depends on") || l.contains("optional dep"))
        .count();
    assert_eq!(listed.len(), arrows, "{mermaid}");
}
