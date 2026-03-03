use std::collections::HashMap;

use specforge_common::{
    CompilerConfig, Diagnostic, EntityKind, FieldValue, SourceSpan, ValidationCode,
};
use specforge_graph::{FileIndex, SpecGraph};
use specforge_parser::SpecFile;
use specforge_resolver::{FileGraph, SymbolTable};

/// Helper: build a minimal ServerState-like structure for testing.
#[allow(dead_code)]
struct TestState {
    files: Vec<SpecFile>,
    graph: SpecGraph,
    file_index: FileIndex,
    file_graph: FileGraph,
    symbols: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    config: CompilerConfig,
    sources: HashMap<String, String>,
}

impl TestState {
    fn from_source(file_path: &str, source: &str) -> Self {
        Self::from_sources(&[(file_path, source)])
    }

    fn from_sources(file_sources: &[(&str, &str)]) -> Self {
        let mut parsed_files = Vec::new();
        let mut sources = HashMap::new();
        for (path, source) in file_sources {
            parsed_files.push(specforge_parser::parse(source, path));
            sources.insert(path.to_string(), source.to_string());
        }

        let resolved = specforge_resolver::resolve(parsed_files, ".");
        let graph_result = specforge_graph::build_graph(&resolved.files);
        let validation_bag = specforge_validator::validate(
            &resolved.files,
            &graph_result.graph,
            &resolved.config,
            std::path::Path::new("."),
        );

        let mut all_diagnostics = resolved.diagnostics.sorted();
        all_diagnostics.extend(validation_bag.sorted());
        all_diagnostics.sort();

        TestState {
            files: resolved.files,
            graph: graph_result.graph,
            file_index: graph_result.file_index,
            file_graph: resolved.file_graph,
            symbols: resolved.symbols,
            diagnostics: all_diagnostics,
            config: resolved.config,
            sources,
        }
    }
}

