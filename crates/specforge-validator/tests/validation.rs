use specforge_graph::build_graph;
use specforge_parser::parse;
use specforge_test_macros::test as specforge_test;
use specforge_validator::{Diagnostic, Severity};

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
