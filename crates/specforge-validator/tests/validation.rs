use specforge_graph::build_graph;
use specforge_parser::parse;
use specforge_test_macros::test as specforge_test;
use specforge_validator::{Diagnostic, Severity};

// === detect_orphan_refs ===

#[specforge_test(
    behavior = "detect_orphan_refs",
    verify = "unreferenced ref produces W012"
)]
fn unreferenced_ref_produces_w012() {
    let source = r#"
behavior alpha "A" { contract "first" }
ref gh.issue:42 "Support Wasm extensions"
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let diagnostics = specforge_validator::validate(&graph);

    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.code == "W012").collect();
    assert_eq!(warnings.len(), 1, "orphan ref should produce W012");
    assert!(warnings[0].message.contains("gh.issue:42"));
    assert_eq!(warnings[0].severity, Severity::Warning);
}

#[specforge_test(
    behavior = "detect_orphan_refs",
    verify = "referenced ref suppresses W012"
)]
fn referenced_ref_suppresses_w012() {
    let source = r#"
behavior alpha "A" {
  contract "first"
  refs [gh.issue:42]
}
ref gh.issue:42 "Support Wasm extensions"
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let diagnostics = specforge_validator::validate(&graph);

    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.code == "W012").collect();
    assert!(
        warnings.is_empty(),
        "referenced ref should not produce W012"
    );
}

#[specforge_test(
    behavior = "detect_orphan_refs",
    verify = "spec block is a root container and does not produce W012"
)]
fn spec_block_does_not_produce_w012() {
    // spec blocks are project root containers — they naturally have no
    // incoming edges and should NOT produce W012.
    let source = r#"
spec "MyProject" {
  version "1.0"
}
behavior alpha "A" { contract "first" }
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let diagnostics = specforge_validator::validate(&graph);

    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.code == "W012").collect();
    assert!(
        warnings.is_empty(),
        "spec block should not produce W012, got: {:?}",
        warnings
    );
}

#[test]
fn non_structural_kind_does_not_produce_w012() {
    // Extension-defined kinds (behavior, feature) are NOT structural —
    // their orphan detection is extension-defined, not core
    let source = r#"
behavior alpha "A" { contract "first" }
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let diagnostics = specforge_validator::validate(&graph);

    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.code == "W012").collect();
    assert!(
        warnings.is_empty(),
        "non-structural kinds should not trigger W012"
    );
}

// === validate_file_reference_paths ===

#[specforge_test(
    behavior = "validate_file_reference_paths",
    verify = "non-existent file reference produces E016"
)]
fn missing_file_reference_produces_e016() {
    use specforge_validator::ValidatorConfig;
    use std::path::Path;

    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["features/alpha.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: Path::new("/nonexistent/project").to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert_eq!(errors.len(), 1, "missing file should produce E016");
    assert!(errors[0].message.contains("alpha.feature"));
}

#[specforge_test(
    behavior = "validate_file_reference_paths",
    verify = "existing file reference passes silently"
)]
fn existing_file_reference_passes() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    let features_dir = dir.path().join("features");
    std::fs::create_dir_all(&features_dir).unwrap();
    std::fs::write(features_dir.join("alpha.feature"), "Feature: Alpha").unwrap();

    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["features/alpha.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert!(errors.is_empty(), "existing file should not produce E016");
}

#[specforge_test(
    behavior = "validate_file_reference_paths",
    verify = "multiple file references in same entity each validated independently"
)]
fn multiple_file_refs_validated_independently() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    let features_dir = dir.path().join("features");
    std::fs::create_dir_all(&features_dir).unwrap();
    std::fs::write(features_dir.join("alpha.feature"), "Feature: Alpha").unwrap();
    // beta.feature does NOT exist

    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["features/alpha.feature", "features/beta.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert_eq!(errors.len(), 1, "only missing file should produce E016");
    assert!(errors[0].message.contains("beta.feature"));
}

// === provide_did_you_mean_suggestions (file references) ===