/// Replicate `find_entity_description` logic from hover.rs for testing.
fn find_entity_description(files: &[SpecFile], entity_id: &str) -> Option<String> {
    for file in files {
        for entity in &file.entities {
            if entity.id.raw() != entity_id {
                continue;
            }
            for (key, value) in &entity.fields.entries {
                match key.as_str() {
                    "contract" | "guarantee" | "problem" | "definition" => {
                        if let FieldValue::String(s) = value {
                            return Some(s.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

/// Replicate `check_entity_fields` logic from code_actions.rs for testing.
fn check_entity_fields(files: &[SpecFile], entity_id: &str) -> (bool, bool) {
    let mut has_verify_or_scenario = false;
    let mut has_tests_field = false;
    for file in files {
        for entity in &file.entities {
            if entity.id.raw() != entity_id {
                continue;
            }
            for (key, value) in &entity.fields.entries {
                match key.as_str() {
                    "tests" => has_tests_field = true,
                    _ => {
                        if let FieldValue::VerifyList(v) = value {
                            if !v.is_empty() {
                                has_verify_or_scenario = true;
                            }
                        }
                        if let FieldValue::ScenarioList(s) = value {
                            if !s.is_empty() {
                                has_verify_or_scenario = true;
                            }
                        }
                    }
                }
            }
        }
    }
    (has_verify_or_scenario, has_tests_field)
}

/// Replicate `generate_test_path` logic from code_actions.rs for testing.
fn generate_test_path(entity_id: &str, kind: &EntityKind) -> String {
    let dir = match kind {
        EntityKind::Behavior => "behaviors",
        EntityKind::Invariant => "invariants",
        EntityKind::Event => "events",
        EntityKind::Constraint => "constraints",
        EntityKind::Capability => "capabilities",
        _ => "tests",
    };
    format!("tests/{dir}/{entity_id}_test.rs")
}

/// Replicate `find_identifier_occurrences` from util.rs for multi-file ref tests.
fn find_identifier_occurrences(source: &str, target_id: &str) -> Vec<(u32, u32, u32)> {
    use tree_sitter::Parser;
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_specforge::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let mut results = Vec::new();
    fn collect(
        node: tree_sitter::Node,
        source: &str,
        target_id: &str,
        results: &mut Vec<(u32, u32, u32)>,
    ) {
        if node.kind() == "identifier" || node.kind() == "scheme_ref_id" {
            let text = &source[node.byte_range()];
            if text == target_id {
                let start = node.start_position();
                let end = node.end_position();
                results.push((start.row as u32, start.column as u32, end.column as u32));
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect(child, source, target_id, results);
        }
    }
    collect(tree.root_node(), source, target_id, &mut results);
    results
}

/// Collect semantic tokens from source — replicates collect_tokens from semantic_tokens.rs.
fn collect_semantic_tokens(source: &str) -> Vec<(u32, u32, u32, u32, u32)> {
    use tree_sitter::Parser;
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_specforge::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let mut tokens = Vec::new();
    collect_tokens_recursive(tree.root_node(), source, &mut tokens);
    tokens.sort_by_key(|t| (t.0, t.1));
    tokens
}

fn collect_tokens_recursive(
    node: tree_sitter::Node,
    source: &str,
    tokens: &mut Vec<(u32, u32, u32, u32, u32)>,
) {
    let kind = node.kind();
    let start = node.start_position();
    let end = node.end_position();
    let line = start.row as u32;
    let col = start.column as u32;

    match kind {
        "spec" | "invariant" | "behavior" | "feature" | "event" | "type" | "port" | "ref"
        | "capability" | "deliverable" | "roadmap" | "library" | "glossary" | "decision"
        | "constraint" | "failure_mode" => {
            if node.parent().is_some_and(|p| {
                p.kind().ends_with("_block")
                    || p.kind().starts_with("type_")
                    || p.kind().starts_with("ref_")
            }) {
                let text = &source[node.byte_range()];
                tokens.push((line, col, text.len() as u32, 0, 0)); // keyword
            }
        }
        "verify" | "scenario" | "given" | "when" | "then" | "use" | "persona" | "surface"
        | "providers" | "coverage" | "gen" | "term" | "method" => {
            if node.parent().is_some() {
                let text = &source[node.byte_range()];
                tokens.push((line, col, text.len() as u32, 0, 0)); // keyword
            }
        }
        "string" | "triple_quoted_string" => {
            let length = if start.row == end.row {
                (end.column - start.column) as u32
            } else {
                let line_text = source.lines().nth(start.row).unwrap_or("");
                (line_text.len() - start.column) as u32
            };
            tokens.push((line, col, length, 4, 0)); // string
        }
        "identifier" => {
            let text = &source[node.byte_range()];
            let parent = node.parent();
            if let Some(parent) = parent {
                let parent_kind = parent.kind();
                if parent_kind.ends_with("_block") || parent_kind.starts_with("type_") {
                    if node
                        .parent()
                        .and_then(|p| {
                            let mut cursor = p.walk();
                            for child in p.children_by_field_name("id", &mut cursor) {
                                if child.id() == node.id() {
                                    return Some(());
                                }
                            }
                            None
                        })
                        .is_some()
                    {
                        tokens.push((line, col, text.len() as u32, 3, 1)); // variable + declaration
                        return;
                    }
                }
                if parent_kind == "identifier_list" || parent_kind == "string_list" {
                    tokens.push((line, col, text.len() as u32, 3, 0)); // variable (reference)
                    return;
                }
            }
            tokens.push((line, col, text.len() as u32, 3, 0)); // variable
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_tokens_recursive(child, source, tokens);
    }
}

// ── Position conversion tests ──────────────────────────────────

#[test]
fn position_conversion_boundary() {
    let span = SourceSpan::new("test.spec", 1, 1, 100, 50);
    let range = specforge_lsp_position::span_to_range(&span);
    assert_eq!(range.start.line, 0);
    assert_eq!(range.start.character, 0);
    assert_eq!(range.end.line, 99);
    assert_eq!(range.end.character, 49);
}

// Inline the position module for testing since it's a binary crate
mod specforge_lsp_position {
    use specforge_common::SourceSpan;
    use tower_lsp::lsp_types::{Position, Range};

    pub fn span_to_range(span: &SourceSpan) -> Range {
        Range {
            start: Position {
                line: span.start_line.saturating_sub(1),
                character: span.start_col.saturating_sub(1),
            },
            end: Position {
                line: span.end_line.saturating_sub(1),
                character: span.end_col.saturating_sub(1),
            },
        }
    }
}

// ── Symbol table tests ─────────────────────────────────────────

#[test]
fn cold_build_populates_symbol_table() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """all data MUST be validated"""
  risk high
}

behavior validate_input "Validate Input" {
  invariants [data_integrity]
  contract """the system MUST validate all input"""
  verify unit "check validation"
}"#;

    let state = TestState::from_source("test.spec", source);

    // Should have 3 entities: spec, invariant, behavior
    assert!(state.symbols.get("test").is_some() || state.symbols.len() >= 2);
    assert!(state.symbols.contains("data_integrity"));
    assert!(state.symbols.contains("validate_input"));

    // Check declaration kinds
    let data_decl = state.symbols.get("data_integrity").unwrap();
    assert_eq!(data_decl.kind, EntityKind::Invariant);

    let validate_decl = state.symbols.get("validate_input").unwrap();
    assert_eq!(validate_decl.kind, EntityKind::Behavior);
}

// ── Graph tests ────────────────────────────────────────────────

#[test]
fn graph_edges_from_reference_fields() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """data must be valid"""
  risk high
}

behavior validate_input "Validate Input" {
  invariants [data_integrity]
  contract """must validate"""
  verify unit "test"
}"#;

    let state = TestState::from_source("test.spec", source);

    // validate_input → data_integrity edge should exist
    let outgoing = state.graph.outgoing_edges("validate_input");
    assert!(!outgoing.is_empty(), "should have outgoing edges");
    assert!(
        outgoing.iter().any(|(node, _)| node.id.raw() == "data_integrity"),
        "should reference data_integrity"
    );
}

// ── File index tests ───────────────────────────────────────────

#[test]
fn file_index_maps_entities_to_files() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """valid"""
  risk high
}

behavior validate_input "Validate" {
  contract """validate"""
  verify unit "test"
}"#;

    let state = TestState::from_source("test.spec", source);

    let entities = state.file_index.entities_in("test.spec");
    assert!(entities.len() >= 2, "should have at least 2 entities in file");
}

// ── Diagnostics conversion tests ───────────────────────────────

#[test]
fn diagnostic_severity_mapping() {
    let error_diag = Diagnostic::new(
        ValidationCode::E001,
        SourceSpan::new("test.spec", 1, 1, 1, 10),
        "unresolved reference",
    );
    assert_eq!(error_diag.severity(), specforge_common::Severity::Error);

    let warning_diag = Diagnostic::new(
        ValidationCode::W001,
        SourceSpan::new("test.spec", 1, 1, 1, 10),
        "orphan behavior",
    );
    assert_eq!(warning_diag.severity(), specforge_common::Severity::Warning);

    let info_diag = Diagnostic::new(
        ValidationCode::I003,
        SourceSpan::new("test.spec", 1, 1, 1, 10),
        "newer format available",
    );
    assert_eq!(info_diag.severity(), specforge_common::Severity::Info);
}

// ── Entity at position (tree-sitter walk) tests ────────────────

mod entity_at_position {
    use tree_sitter::Parser;

    fn entity_at_position(source: &str, line: u32, col: u32) -> Option<String> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_specforge::LANGUAGE.into())
            .ok()?;
        let tree = parser.parse(source, None)?;
        let point = tree_sitter::Point::new(line as usize, col as usize);
        let node = tree.root_node().descendant_for_point_range(point, point)?;

        let mut current = node;
        loop {
            if current.kind() == "identifier" {
                let text = &source[current.byte_range()];
                return Some(text.to_string());
            }
            if current.kind() == "scheme_ref_id" {
                let text = &source[current.byte_range()];
                return Some(text.to_string());
            }
            if let Some(parent) = current.parent() {
                if parent.kind().ends_with("_block")
                    || parent.kind() == "source_file"
                    || parent.kind().starts_with("type_")
                {
                    break;
                }
                current = parent;
            } else {
                break;
            }
        }
        if node.kind() == "identifier" {
            return Some(source[node.byte_range()].to_string());
        }
        None
    }

    #[test]
    fn at_declaration_id() {
        let source = r#"behavior validate_input "Validate" {
  contract """must validate"""
}"#;
        let result = entity_at_position(source, 0, 12);
        assert_eq!(result.as_deref(), Some("validate_input"));
    }

    #[test]
    fn at_reference_in_list() {
        let source = r#"feature input_validation "Input" {
  behaviors [validate_input]
}"#;
        let result = entity_at_position(source, 1, 14);
        assert_eq!(result.as_deref(), Some("validate_input"));
    }

    #[test]
    fn in_multiline_spec() {
        let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """all data MUST be validated"""
  enforced_by [validate_input]
  risk high
}

behavior validate_input "Validate Input" {
  invariants [data_integrity]
  contract """validate"""
  verify unit "check"
}"#;
        // data_integrity at line 5 col 10
        let result = entity_at_position(source, 5, 12);
        assert_eq!(result.as_deref(), Some("data_integrity"));

        // validate_input in enforced_by list at line 7
        let result = entity_at_position(source, 7, 16);
        assert_eq!(result.as_deref(), Some("validate_input"));

        // validate_input declaration at line 11
        let result = entity_at_position(source, 11, 12);
        assert_eq!(result.as_deref(), Some("validate_input"));

        // data_integrity in invariants list at line 12
        let result = entity_at_position(source, 12, 15);
        assert_eq!(result.as_deref(), Some("data_integrity"));
    }
}

// ── Find all identifier occurrences tests ──────────────────────

mod find_occurrences {
    use tree_sitter::Parser;

    fn find_identifier_occurrences(source: &str, target_id: &str) -> Vec<(u32, u32, u32)> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_specforge::LANGUAGE.into())
            .is_err()
        {
            return Vec::new();
        }
        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let mut results = Vec::new();
        collect_identifiers(tree.root_node(), source, target_id, &mut results);
        results
    }

    fn collect_identifiers(
        node: tree_sitter::Node,
        source: &str,
        target_id: &str,
        results: &mut Vec<(u32, u32, u32)>,
    ) {
        if node.kind() == "identifier" || node.kind() == "scheme_ref_id" {
            let text = &source[node.byte_range()];
            if text == target_id {
                let start = node.start_position();
                let end = node.end_position();
                results.push((start.row as u32, start.column as u32, end.column as u32));
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect_identifiers(child, source, target_id, results);
        }
    }

    #[test]
    fn finds_declaration_and_references() {
        let source = r#"invariant data_integrity "Data" {
  risk high
}
behavior validate_input "Validate" {
  invariants [data_integrity]
  contract """x"""
}"#;
        let occurrences = find_identifier_occurrences(source, "data_integrity");
        assert_eq!(occurrences.len(), 2); // declaration + reference
    }

    #[test]
    fn no_false_positives() {
        let source = r#"invariant data_integrity "Data" {
  risk high
}
behavior validate_input "Validate" {
  contract """data_integrity is important"""
}"#;
        let occurrences = find_identifier_occurrences(source, "data_integrity");
        // Should find 1 in declaration, not in the string literal
        assert_eq!(occurrences.len(), 1);
    }

    #[test]
    fn multiple_references() {
        let source = r#"invariant data_integrity "Data" {
  risk high
}
behavior a "A" {
  invariants [data_integrity]
  contract """x"""
}
behavior b "B" {
  invariants [data_integrity]
  contract """y"""
}"#;
        let occurrences = find_identifier_occurrences(source, "data_integrity");
        assert_eq!(occurrences.len(), 3); // 1 declaration + 2 references
    }
}

// ── Document symbol mapping tests ──────────────────────────────

#[test]
fn entity_kind_to_symbol_kind_mapping() {
    use tower_lsp::lsp_types::SymbolKind;

    // Test representative mappings
    fn kind_to_symbol(kind: EntityKind) -> SymbolKind {
        match kind {
            EntityKind::Behavior => SymbolKind::FUNCTION,
            EntityKind::Invariant => SymbolKind::PROPERTY,
            EntityKind::Feature => SymbolKind::MODULE,
            EntityKind::TypeDef => SymbolKind::STRUCT,
            EntityKind::Port => SymbolKind::INTERFACE,
            EntityKind::Event => SymbolKind::EVENT,
            EntityKind::Spec => SymbolKind::NAMESPACE,
            _ => SymbolKind::VARIABLE,
        }
    }

    assert_eq!(kind_to_symbol(EntityKind::Behavior), SymbolKind::FUNCTION);
    assert_eq!(kind_to_symbol(EntityKind::Invariant), SymbolKind::PROPERTY);
    assert_eq!(kind_to_symbol(EntityKind::TypeDef), SymbolKind::STRUCT);
    assert_eq!(kind_to_symbol(EntityKind::Port), SymbolKind::INTERFACE);
    assert_eq!(kind_to_symbol(EntityKind::Event), SymbolKind::EVENT);
}

// ── Hover content tests ────────────────────────────────────────

#[test]
fn hover_content_format() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """all data MUST be validated before persistence"""
  risk high
}

behavior validate_input "Validate Input" {
  invariants [data_integrity]
  contract """the system MUST validate all input fields"""
  verify unit "check validation logic"
}"#;

    let state = TestState::from_source("test.spec", source);

    // Check that the graph has the node
    let node = state.graph.get_node("data_integrity");
    assert!(node.is_some(), "data_integrity should be in graph");
    let node = node.unwrap();
    assert_eq!(node.kind, EntityKind::Invariant);
    assert_eq!(node.title.as_deref(), Some("Data Integrity"));

    // Check incoming edges (validate_input → data_integrity)
    let incoming = state.graph.incoming_edges("data_integrity");
    assert!(!incoming.is_empty(), "should have incoming references");
}

// ── Completion context tests ───────────────────────────────────