#[specforge_test(
    behavior = "provide_did_you_mean_suggestions",
    verify = "close match produces suggestion"
)]
fn e016_suggests_similar_filename() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    let features_dir = dir.path().join("features");
    std::fs::create_dir_all(&features_dir).unwrap();
    std::fs::write(features_dir.join("alpha.feature"), "Feature: Alpha").unwrap();

    // Typo: "alpa.feature" instead of "alpha.feature"
    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["features/alpa.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0]
            .suggestion
            .as_ref()
            .is_some_and(|s| s.contains("alpha.feature")),
        "E016 should suggest 'alpha.feature', got: {:?}",
        errors[0].suggestion
    );
}

#[specforge_test(
    behavior = "provide_did_you_mean_suggestions",
    verify = "distant match produces no suggestion"
)]
fn e016_no_suggestion_when_no_similar_file() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    let features_dir = dir.path().join("features");
    std::fs::create_dir_all(&features_dir).unwrap();
    std::fs::write(features_dir.join("zebra.feature"), "Feature: Zebra").unwrap();

    // "alpha.feature" is completely different from "zebra.feature"
    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["features/alpha.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].suggestion.is_none(),
        "should not suggest unrelated file, got: {:?}",
        errors[0].suggestion
    );
}

// === validate_file_reference_paths: relative path ===

#[test]
fn relative_path_resolved_from_spec_root() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    // Create nested structure: spec_root/sub/features/alpha.feature
    let sub_dir = dir.path().join("sub").join("features");
    std::fs::create_dir_all(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("alpha.feature"), "Feature: Alpha").unwrap();

    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["sub/features/alpha.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert!(
        errors.is_empty(),
        "relative path from spec root should resolve, got: {:?}",
        errors
    );
}

// === Contract tests ===

#[specforge_test(
    behavior = "detect_orphan_refs",
    verify = "Detect Orphan Structural Nodes: orphan structural node detection holds — graph_built_fired, orphans_detected, referenced_nodes_clean"
)]
fn orphan_refs_contract_consistency() {
    // Requires: graph_built event has fired (graph is fully constructed)
    // Ensures: all structural nodes with zero incoming edges produce W012,
    //          structural nodes with incoming edges produce no warning

    // Case 1: orphan ref (zero incoming edges) → W012
    let source_orphan = r#"
behavior alpha "A" { contract "first" }
ref gh.issue:42 "Orphan ref"
"#;
    let spec_file = parse(source_orphan, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);
    let diagnostics = specforge_validator::validate(&graph);
    let w012: Vec<_> = diagnostics.iter().filter(|d| d.code == "W012").collect();
    assert_eq!(w012.len(), 1, "orphan structural node must produce W012");

    // Case 2: referenced ref (has incoming edge) → no W012
    let source_linked = r#"
behavior alpha "A" { contract "first" refs [gh.issue:42] }
ref gh.issue:42 "Linked ref"
"#;
    let spec_file = parse(source_linked, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);
    let diagnostics = specforge_validator::validate(&graph);
    let w012: Vec<_> = diagnostics.iter().filter(|d| d.code == "W012").collect();
    assert!(
        w012.is_empty(),
        "referenced structural node must not produce W012"
    );
}

#[specforge_test(
    behavior = "validate_file_reference_paths",
    verify = "Validate File Reference Paths: file reference validation holds — graph_built_fired, filesystem_available, missing_files_diagnosed, existing_files_pass"
)]
fn file_ref_contract_consistency() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    let features_dir = dir.path().join("features");
    std::fs::create_dir_all(&features_dir).unwrap();
    std::fs::write(features_dir.join("exists.feature"), "Feature: OK").unwrap();

    // Requires: graph is built, filesystem is available
    // Ensures: missing files → E016, existing files → no diagnostic
    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["features/exists.feature", "features/missing.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert_eq!(errors.len(), 1, "only missing file should produce E016");
    assert!(errors[0].message.contains("missing.feature"));
}