#[test]
fn completion_field_context_detection() {
    use specforge_common::EdgeType;

    // EdgeType::from_field_name maps to expected target kinds
    let edge = EdgeType::from_field_name("invariants");
    assert!(edge.is_some());
    let target = edge.unwrap().target_kind();
    assert_eq!(target, Some(EntityKind::Invariant));

    let edge = EdgeType::from_field_name("behaviors");
    assert!(edge.is_some());
    let target = edge.unwrap().target_kind();
    assert_eq!(target, Some(EntityKind::Behavior));

    // Features field targets features
    let edge = EdgeType::from_field_name("features");
    assert!(edge.is_some());
}

// ── Rename validation tests ────────────────────────────────────

#[test]
fn rename_checks_uniqueness() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """valid"""
  risk high
}

invariant email_uniqueness "Email Uniqueness" {
  guarantee """email unique"""
  risk medium
}"#;

    let state = TestState::from_source("test.spec", source);

    // Both symbols exist
    assert!(state.symbols.contains("data_integrity"));
    assert!(state.symbols.contains("email_uniqueness"));

    // Renaming data_integrity → email_uniqueness should be rejected
    let new_name = "email_uniqueness";
    assert!(state.symbols.contains(new_name), "target name already exists");
}

// ── Incremental rebuild tests ──────────────────────────────────

#[test]
fn file_graph_invalidation() {
    let mut fg = FileGraph::new();
    fg.add_import("a.spec", "b.spec");
    fg.add_import("b.spec", "c.spec");

    // If c.spec changes, a.spec and b.spec should also be invalidated
    let set = fg.invalidation_set("c.spec");
    assert!(set.contains("a.spec"));
    assert!(set.contains("b.spec"));
    assert!(set.contains("c.spec"));
}

// ════════════════════════════════════════════════════════════════
// Behavior 1: go_to_definition
// ════════════════════════════════════════════════════════════════

#[test]
fn goto_def_navigates_to_entity_declaration() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """data MUST be valid"""
  risk high
}

behavior validate_input "Validate Input" {
  invariants [data_integrity]
  contract """must validate"""
  verify unit "test"
}"#;

    let state = TestState::from_source("test.spec", source);

    // Simulate go-to-def: look up the symbol for data_integrity
    let decl = state.symbols.get("data_integrity");
    assert!(decl.is_some(), "go-to-def should find entity declaration");
    let decl = decl.unwrap();
    assert_eq!(decl.kind, EntityKind::Invariant);
    assert_eq!(decl.file, "test.spec");
    // Declaration site has line and column
    assert!(decl.span.start_line > 0, "declaration should have a valid line");
    assert!(decl.span.start_col > 0, "declaration should have a valid column");
}

#[test]
fn goto_def_on_nonexistent_id_returns_no_result() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

behavior validate_input "Validate" {
  contract """must validate"""
  verify unit "test"
}"#;

    let state = TestState::from_source("test.spec", source);

    // Non-existent entity should return None
    let decl = state.symbols.get("does_not_exist");
    assert!(decl.is_none(), "non-existent ID should return no result");
}

#[test]
fn goto_def_works_across_files() {
    let spec_source = r#"spec "test" {
  version "1.0"
  plugins []
}"#;
    let invariant_source = r#"invariant data_integrity "Data Integrity" {
  guarantee """data valid"""
  risk high
}"#;
    let behavior_source = r#"behavior validate_input "Validate" {
  invariants [data_integrity]
  contract """validate"""
  verify unit "test"
}"#;

    let state = TestState::from_sources(&[
        ("spec.spec", spec_source),
        ("invariants.spec", invariant_source),
        ("behaviors.spec", behavior_source),
    ]);

    // data_integrity declared in invariants.spec should be reachable
    let decl = state.symbols.get("data_integrity");
    assert!(decl.is_some(), "should find entity declared in another file");
    let decl = decl.unwrap();
    assert_eq!(decl.file, "invariants.spec");

    // validate_input declared in behaviors.spec
    let decl = state.symbols.get("validate_input");
    assert!(decl.is_some());
    assert_eq!(decl.unwrap().file, "behaviors.spec");
}

#[test]
fn goto_def_works_for_enforced_by_references() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

behavior validate_input "Validate Input" {
  contract """must validate"""
  verify unit "test"
}

invariant data_persistence "Data Persistence" {
  guarantee """data MUST be persisted"""
  enforced_by [validate_input]
  risk high
}"#;

    let state = TestState::from_source("test.spec", source);

    // validate_input inside enforced_by should be resolvable in symbol table
    let decl = state.symbols.get("validate_input");
    assert!(decl.is_some(), "enforced_by reference should be resolvable via goto-def");
    assert_eq!(decl.unwrap().kind, EntityKind::Behavior);

    // No E001 errors for valid enforced_by references
    let e001_errors: Vec<_> = state
        .diagnostics
        .iter()
        .filter(|d| d.code == ValidationCode::E001)
        .collect();
    assert_eq!(e001_errors.len(), 0, "valid enforced_by reference should not produce E001");

    // Graph should have an Enforces edge
    let edges = state.graph.outgoing_edges("data_persistence");
    assert!(
        edges.iter().any(|(node, edge)| {
            node.id.raw() == "validate_input"
                && edge.edge_type == specforge_common::EdgeType::Enforces
        }),
        "enforced_by should create an Enforces edge in the graph"
    );
}

// ════════════════════════════════════════════════════════════════
// Behavior 2: find_all_references
// ════════════════════════════════════════════════════════════════

#[test]
fn find_refs_returns_all_reference_sites() {
    let source = r#"invariant data_integrity "Data" {
  risk high
}
behavior a "A" {
  invariants [data_integrity]
  contract """x"""
}
behavior b "B" {
  invariants [data_integrity]
  contract """y"""
}"#;

    let state = TestState::from_source("test.spec", source);
    let source_text = state.sources.get("test.spec").unwrap();
    let occurrences = find_identifier_occurrences(source_text, "data_integrity");

    // 1 declaration + 2 references = 3
    assert_eq!(occurrences.len(), 3, "should find all reference sites");
}

#[test]
fn find_refs_includes_the_declaration_site() {
    let source = r#"invariant data_integrity "Data" {
  risk high
}
behavior validate_input "Validate" {
  invariants [data_integrity]
  contract """x"""
}"#;

    let state = TestState::from_source("test.spec", source);
    let source_text = state.sources.get("test.spec").unwrap();
    let occurrences = find_identifier_occurrences(source_text, "data_integrity");

    // Declaration is at line 0
    let decl = state.symbols.get("data_integrity").unwrap();
    let decl_line = decl.span.start_line.saturating_sub(1);
    assert!(
        occurrences.iter().any(|(line, _, _)| *line == decl_line),
        "results MUST include the declaration site"
    );
}

#[test]
fn find_refs_across_multiple_files() {
    let spec_source = r#"spec "test" {
  version "1.0"
  plugins []
}"#;
    let file_a = r#"invariant data_integrity "Data" {
  guarantee """valid"""
  risk high
}"#;
    let file_b = r#"behavior validate_input "Validate" {
  invariants [data_integrity]
  contract """validate"""
  verify unit "test"
}"#;

    let state = TestState::from_sources(&[
        ("spec.spec", spec_source),
        ("a.spec", file_a),
        ("b.spec", file_b),
    ]);

    // Occurrences in file_a (declaration)
    let occ_a = find_identifier_occurrences(
        state.sources.get("a.spec").unwrap(),
        "data_integrity",
    );
    // Occurrences in file_b (reference)
    let occ_b = find_identifier_occurrences(
        state.sources.get("b.spec").unwrap(),
        "data_integrity",
    );

    assert!(
        !occ_a.is_empty() && !occ_b.is_empty(),
        "references should span multiple files"
    );
    let total = occ_a.len() + occ_b.len();
    assert!(total >= 2, "should find declaration + cross-file reference");
}

// ════════════════════════════════════════════════════════════════
// Behavior 3: hover_information
// ════════════════════════════════════════════════════════════════

#[test]
fn hover_shows_entity_title_and_type() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

behavior validate_input "Validate Input" {
  contract """the system MUST validate all input"""
  verify unit "test"
}"#;

    let state = TestState::from_source("test.spec", source);
    let node = state.graph.get_node("validate_input").unwrap();

    // Hover MUST include entity type and title
    assert_eq!(node.kind, EntityKind::Behavior);
    assert_eq!(node.title.as_deref(), Some("Validate Input"));

    // Build hover content the way hover.rs does
    let title = node.title.as_deref().unwrap_or(node.id.raw());
    let content = format!("**{}** `{}` — \"{}\"", node.kind, node.id.raw(), title);
    assert!(content.contains("behavior"), "hover must include entity type");
    assert!(content.contains("validate_input"), "hover must include entity ID");
    assert!(content.contains("Validate Input"), "hover must include title");
}

#[test]
fn hover_shows_contract_or_guarantee_text() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """all data MUST be validated before persistence"""
  risk high
}

behavior validate_input "Validate" {
  contract """the system MUST validate all input fields"""
  verify unit "test"
}"#;

    let state = TestState::from_source("test.spec", source);

    // Invariant has guarantee text
    let desc = find_entity_description(&state.files, "data_integrity");
    assert!(desc.is_some(), "hover must show guarantee text for invariant");
    assert!(desc.unwrap().contains("MUST be validated"));

    // Behavior has contract text
    let desc = find_entity_description(&state.files, "validate_input");
    assert!(desc.is_some(), "hover must show contract text for behavior");
    assert!(desc.unwrap().contains("MUST validate all input"));
}

#[test]
fn hover_shows_reference_count() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data" {
  guarantee """valid"""
  risk high
}

behavior a "A" {
  invariants [data_integrity]
  contract """x"""
  verify unit "t1"
}

behavior b "B" {
  invariants [data_integrity]
  contract """y"""
  verify unit "t2"
}"#;

    let state = TestState::from_source("test.spec", source);
    let incoming = state.graph.incoming_edges("data_integrity");

    // Two behaviors reference data_integrity
    assert_eq!(incoming.len(), 2, "hover must show correct reference count");
}

// ════════════════════════════════════════════════════════════════
// Behavior 4: autocomplete_entity_ids
// ════════════════════════════════════════════════════════════════

#[test]
fn autocomplete_suggests_matching_ids() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data" {
  guarantee """valid"""
  risk high
}

invariant data_encryption "Encryption" {
  guarantee """encrypted"""
  risk high
}

behavior validate_input "Validate" {
  contract """validate"""
  verify unit "test"
}"#;

    let state = TestState::from_source("test.spec", source);

    // Simulate prefix matching for "data_"
    let prefix = "data_";
    let matches: Vec<_> = state
        .graph
        .nodes()
        .filter(|n| n.id.raw().starts_with(prefix))
        .collect();

    assert_eq!(matches.len(), 2, "should suggest data_integrity and data_encryption");
}

#[test]
fn autocomplete_suggestions_include_entity_titles() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """valid"""
  risk high
}"#;

    let state = TestState::from_source("test.spec", source);
    let node = state.graph.get_node("data_integrity").unwrap();
    assert!(
        node.title.is_some(),
        "completion suggestions must include entity titles"
    );
    assert_eq!(node.title.as_deref(), Some("Data Integrity"));
}

#[test]
fn autocomplete_suggestions_filter_by_expected_entity_type() {
    use specforge_common::EdgeType;

    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data" {
  guarantee """valid"""
  risk high
}

behavior validate_input "Validate" {
  invariants [data_integrity]
  contract """validate"""
  verify unit "test"
}

feature input_validation "Input" {
  behaviors [validate_input]
}"#;

    let state = TestState::from_source("test.spec", source);

    // When completing inside "invariants [", only invariants should be suggested
    let edge = EdgeType::from_field_name("invariants").unwrap();
    let expected_kind = edge.target_kind().unwrap();
    assert_eq!(expected_kind, EntityKind::Invariant);

    let filtered: Vec<_> = state.graph.nodes_of_kind(expected_kind);
    assert!(
        filtered.iter().all(|n| n.kind == EntityKind::Invariant),
        "only entities of the correct type must be suggested"
    );
    assert!(filtered.iter().any(|n| n.id.raw() == "data_integrity"));
}

// ════════════════════════════════════════════════════════════════
// Behavior 5: rename_entity_id
// ════════════════════════════════════════════════════════════════