#[specforge_test(
    behavior = "provide_did_you_mean_suggestions",
    verify = "Provide Did-You-Mean Suggestions: did-you-mean suggestions holds — unresolved_reference_available, kind_registry_populated, distance_threshold, sorted_by_distance"
)]
fn did_you_mean_contract_consistency() {
    // The graph's suggestion for an unresolved `order_service_v2` among `known`.
    let suggest = |known: &[&str]| -> Option<String> {
        let mut source: String = known
            .iter()
            .map(|id| format!("behavior {id} \"X\" {{ contract \"c\" }}\n"))
            .collect();
        source.push_str("feature gamma \"G\" { behaviors [order_service_v2] }\n");
        let (_, diagnostics) = build_graph(&[parse(&source, "main.spec")]);
        // requires unresolved_reference_available: exactly the one E003.
        let e003: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
        assert_eq!(e003.len(), 1, "{diagnostics:?}");
        e003[0].suggestion.clone()
    };

    // distance_threshold: three edits away is suggested, four is not.
    assert_eq!(
        suggest(&["order_service_xyz"]).as_deref(),
        Some("did you mean 'order_service_xyz'?")
    );
    assert_eq!(suggest(&["order_service_v2_log"]), None);

    // sorted_by_distance: two edits beats three, though the three-edit
    // candidate shares the longer prefix; the order written doesn't matter.
    for known in [
        ["order_service_v2_xy", "ordr_servce_v2"],
        ["ordr_servce_v2", "order_service_v2_xy"],
    ] {
        assert_eq!(
            suggest(&known).as_deref(),
            Some("did you mean 'ordr_servce_v2'?"),
            "{known:?}"
        );
    }
}

#[specforge_test(
    behavior = "provide_did_you_mean_suggestions",
    verify = "suggestion appears in help text"
)]
fn suggestion_appears_in_help_text_for_file_refs() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    let features_dir = dir.path().join("features");
    std::fs::create_dir_all(&features_dir).unwrap();
    std::fs::write(features_dir.join("login.feature"), "Feature: Login").unwrap();

    // Typo: "logn.feature" instead of "login.feature"
    let source = r#"
behavior login_flow "Login" {
  contract "handles login"
  gherkin ["features/logn.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let e016 = diagnostics.iter().find(|d| d.code == "E016").unwrap();

    // The rendered diagnostic carries the suggestion on its help line.
    let sources = std::collections::HashMap::from([("main.spec".to_string(), source.to_string())]);
    let rendered =
        specforge_common::render_diagnostics(std::slice::from_ref(e016), &sources, false);
    let help: Vec<&str> = rendered
        .lines()
        .filter(|line| line.contains("Help:"))
        .collect();
    assert_eq!(help.len(), 1, "{rendered}");
    assert!(
        help[0].ends_with("Help: did you mean 'features/login.feature'?"),
        "{rendered}"
    );
}

// === detect_dangling_references ===

#[test]
fn dangling_ref_without_edge_indicates_resolver_bug() {
    // A reference list entry that resolves (target exists) should always
    // produce a corresponding graph edge. If it doesn't, that's a resolver bug.
    // Here we verify the normal path: unresolved references produce E003.
    let source = r#"
behavior alpha "A" {
  contract "first"
  invariants [nonexistent_invariant]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);

    // The reference target doesn't exist → E003 emitted, no edge created
    let e001: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert_eq!(e001.len(), 1, "unresolved reference should produce E003");

    // No edge should exist for the unresolved reference
    let edges = graph.edges_from("alpha");
    assert!(
        edges.iter().all(|e| e.target != "nonexistent_invariant"),
        "no edge should exist for unresolved reference"
    );
}

#[test]
fn resolved_ref_has_corresponding_edge() {
    let source = r#"
behavior alpha "A" {
  contract "first"
  invariants [inv_one]
}
invariant inv_one "Invariant One" {
  contract "must hold"
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);

    // No E003 — reference resolves cleanly
    let e001: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert!(
        e001.is_empty(),
        "resolved reference should not produce E003"
    );

    // Edge must exist from alpha to inv_one
    let edges = graph.edges_from("alpha");
    assert!(
        edges.iter().any(|e| e.target == "inv_one"),
        "resolved reference must have a corresponding graph edge"
    );
}

#[test]
fn empty_graph_no_dangling_diagnostics() {
    let source = r#"
behavior alpha "A" { contract "first" }
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);

    // No reference lists → zero edges → no E003
    assert_eq!(graph.edge_count(), 0, "graph should have zero edges");
    let e001: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert!(
        e001.is_empty(),
        "empty graph should produce no dangling reference diagnostic"
    );
}