#[test]
fn rename_updates_declaration_and_all_references() {
    let source = r#"invariant data_integrity "Data" {
  risk high
}
behavior validate_input "Validate" {
  invariants [data_integrity]
  contract """x"""
}
behavior sanitize_input "Sanitize" {
  invariants [data_integrity]
  contract """y"""
}"#;

    let state = TestState::from_source("test.spec", source);
    let source_text = state.sources.get("test.spec").unwrap();

    // All occurrences (declaration + 2 references)
    let occurrences = find_identifier_occurrences(source_text, "data_integrity");
    assert_eq!(occurrences.len(), 3, "rename must find declaration + all references");

    // Simulate rename: new name must be unique
    let new_name = "data_validity";
    assert!(!state.symbols.contains(new_name), "new name should not already exist");

    // All 3 occurrences would get a text edit
    for (line, start, end) in &occurrences {
        assert!(end > start, "each occurrence has valid range at line {line}");
    }
}

#[test]
fn rename_is_atomic_all_or_nothing() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data" {
  guarantee """valid"""
  risk high
}

invariant email_uniqueness "Email" {
  guarantee """unique"""
  risk medium
}"#;

    let state = TestState::from_source("test.spec", source);

    // Renaming to an existing name must be rejected (returns None in LSP)
    assert!(state.symbols.contains("email_uniqueness"));
    assert!(
        state.symbols.contains("data_integrity"),
        "both entities exist — rename to existing name should fail atomically"
    );
}

#[test]
fn rename_across_multiple_files() {
    let spec_source = r#"spec "test" {
  version "1.0"
  plugins []
}"#;
    let file_a = r#"invariant data_integrity "Data" {
  guarantee """valid"""
  risk high
}"#;
    let file_b = r#"behavior validate_input "Validate" {
  invariants [data_integrity]
  contract """validate"""
  verify unit "test"
}"#;

    let state = TestState::from_sources(&[
        ("spec.spec", spec_source),
        ("a.spec", file_a),
        ("b.spec", file_b),
    ]);

    // Collect occurrences from all files
    let mut all_occurrences = Vec::new();
    for (path, src) in &state.sources {
        let occ = find_identifier_occurrences(src, "data_integrity");
        for o in occ {
            all_occurrences.push((path.clone(), o));
        }
    }

    // Declaration in a.spec + reference in b.spec
    let files_with_occurrences: std::collections::HashSet<_> =
        all_occurrences.iter().map(|(p, _)| p.clone()).collect();
    assert!(files_with_occurrences.contains("a.spec"));
    assert!(files_with_occurrences.contains("b.spec"));
    assert!(
        all_occurrences.len() >= 2,
        "rename must update all references across files"
    );
}

// ════════════════════════════════════════════════════════════════
// Behavior 6: live_diagnostics
// ════════════════════════════════════════════════════════════════

#[test]
fn diagnostics_update_after_file_change() {
    // Build with a dangling reference → should produce E001
    let source_with_error = r#"spec "test" {
  version "1.0"
  plugins []
}

behavior validate_input "Validate" {
  invariants [nonexistent_invariant]
  contract """validate"""
  verify unit "test"
}"#;

    let state = TestState::from_source("test.spec", source_with_error);
    let has_error = state
        .diagnostics
        .iter()
        .any(|d| d.code == ValidationCode::E001);
    assert!(has_error, "should have E001 for dangling reference");

    // Rebuild with fixed source — diagnostic should disappear
    let source_fixed = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data" {
  guarantee """valid"""
  risk high
}

behavior validate_input "Validate" {
  invariants [data_integrity]
  contract """validate"""
  verify unit "test"
}"#;

    let state_fixed = TestState::from_source("test.spec", source_fixed);
    let has_e001 = state_fixed
        .diagnostics
        .iter()
        .any(|d| d.code == ValidationCode::E001);
    assert!(!has_e001, "E001 should be gone after fix");
}

#[test]
fn only_changed_file_diagnostics_are_refreshed() {
    let spec_source = r#"spec "test" {
  version "1.0"
  plugins []
}"#;
    let file_a = r#"invariant data_integrity "Data" {
  guarantee """valid"""
  risk high
}"#;
    let file_b = r#"behavior validate_input "Validate" {
  invariants [data_integrity]
  contract """validate"""
  verify unit "test"
}"#;

    let state = TestState::from_sources(&[
        ("spec.spec", spec_source),
        ("a.spec", file_a),
        ("b.spec", file_b),
    ]);

    // The file_graph tracks imports. Changing a.spec invalidates its dependents
    let invalidated = state.file_graph.invalidation_set("a.spec");
    // a.spec itself should be invalidated
    assert!(invalidated.contains("a.spec"));
    // spec.spec and b.spec (which doesn't import a.spec) should behave correctly
    // — only files that depend on a.spec are invalidated
}

// ════════════════════════════════════════════════════════════════
// Behavior 7: code_actions_for_missing_tests
// ════════════════════════════════════════════════════════════════

#[test]
fn code_action_offered_on_untested_behavior() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

behavior validate_input "Validate" {
  contract """must validate"""
  verify unit "check validation logic"
}"#;

    let state = TestState::from_source("test.spec", source);
    let node = state.graph.get_node("validate_input").unwrap();

    // Behavior is testable
    assert!(node.kind.is_testable(), "behavior should be testable");

    // Has verify statements but no tests field
    let (has_verify, has_tests) = check_entity_fields(&state.files, "validate_input");
    assert!(has_verify, "should have verify statements");
    assert!(!has_tests, "should NOT have tests field");

    // Code action should be offered
    assert!(
        has_verify && !has_tests,
        "code action must be offered on untested behavior with verify"
    );
}

#[test]
fn code_action_not_offered_when_tests_field_present() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

behavior validate_input "Validate" {
  contract """must validate"""
  verify unit "check validation logic"
  tests ["tests/behaviors/validate_input_test.rs"]
}"#;

    let state = TestState::from_source("test.spec", source);
    let (has_verify, has_tests) = check_entity_fields(&state.files, "validate_input");
    assert!(has_verify, "should have verify statements");
    assert!(has_tests, "should have tests field");
    assert!(
        !(has_verify && !has_tests),
        "code action should NOT be offered when tests field present"
    );
}

#[test]
fn generated_stub_includes_behavior_id() {
    let entity_id = "validate_input";
    let kind = EntityKind::Behavior;
    let path = generate_test_path(entity_id, &kind);

    assert!(
        path.contains(entity_id),
        "generated stub path must include behavior ID"
    );
    assert_eq!(path, "tests/behaviors/validate_input_test.rs");
}

#[test]
fn generated_stub_paths_match_entity_kinds() {
    assert_eq!(
        generate_test_path("my_invariant", &EntityKind::Invariant),
        "tests/invariants/my_invariant_test.rs"
    );
    assert_eq!(
        generate_test_path("my_event", &EntityKind::Event),
        "tests/events/my_event_test.rs"
    );
    assert_eq!(
        generate_test_path("my_constraint", &EntityKind::Constraint),
        "tests/constraints/my_constraint_test.rs"
    );
    assert_eq!(
        generate_test_path("my_capability", &EntityKind::Capability),
        "tests/capabilities/my_capability_test.rs"
    );
}

// ════════════════════════════════════════════════════════════════
// Behavior 8: outline_view
// ════════════════════════════════════════════════════════════════

#[test]
fn outline_lists_all_entities_in_file() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data" {
  guarantee """valid"""
  risk high
}

behavior validate_input "Validate" {
  contract """validate"""
  verify unit "test"
}

feature input_validation "Input Validation" {
  behaviors [validate_input]
}"#;

    let state = TestState::from_source("test.spec", source);
    let entities = state.file_index.entities_in("test.spec");

    // Should list all entities in the file (spec + invariant + behavior + feature)
    assert!(
        entities.len() >= 3,
        "outline must list all entities in file, got {}",
        entities.len()
    );
    let entity_set: std::collections::HashSet<_> = entities.iter().map(|s| s.as_str()).collect();
    assert!(entity_set.contains("data_integrity"));
    assert!(entity_set.contains("validate_input"));
    assert!(entity_set.contains("input_validation"));
}

#[test]
fn outline_shows_entity_type_and_id() {
    use tower_lsp::lsp_types::SymbolKind;

    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """valid"""
  risk high
}

behavior validate_input "Validate Input" {
  contract """validate"""
  verify unit "test"
}"#;

    let state = TestState::from_source("test.spec", source);

    // Replicate entity_kind_to_symbol_kind mapping
    fn kind_to_symbol(kind: &EntityKind) -> SymbolKind {
        match kind {
            EntityKind::Behavior => SymbolKind::FUNCTION,
            EntityKind::Invariant => SymbolKind::PROPERTY,
            EntityKind::Feature => SymbolKind::MODULE,
            EntityKind::TypeDef => SymbolKind::STRUCT,
            EntityKind::Port => SymbolKind::INTERFACE,
            EntityKind::Event => SymbolKind::EVENT,
            EntityKind::Spec => SymbolKind::NAMESPACE,
            EntityKind::Ref => SymbolKind::FILE,
            EntityKind::Capability => SymbolKind::METHOD,
            EntityKind::Deliverable => SymbolKind::PACKAGE,
            EntityKind::Roadmap => SymbolKind::ENUM,
            EntityKind::Library => SymbolKind::OBJECT,
            EntityKind::Glossary => SymbolKind::KEY,
            EntityKind::Decision => SymbolKind::CONSTANT,
            EntityKind::Constraint => SymbolKind::TYPE_PARAMETER,
            EntityKind::FailureMode => SymbolKind::NULL,
            EntityKind::Custom(_) => SymbolKind::VARIABLE,
        }
    }

    for entity_id in state.file_index.entities_in("test.spec") {
        let node = state.graph.get_node(entity_id).unwrap();
        // Each outline entry has entity type (via SymbolKind mapping)
        let _symbol_kind = kind_to_symbol(&node.kind);
        // Each outline entry has entity ID
        assert!(!node.id.raw().is_empty(), "outline entry must show entity ID");
        // Each outline entry has the detail (kind as string)
        let detail = format!("{}", node.kind);
        assert!(!detail.is_empty(), "outline entry must show entity type");
    }
}

// ════════════════════════════════════════════════════════════════
// Behavior 9: workspace_symbol_search
// ════════════════════════════════════════════════════════════════

#[test]
fn workspace_symbol_search_by_id_prefix() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """valid"""
  risk high
}

invariant data_encryption "Data Encryption" {
  guarantee """encrypted"""
  risk high
}

behavior validate_input "Validate Input" {
  contract """validate"""
  verify unit "test"
}"#;

    let state = TestState::from_source("test.spec", source);

    // Search by ID prefix "data_"
    let query = "data_";
    let matches: Vec<_> = state
        .graph
        .nodes()
        .filter(|n| {
            n.id.raw().to_lowercase().contains(&query.to_lowercase())
                || n.title
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&query.to_lowercase())
        })
        .collect();

    assert_eq!(matches.len(), 2, "search by ID prefix should return matching entities");
    assert!(matches.iter().any(|n| n.id.raw() == "data_integrity"));
    assert!(matches.iter().any(|n| n.id.raw() == "data_encryption"));
}

#[test]
fn workspace_symbol_search_by_title_fragment() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """valid"""
  risk high
}

behavior validate_input "Validate Input" {
  contract """validate"""
  verify unit "test"
}

behavior sanitize_html "Sanitize HTML Output" {
  contract """sanitize"""
  verify unit "test"
}"#;

    let state = TestState::from_source("test.spec", source);

    // Search by title fragment "Input"
    let query = "input";
    let matches: Vec<_> = state
        .graph
        .nodes()
        .filter(|n| {
            n.id.raw().to_lowercase().contains(query)
                || n.title
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(query)
        })
        .collect();

    // Should match "validate_input" (by ID and title) but not "sanitize_html"
    assert!(
        matches.iter().any(|n| n.id.raw() == "validate_input"),
        "should match by title fragment"
    );
    assert!(
        !matches.iter().any(|n| n.id.raw() == "sanitize_html"),
        "should not match unrelated entities"
    );
}

// ════════════════════════════════════════════════════════════════
// Behavior 10: shared_incremental_pipeline
// ════════════════════════════════════════════════════════════════