#[test]
fn dangling_ref_contract_consistency() {
    // Requires: graph_built event has fired (graph is fully constructed)
    // Ensures: every reference list entry has a corresponding graph edge,
    //          or E003 is raised; no duplicate diagnostics

    // Case 1: resolved reference → edge exists, no E003
    let source_ok = r#"
behavior alpha "A" { contract "first" invariants [inv_one] }
invariant inv_one "I" { contract "must hold" }
"#;
    let spec_file = parse(source_ok, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);
    let e001: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert!(e001.is_empty(), "resolved ref must not produce E003");
    assert!(
        graph
            .edges_from("alpha")
            .iter()
            .any(|e| e.target == "inv_one"),
        "resolved ref must have corresponding edge"
    );

    // Case 2: unresolved reference → E003, no edge
    let source_bad = r#"
behavior beta "B" { contract "second" invariants [missing] }
"#;
    let spec_file = parse(source_bad, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);
    let e001: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert_eq!(
        e001.len(),
        1,
        "unresolved ref must produce exactly one E003"
    );
    assert!(
        graph.edges_from("beta").is_empty(),
        "unresolved ref must not create edge"
    );
}

const REFERENCING: &str = r#"
behavior alpha "A" {
  contract "first"
  invariants [inv_one]
}
invariant inv_one "Invariant One" {
  contract "must hold"
}
"#;

fn dangling(graph: &specforge_graph::Graph) -> Vec<Diagnostic> {
    specforge_validator::validate(graph)
        .into_iter()
        .filter(|d| d.code == "E060")
        .collect()
}

#[specforge_test(
    behavior = "detect_dangling_references",
    verify = "reference without corresponding graph edge indicates resolver bug"
)]
fn reference_without_its_edge_is_a_resolver_bug() {
    let (mut graph, _) = build_graph(&[parse(REFERENCING, "main.spec")]);
    // What a resolver that forgot an edge leaves behind.
    graph.clear_edges();

    let found = dangling(&graph);

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].severity, Severity::Error);
    assert!(
        found[0].message.contains("'alpha'")
            && found[0].message.contains("'inv_one'")
            && found[0].message.contains("invariants"),
        "{}",
        found[0].message
    );
    assert!(found[0].span.is_some(), "points at the reference");
}

#[specforge_test(
    behavior = "detect_dangling_references",
    verify = "reference with corresponding graph edge passes"
)]
fn reference_with_its_edge_passes() {
    let (graph, _) = build_graph(&[parse(REFERENCING, "main.spec")]);
    assert!(dangling(&graph).is_empty());
}

#[specforge_test(
    behavior = "detect_dangling_references",
    verify = "empty graph with zero edges produces no dangling reference diagnostic"
)]
fn empty_graph_has_no_dangling_references() {
    let (graph, _) = build_graph(&[]);
    assert_eq!(graph.edge_count(), 0);
    assert!(dangling(&graph).is_empty());
}

#[specforge_test(
    behavior = "detect_dangling_references",
    verify = "Detect Dangling References: dangling reference detection holds — graph_built_fired, resolver_integrity_verified, no_duplicate_diagnostics"
)]
fn dangling_reference_contract() {
    // An unresolved id is the linker's E003; the validator adds nothing.
    let source = "behavior beta \"B\" { contract \"second\" invariants [missing] }\n";
    let (graph, linker) = build_graph(&[parse(source, "main.spec")]);
    assert_eq!(linker.iter().filter(|d| d.code == "E003").count(), 1);

    let validator = specforge_validator::validate(&graph);
    assert!(
        !validator
            .iter()
            .any(|d| d.code == "E003" || d.code == "E060"),
        "{validator:?}"
    );
}

// === detect_duplicate_entity_ids ===