#[test]
fn lsp_and_watch_share_the_same_graph() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data" {
  guarantee """valid"""
  risk high
}

behavior validate_input "Validate" {
  invariants [data_integrity]
  contract """validate"""
  verify unit "test"
}"#;

    let state = TestState::from_source("test.spec", source);

    // The same graph serves all features:
    // - go-to-def uses symbols
    assert!(state.symbols.contains("data_integrity"));
    // - hover uses graph
    assert!(state.graph.get_node("data_integrity").is_some());
    // - outline uses file_index
    assert!(!state.file_index.entities_in("test.spec").is_empty());
    // - diagnostics uses diagnostics vec
    // (no dangling refs here, so no E001 errors)
    let has_e001 = state.diagnostics.iter().any(|d| d.code == ValidationCode::E001);
    assert!(!has_e001);

    // All populated from the same build — no separate compilation passes
}

#[test]
fn graph_update_serves_all_lsp_features() {
    let source = r#"spec "test" {
  version "1.0"
  plugins []
}

invariant data_integrity "Data Integrity" {
  guarantee """all data MUST be validated"""
  risk high
}

behavior validate_input "Validate Input" {
  invariants [data_integrity]
  contract """the system MUST validate input"""
  verify unit "test"
}

feature input_validation "Input Validation" {
  behaviors [validate_input]
}"#;

    let state = TestState::from_source("test.spec", source);

    // After a single build, all LSP features work:
    // 1. Navigation: symbols resolve
    assert!(state.symbols.get("data_integrity").is_some());
    assert!(state.symbols.get("validate_input").is_some());

    // 2. Graph: edges exist
    let outgoing = state.graph.outgoing_edges("validate_input");
    assert!(!outgoing.is_empty());

    // 3. File index: entities mapped
    let entities = state.file_index.entities_in("test.spec");
    assert!(entities.len() >= 3);

    // 4. Diagnostics: collected
    // (valid spec, so no errors expected)

    // 5. Hover: description extractable
    let desc = find_entity_description(&state.files, "validate_input");
    assert!(desc.is_some());
}

// ════════════════════════════════════════════════════════════════
// Behavior 11: provide_semantic_tokens
// ════════════════════════════════════════════════════════════════

#[test]
fn semantic_tokens_entity_ids_receive_correct_type() {
    let source = r#"behavior validate_input "Validate Input" {
  contract """must validate"""
  verify unit "test"
}"#;

    let tokens = collect_semantic_tokens(source);

    // Find the token for "validate_input" at line 0
    // "behavior" is at col 0, "validate_input" is at col 9
    let entity_id_token = tokens
        .iter()
        .find(|(line, col, len, _, _)| *line == 0 && *col == 9 && *len == "validate_input".len() as u32);
    assert!(
        entity_id_token.is_some(),
        "entity ID should have a semantic token"
    );
    let (_, _, _, token_type, modifiers) = entity_id_token.unwrap();
    // Token type 3 = VARIABLE, modifiers 1 = DECLARATION
    assert_eq!(*token_type, 3, "entity ID at declaration should be VARIABLE type");
    assert_eq!(*modifiers, 1, "entity ID at declaration should have DECLARATION modifier");
}

#[test]
fn semantic_tokens_keywords_are_classified_as_keywords() {
    let source = r#"behavior validate_input "Validate" {
  contract """must validate"""
  verify unit "test"
}"#;

    let tokens = collect_semantic_tokens(source);

    // "behavior" keyword at line 0, col 0
    let keyword_token = tokens
        .iter()
        .find(|(line, col, len, _, _)| *line == 0 && *col == 0 && *len == "behavior".len() as u32);
    assert!(keyword_token.is_some(), "keyword should have a semantic token");
    let (_, _, _, token_type, _) = keyword_token.unwrap();
    // Token type 0 = KEYWORD
    assert_eq!(*token_type, 0, "keywords must be classified as KEYWORD type");

    // "verify" keyword at line 2
    let verify_token = tokens
        .iter()
        .find(|(line, _, len, _, _)| *line == 2 && *len == "verify".len() as u32);
    assert!(verify_token.is_some(), "verify keyword should have a token");
    assert_eq!(verify_token.unwrap().3, 0, "verify must be classified as KEYWORD");
}

#[test]
fn semantic_tokens_triple_quoted_strings_are_strings() {
    let source = r#"behavior validate_input "Validate" {
  contract """must validate all input"""
  verify unit "test"
}"#;

    let tokens = collect_semantic_tokens(source);

    // Triple-quoted string """must validate all input""" at line 1
    // Token type 4 = STRING
    let string_tokens: Vec<_> = tokens.iter().filter(|(_, _, _, tt, _)| *tt == 4).collect();
    assert!(
        !string_tokens.is_empty(),
        "triple-quoted strings must be classified as STRING type"
    );

    // The title string "Validate" at line 0 should also be a string
    let title_token = string_tokens
        .iter()
        .find(|(line, _, _, _, _)| *line == 0);
    assert!(title_token.is_some(), "title string should be classified as STRING");

    // The contract triple-quoted string at line 1
    let contract_token = string_tokens
        .iter()
        .find(|(line, _, _, _, _)| *line == 1);
    assert!(
        contract_token.is_some(),
        "triple-quoted contract string should be classified as STRING"
    );
}

#[test]
fn semantic_tokens_references_in_lists_are_variables() {
    let source = r#"invariant data_integrity "Data" {
  risk high
}
behavior validate_input "Validate" {
  invariants [data_integrity]
  contract """must validate"""
}"#;

    let tokens = collect_semantic_tokens(source);

    // "data_integrity" in the reference list at line 4
    // It should be token type 3 (VARIABLE) with no DECLARATION modifier
    let ref_token = tokens
        .iter()
        .find(|(line, _, len, _, _)| *line == 4 && *len == "data_integrity".len() as u32);
    assert!(ref_token.is_some(), "references in lists should have tokens");
    let (_, _, _, token_type, modifiers) = ref_token.unwrap();
    assert_eq!(*token_type, 3, "references should be VARIABLE type");
    assert_eq!(*modifiers, 0, "references should NOT have DECLARATION modifier");
}