#[specforge_test(
    behavior = "detect_duplicate_entity_ids",
    verify = "duplicate ID in same file produces E002"
)]
fn duplicate_id_same_file_produces_e002() {
    let source = r#"
behavior alpha "First Alpha" { contract "first" }
behavior alpha "Second Alpha" { contract "second" }
"#;
    let spec_file = parse(source, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);

    let e002: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert_eq!(
        e002.len(),
        1,
        "duplicate ID in same file should produce E002"
    );
    assert!(
        e002[0].message.contains("alpha"),
        "E002 message should name the duplicate ID"
    );
}

#[specforge_test(
    behavior = "detect_duplicate_entity_ids",
    verify = "duplicate ID across files produces E002"
)]
fn duplicate_id_across_files_produces_e002() {
    let source_a = r#"
behavior alpha "Alpha in file A" { contract "first" }
"#;
    let source_b = r#"
behavior alpha "Alpha in file B" { contract "second" }
"#;
    let spec_file_a = parse(source_a, "a.spec");
    let spec_file_b = parse(source_b, "b.spec");
    let (_, diagnostics) = build_graph(&[spec_file_a, spec_file_b]);

    let e002: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert_eq!(
        e002.len(),
        1,
        "duplicate ID across files should produce E002"
    );
    assert!(
        e002[0].message.contains("alpha"),
        "E002 message should name the duplicate ID"
    );
}

#[specforge_test(
    behavior = "detect_duplicate_entity_ids",
    verify = "E002 includes both source locations"
)]
fn e002_includes_both_source_locations() {
    let source_a = r#"
behavior alpha "Alpha in file A" { contract "first" }
"#;
    let source_b = r#"
behavior alpha "Alpha in file B" { contract "second" }
"#;
    let spec_file_a = parse(source_a, "a.spec");
    let spec_file_b = parse(source_b, "b.spec");
    let (_, diagnostics) = build_graph(&[spec_file_a, spec_file_b]);

    let e002: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert_eq!(e002.len(), 1, "should have exactly one E002");

    // The E002 diagnostic's span points to the duplicate (second) declaration,
    // and the message includes the file where the duplicate was found.
    // Both declaration sites are identifiable: the first via the graph node
    // (which retains the original), and the second via the E002 diagnostic span.
    // The span points at the duplicate in b.spec; the message names the first
    // declaration in a.spec. Both entities sit at line 2, column 1.
    let diag = &e002[0];
    let span = diag
        .span
        .as_ref()
        .expect("E002 carries the duplicate's span");
    assert_eq!(span.file.as_str(), "b.spec");
    assert_eq!((span.start_line, span.start_col), (2, 1));
    assert_eq!(
        diag.message,
        "duplicate entity ID 'alpha' (first declared at a.spec:2:1)"
    );
}

#[specforge_test(
    behavior = "detect_duplicate_entity_ids",
    verify = "Detect Duplicate Entity IDs: duplicate entity ID detection holds — all_files_parsed, duplicate_ids_diagnosed"
)]
fn duplicate_id_contract_consistency() {
    // Requires: all_files_parsed (all .spec files parsed, entity IDs collected)
    // Ensures: every duplicate entity ID has E002 naming both declaration sites

    // Case 1: unique IDs → no E002
    let source_unique = r#"
behavior alpha "A" { contract "first" }
behavior beta "B" { contract "second" }
"#;
    let spec_file = parse(source_unique, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);
    let e002: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert!(e002.is_empty(), "unique IDs must not produce E002");

    // Case 2: duplicate IDs → E002 with both sites
    let source_a = r#"
behavior gamma "Gamma A" { contract "first" }
"#;
    let source_b = r#"
behavior gamma "Gamma B" { contract "second" }
"#;
    let spec_file_a = parse(source_a, "first.spec");
    let spec_file_b = parse(source_b, "second.spec");
    let (_, diagnostics) = build_graph(&[spec_file_a, spec_file_b]);
    let e002: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert_eq!(e002.len(), 1, "duplicate IDs must produce exactly one E002");
    assert!(
        e002[0].message.contains("gamma"),
        "E002 must name the duplicate ID"
    );
    assert!(
        e002[0].span.is_some(),
        "E002 must include source span identifying a declaration site"
    );
}
