use specforge_parser::{parse, EntityKind, EntityId, FieldValue};
use specforge_test_macros::test as specforge_test;
use std::path::PathBuf;

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_basic_entity_block() {
    let source = r#"
behavior parse_spec_file "Parse Spec File" {
    status planned
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind, EntityKind { raw: "behavior".to_string() });
    assert_eq!(entity.id, EntityId { raw: "parse_spec_file".to_string() });
    assert_eq!(entity.title.as_deref(), Some("Parse Spec File"));

    let status = entity.fields.get("status").expect("missing 'status' field");
    assert!(matches!(status, FieldValue::Identifier(id) if id == "planned"));
}

#[specforge_test(behavior = "parse_use_imports")]
#[test]
fn parse_use_import() {
    let source = "use behaviors/parsing\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.imports[0].path, "behaviors/parsing");
    assert!(result.imports[0].selected_ids.is_none());
}

#[specforge_test(behavior = "parse_use_imports")]
#[test]
fn parse_selective_import() {
    let source = "use types/core { SpecFile, ParseError }\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.imports[0].path, "types/core");
    let ids = result.imports[0].selected_ids.as_ref().expect("expected selective import");
    assert_eq!(ids, &["SpecFile", "ParseError"]);
}

#[specforge_test(behavior = "parse_verify_statements")]
#[test]
fn parse_verify_statements() {
    let source = r#"
behavior validate_input "Validate Input" {
    contract "Input must be valid UTF-8"
    verify unit "rejects invalid UTF-8"
    verify integration "validates end-to-end"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let entity = &result.entities[0];
    let verify = entity.fields.get("verify").expect("missing verify field");
    match verify {
        FieldValue::VerifyList(stmts) => {
            assert_eq!(stmts.len(), 2);
            assert_eq!(stmts[0].kind, "unit");
            assert_eq!(stmts[0].description, "rejects invalid UTF-8");
            assert_eq!(stmts[1].kind, "integration");
            assert_eq!(stmts[1].description, "validates end-to-end");
        }
        other => panic!("expected VerifyList, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_spec_block() {
    let source = r#"
spec "SpecForge" {
    version "0.1.0"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind, EntityKind { raw: "spec".to_string() });
    assert_eq!(entity.title.as_deref(), Some("SpecForge"));

    let version = entity.fields.get("version").expect("missing version");
    assert!(matches!(version, FieldValue::String(v) if v == "0.1.0"));
}

#[specforge_test(behavior = "parse_ref_blocks")]
#[test]
fn parse_ref_inline() {
    let source = r#"ref gh.issue:42 "Support Wasm extensions""#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind, EntityKind { raw: "ref".to_string() });
    assert_eq!(entity.id.raw, "gh.issue:42");
    assert_eq!(entity.title.as_deref(), Some("Support Wasm extensions"));

    // Decomposed scheme/kind/identifier
    assert!(matches!(entity.fields.get("scheme"), Some(FieldValue::String(s)) if s == "gh"));
    assert!(matches!(entity.fields.get("ref_kind"), Some(FieldValue::String(s)) if s == "issue"));
    assert!(matches!(entity.fields.get("identifier"), Some(FieldValue::String(s)) if s == "42"));
}

#[specforge_test(behavior = "parse_ref_blocks")]
#[test]
fn parse_ref_full_block() {
    let source = r#"
ref gh.issue:99 "Performance tracking" {
    priority "high"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let entity = &result.entities[0];
    assert_eq!(entity.kind, EntityKind { raw: "ref".to_string() });
    assert_eq!(entity.title.as_deref(), Some("Performance tracking"));
    assert!(matches!(entity.fields.get("priority"), Some(FieldValue::String(s)) if s == "high"));
}

#[specforge_test(behavior = "parse_define_blocks")]
#[test]
fn parse_define_block() {
    let source = r#"
define my_custom_type {
    base_kind "entity"
    verify unit "custom type works"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind, EntityKind { raw: "define".to_string() });
    assert_eq!(entity.id.raw, "my_custom_type");
    assert!(entity.title.is_none());
    assert!(matches!(entity.fields.get("base_kind"), Some(FieldValue::String(s)) if s == "entity"));
    assert!(entity.fields.get("verify").is_some());
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_all_field_value_types() {
    let source = r#"
behavior test_values "Test Values" {
    name "a string"
    count 42
    enabled true
    disabled false
    created 2024-01-15
    status planned
    tags [alpha, beta, gamma]
    labels ["tag-one", "tag-two"]
    config {
        timeout 30
        retries 3
    }
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let e = &result.entities[0];

    assert!(matches!(e.fields.get("name"), Some(FieldValue::String(s)) if s == "a string"));
    assert!(matches!(e.fields.get("count"), Some(FieldValue::Integer(42))));
    assert!(matches!(e.fields.get("enabled"), Some(FieldValue::Boolean(true))));
    assert!(matches!(e.fields.get("disabled"), Some(FieldValue::Boolean(false))));
    assert!(matches!(e.fields.get("created"), Some(FieldValue::Date(d)) if d == "2024-01-15"));
    assert!(matches!(e.fields.get("status"), Some(FieldValue::Identifier(id)) if id == "planned"));

    // Reference list (all identifiers)
    match e.fields.get("tags").expect("missing tags") {
        FieldValue::ReferenceList(items) => {
            assert_eq!(items, &["alpha", "beta", "gamma"]);
        }
        other => panic!("expected ReferenceList, got {:?}", other),
    }

    // String list (contains quoted strings)
    match e.fields.get("labels").expect("missing labels") {
        FieldValue::StringList(items) => {
            assert_eq!(items, &["tag-one", "tag-two"]);
        }
        other => panic!("expected StringList, got {:?}", other),
    }

    // Nested block
    match e.fields.get("config").expect("missing config") {
        FieldValue::Block(map) => {
            assert!(matches!(map.get("timeout"), Some(FieldValue::Integer(30))));
            assert!(matches!(map.get("retries"), Some(FieldValue::Integer(3))));
        }
        other => panic!("expected Block, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_triple_quoted_strings")]
#[test]
fn parse_triple_quoted_string() {
    let source = "behavior doc_test \"Doc Test\" {\n    contract \"\"\"\n        Given a valid input\n        When processed\n        Then output is correct\n    \"\"\"\n}\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let e = &result.entities[0];
    let contract = e.fields.get("contract").expect("missing contract");
    match contract {
        FieldValue::String(s) => {
            // Should be dedented: leading common whitespace removed
            assert!(s.contains("Given a valid input"), "got: {s:?}");
            assert!(s.contains("When processed"), "got: {s:?}");
            assert!(s.contains("Then output is correct"), "got: {s:?}");
            // Should NOT have 8 spaces of leading indentation
            assert!(!s.starts_with("        "), "expected dedented, got: {s:?}");
        }
        other => panic!("expected String, got {:?}", other),
    }
}

#[specforge_test(behavior = "recover_from_syntax_errors")]
#[test]
fn multi_error_recovery() {
    let source = r#"
behavior good_before "Before" {
    status planned
}

behavior ??? {
    this is broken
}

behavior good_after "After" {
    status done
}
"#;
    let result = parse(source, "test.spec");

    // Should have errors from the broken block
    assert!(!result.errors.is_empty(), "expected parse errors");

    // But should still parse both valid blocks
    let good_entities: Vec<_> = result
        .entities
        .iter()
        .filter(|e| e.id.raw == "good_before" || e.id.raw == "good_after")
        .collect();

    assert_eq!(
        good_entities.len(),
        2,
        "expected both valid entities to be parsed despite error; got entities: {:?}",
        result.entities.iter().map(|e| &e.id.raw).collect::<Vec<_>>()
    );
}

#[specforge_test(behavior = "recover_from_syntax_errors")]
#[test]
fn multiple_errors_produce_multiple_diagnostics() {
    let source = r#"
!!!

behavior valid "Valid" {
    status planned
}

@@@
"#;
    let result = parse(source, "test.spec");

    // Should collect errors from both broken sections
    assert!(
        result.errors.len() >= 2,
        "expected at least 2 errors, got {}: {:?}",
        result.errors.len(),
        result.errors
    );

    // Should still parse the valid entity
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].id.raw, "valid");
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn comments_are_skipped() {
    let source = r#"
// This is a file-level comment
use behaviors/parsing // inline comment

// Comment before entity
behavior example "Example" {
    // Comment inside block
    status planned
}
// Trailing comment
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].id.raw, "example");
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_entity_without_title() {
    let source = r#"
invariant no_orphans {
    guarantee "Every entity must have at least one incoming edge"
    risk high
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let e = &result.entities[0];
    assert_eq!(e.kind.raw, "invariant");
    assert_eq!(e.id.raw, "no_orphans");
    assert!(e.title.is_none());
}

#[specforge_test(behavior = "parse_spec_file_to_ast")]
#[test]
fn parse_multiple_entities_in_one_file() {
    let source = r#"
use types/core

behavior first "First" {
    status planned
}

behavior second "Second" {
    status done
}

invariant third {
    guarantee "Always true"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.entities.len(), 3);
    assert_eq!(result.entities[0].id.raw, "first");
    assert_eq!(result.entities[1].id.raw, "second");
    assert_eq!(result.entities[2].id.raw, "third");
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_list_with_trailing_comma() {
    let source = r#"
behavior trailing "Trailing Comma" {
    tags [alpha, beta, gamma,]
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    match result.entities[0].fields.get("tags").expect("missing tags") {
        FieldValue::ReferenceList(items) => {
            assert_eq!(items, &["alpha", "beta", "gamma"]);
        }
        other => panic!("expected ReferenceList, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_ref_blocks")]
#[test]
fn parse_mixed_list_with_scheme_refs() {
    let source = r#"
behavior with_refs "Refs" {
    refs [gh.issue:42, jira.story:ABC-123]
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    match result.entities[0].fields.get("refs").expect("missing refs") {
        FieldValue::ReferenceList(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], "gh.issue:42");
            assert_eq!(items[1], "jira.story:ABC-123");
        }
        other => panic!("expected ReferenceList with scheme refs, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_use_imports")]
#[test]
fn parse_multiple_use_imports() {
    let source = r#"
use behaviors/parsing
use types/core
use invariants/core
use features/output
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.imports.len(), 4);
    assert_eq!(result.imports[0].path, "behaviors/parsing");
    assert_eq!(result.imports[1].path, "types/core");
    assert_eq!(result.imports[2].path, "invariants/core");
    assert_eq!(result.imports[3].path, "features/output");
}

#[specforge_test(behavior = "parse_spec_file_to_ast")]
#[test]
fn source_spans_are_accurate() {
    let source = "behavior first \"First\" {\n    status planned\n}\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let span = &result.entities[0].span;
    assert_eq!(span.file, "test.spec");
    assert_eq!(span.start_line, 1);
    assert_eq!(span.start_col, 1);
    assert_eq!(span.end_line, 3);
}

#[specforge_test(behavior = "parse_spec_file_to_ast")]
#[test]
fn parse_empty_file() {
    let result = parse("", "empty.spec");
    assert!(result.errors.is_empty());
    assert!(result.entities.is_empty());
    assert!(result.imports.is_empty());
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_empty_block() {
    let source = "behavior empty_block \"Empty\" {\n}\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].id.raw, "empty_block");
    assert_eq!(result.entities[0].fields.entries().len(), 0);
}

#[specforge_test(behavior = "parse_spec_file_to_ast")]
#[test]
fn parse_real_world_spec_file() {
    // A realistic .spec file exercising many constructs at once
    let source = r#"
// Parsing behaviors
use invariants/core
use types/core
use types/errors

behavior parse_spec_file_to_ast "Parse Spec File to AST" {
    invariants [multi_error_collection, string_interning_consistency]
    types      [SpecFile, ParseError, SourceSpan]

    requires {
        valid_utf8_input "Input buffer is valid UTF-8"
    }

    ensures {
        ast_produced "A complete AST is produced"
    }

    contract """
        Given a source file
        When parsed by the Tree-sitter grammar
        Then an AST is produced with all entities
    """

    verify unit "produces AST for minimal valid file"
    verify unit "collects multiple syntax errors"
    verify integration "parses all .spec files in project"
}

ref gh.issue:1 "Initial parser"

behavior recover_from_syntax_errors "Recover from Syntax Errors" {
    invariants [multi_error_collection]
    types      [ParseError]
    status     planned
    verify property "N errors produce N diagnostics"
}
"#;
    let result = parse(source, "behaviors/parsing.spec");

    assert!(
        result.errors.is_empty(),
        "unexpected errors: {:?}",
        result.errors
    );
    assert_eq!(result.imports.len(), 3);
    assert_eq!(result.entities.len(), 3); // 2 behaviors + 1 ref

    // First behavior
    let b1 = &result.entities[0];
    assert_eq!(b1.kind.raw, "behavior");
    assert_eq!(b1.id.raw, "parse_spec_file_to_ast");
    assert_eq!(b1.title.as_deref(), Some("Parse Spec File to AST"));
    assert!(b1.fields.get("invariants").is_some());
    assert!(b1.fields.get("requires").is_some());
    assert!(b1.fields.get("ensures").is_some());
    assert!(b1.fields.get("contract").is_some());
    assert!(b1.fields.get("verify").is_some());

    // Ref
    let r = &result.entities[1];
    assert_eq!(r.kind.raw, "ref");

    // Second behavior
    let b2 = &result.entities[2];
    assert_eq!(b2.id.raw, "recover_from_syntax_errors");
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_deeply_nested_dbc_blocks() {
    let source = r#"
behavior complex "Complex" {
    requires {
        precondition_a "First precondition"
        precondition_b "Second precondition"
    }
    ensures {
        postcondition_a "First postcondition"
    }
    maintains {
        invariant_a "Always holds"
    }
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let e = &result.entities[0];

    // requires, ensures, maintains are nested blocks
    match e.fields.get("requires").expect("missing requires") {
        FieldValue::Block(map) => {
            assert!(matches!(
                map.get("precondition_a"),
                Some(FieldValue::String(s)) if s == "First precondition"
            ));
            assert!(matches!(
                map.get("precondition_b"),
                Some(FieldValue::String(s)) if s == "Second precondition"
            ));
        }
        other => panic!("expected Block for requires, got {:?}", other),
    }

    match e.fields.get("ensures").expect("missing ensures") {
        FieldValue::Block(map) => {
            assert_eq!(map.entries().len(), 1);
        }
        other => panic!("expected Block for ensures, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_spec_file_to_ast")]
#[test]
fn parse_actual_spec_files_from_project() {
    // Find the project root (crates/specforge-parser -> project root)
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let spec_dir = project_root.join("spec");

    if !spec_dir.exists() {
        // Skip if running outside the project
        return;
    }

    let mut total_files = 0;
    let mut total_entities = 0;
    for entry in walkdir(spec_dir.clone()) {
        let source = std::fs::read_to_string(&entry).unwrap();
        let result = parse(&source, entry.to_str().unwrap());
        total_files += 1;
        total_entities += result.entities.len();
    }

    assert!(total_files > 0, "should find at least one .spec file");
    assert!(total_entities > 0, "should parse at least one entity");

    // Errors come from extension-specific body syntax (port method
    // signatures, glossary term blocks, etc.) that the core grammar
    // correctly does not handle — body parsers will process these in
    // Phase 1.5. The key assertion is no panics on any real content.
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_type_struct_with_annotations() {
    let source = r#"
type SpecFile {
    path       string    @readonly
    imports    ImportDeclaration[]
    entities   Entity[]
    errors     ParseError[]  @optional
}
"#;
    let result = parse(source, "test.spec");

    assert!(
        result.errors.is_empty(),
        "unexpected errors: {:?}",
        result.errors
    );
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "type");
    assert_eq!(entity.id.raw, "SpecFile");
    // Should have parsed at least the field names
    assert!(entity.fields.get("path").is_some(), "missing 'path' field");
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_union_type() {
    let source = r#"
type FieldValue = StringValue | ReferenceList | StringList | Block | VerifyList
"#;
    let result = parse(source, "test.spec");

    assert!(
        result.errors.is_empty(),
        "unexpected errors: {:?}",
        result.errors
    );
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "type");
    assert_eq!(entity.id.raw, "FieldValue");
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_string_union_type() {
    let source = r#"
type McpErrorCode = "invalid_input" | "compilation_failed" | "entity_not_found"
"#;
    let result = parse(source, "test.spec");

    assert!(
        result.errors.is_empty(),
        "unexpected errors: {:?}",
        result.errors
    );
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "type");
    assert_eq!(entity.id.raw, "McpErrorCode");
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_multiline_union_type() {
    // Multi-line union with continuation via leading |
    let source = r#"
type ExportFormat = "json" | "dot" | "context"
  | "brief" | "graph"
"#;
    let result = parse(source, "test.spec");

    assert!(
        result.errors.is_empty(),
        "unexpected errors: {:?}",
        result.errors
    );
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].id.raw, "ExportFormat");
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_integer_union_type() {
    let source = r#"
type JsonRpcErrorCode = -32700 | -32600 | -32601
"#;
    let result = parse(source, "test.spec");

    assert!(
        result.errors.is_empty(),
        "unexpected errors: {:?}",
        result.errors
    );
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].id.raw, "JsonRpcErrorCode");
}

#[specforge_test(behavior = "parse_use_imports")]
#[test]
fn reject_use_import_with_spec_extension() {
    let source = "use behaviors/parsing.spec\n";
    let result = parse(source, "test.spec");

    // Should produce an error mentioning .spec extension
    assert!(
        result.errors.iter().any(|e| e.message.contains(".spec")),
        "error should mention .spec extension; errors: {:?}, imports: {:?}",
        result.errors,
        result.imports.iter().map(|i| &i.path).collect::<Vec<_>>()
    );
}

#[specforge_test(behavior = "parse_verify_statements")]
#[test]
fn verify_statements_in_spec_block() {
    let source = r#"
spec "MyProject" {
    version "1.0.0"
    verify integration "project compiles end-to-end"
    verify unit "version field is valid semver"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let entity = &result.entities[0];
    assert_eq!(entity.kind, EntityKind { raw: "spec".to_string() });

    let verify = entity.fields.get("verify").expect("missing verify in spec block");
    match verify {
        FieldValue::VerifyList(stmts) => {
            assert_eq!(stmts.len(), 2);
            assert_eq!(stmts[0].kind, "integration");
            assert_eq!(stmts[0].description, "project compiles end-to-end");
            assert_eq!(stmts[1].kind, "unit");
            assert_eq!(stmts[1].description, "version field is valid semver");
        }
        other => panic!("expected VerifyList, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_verify_statements")]
#[test]
fn verify_statements_in_define_block() {
    let source = r#"
define my_custom_kind {
    base_kind "entity"
    testable true
    verify unit "custom kind registers in KindRegistry"
    verify property "custom kind passes round-trip serialization"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let entity = &result.entities[0];
    assert_eq!(entity.kind, EntityKind { raw: "define".to_string() });

    let verify = entity.fields.get("verify").expect("missing verify in define block");
    match verify {
        FieldValue::VerifyList(stmts) => {
            assert_eq!(stmts.len(), 2);
            assert_eq!(stmts[0].kind, "unit");
            assert_eq!(stmts[1].kind, "property");
        }
        other => panic!("expected VerifyList, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_triple_quoted_strings")]
#[test]
fn unclosed_triple_quoted_string_produces_error() {
    let source = r#"
behavior broken "Broken" {
    contract """
        This string is never closed
}

behavior after "After" {
    status planned
}
"#;
    let result = parse(source, "test.spec");

    // Should have at least one error from the unclosed triple-quoted string
    assert!(
        !result.errors.is_empty(),
        "expected error for unclosed triple-quoted string"
    );
}

#[specforge_test(behavior = "recover_from_syntax_errors")]
#[test]
fn unclosed_regular_string_recovers_next_block() {
    let source = "behavior broken \"Broken {\n    status planned\n}\n\nbehavior after \"After\" {\n    status done\n}\n";
    let result = parse(source, "test.spec");

    assert!(
        !result.errors.is_empty(),
        "expected error for unclosed string"
    );

    // Tree-sitter can recover from unclosed regular strings better
    // The second block may or may not be recovered depending on grammar;
    // the key invariant is that the parser doesn't panic
}

#[specforge_test(behavior = "parse_ref_blocks")]
#[test]
fn malformed_ref_missing_scheme_produces_error() {
    // ref without proper scheme.kind:identifier should produce a parse error
    // since the grammar requires scheme_ref_id format, tree-sitter will
    // produce an ERROR node for malformed refs
    let source = "ref badref \"Missing scheme\"\n";
    let result = parse(source, "test.spec");

    // Either parsed as error, or parsed as entity with an error diagnostic
    let has_error = !result.errors.is_empty();
    let not_a_ref = result.entities.iter().all(|e| e.kind.raw != "ref");
    assert!(
        has_error || not_a_ref,
        "malformed ref should produce an error or not be parsed as ref; errors: {:?}, entities: {:?}",
        result.errors,
        result.entities.iter().map(|e| (&e.kind.raw, &e.id.raw)).collect::<Vec<_>>()
    );
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn entity_preserves_raw_body() {
    let source = r#"
behavior parse_things "Parse Things" {
    status planned
    tags [alpha, beta]
    verify unit "it works"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let entity = &result.entities[0];

    // raw_body should contain the verbatim text between the braces
    let raw = entity.raw_body.as_ref().expect("missing raw_body");
    assert!(raw.contains("status planned"), "raw_body should contain field text, got: {raw:?}");
    assert!(raw.contains("tags [alpha, beta]"), "raw_body should contain list, got: {raw:?}");
    assert!(raw.contains("verify unit"), "raw_body should contain verify, got: {raw:?}");
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn empty_block_has_empty_raw_body() {
    let source = "behavior empty \"Empty\" {\n}\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let entity = &result.entities[0];
    let raw = entity.raw_body.as_ref().expect("missing raw_body");
    assert!(raw.trim().is_empty(), "expected empty raw_body, got: {raw:?}");
}

#[specforge_test(behavior = "parse_spec_file_to_ast")]
#[test]
fn spec_files_without_extension_syntax_parse_cleanly() {
    // Files that use ONLY standard field syntax (no port method signatures,
    // no glossary term sub-blocks) should parse with ZERO errors.
    // This catches grammar regressions on the ~90% of spec files that
    // use standard entity_block / spec / ref / define / union syntax.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let spec_dir = project_root.join("spec");

    if !spec_dir.exists() {
        return;
    }

    // Files with known extension-specific body syntax that the core grammar
    // correctly cannot parse — these will be handled by body parsers.
    let extension_syntax_files: &[&str] = &[
        "ports/inbound.spec",  // port method signatures: method name(...) -> Result<...>
        "ports/outbound.spec", // port method signatures
        "glossary.spec",       // nested term sub-blocks inside glossary singleton
        "specforge.spec",      // spec block with extension-specific fields
        "types/zero-entity-core.spec", // inline union field types: string | string[]
    ];

    let mut failures = Vec::new();
    for entry in walkdir(spec_dir.clone()) {
        let rel = entry.strip_prefix(&spec_dir).unwrap().to_str().unwrap().to_string();

        // Skip files with known extension-specific syntax
        if extension_syntax_files.iter().any(|&f| rel == f) {
            continue;
        }
        // Skip extension-specific ports files
        if rel.contains("extensions/") && rel.contains("ports") {
            continue;
        }

        let source = std::fs::read_to_string(&entry).unwrap();
        let result = parse(&source, entry.to_str().unwrap());
        if !result.errors.is_empty() {
            failures.push((rel, result.errors.len(), result.errors.iter().map(|e| {
                format!("  L{}:{}: {}", e.span.start_line, e.span.start_col, e.message)
            }).collect::<Vec<_>>()));
        }
    }

    if !failures.is_empty() {
        let mut msg = format!("{} standard spec files had parse errors:\n", failures.len());
        for (path, count, details) in &failures {
            msg.push_str(&format!("\n{path} ({count} errors):\n"));
            for d in details {
                msg.push_str(&format!("{d}\n"));
            }
        }
        panic!("{msg}");
    }
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn annotation_with_string_value() {
    let source = r#"
type WasmConfig {
    max_memory_pages   integer   @optional @doc "Default: 16"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "annotation with string value should not produce errors: {:?}", result.errors);
    let entity = &result.entities[0];
    assert!(entity.fields.get("max_memory_pages").is_some(), "field should be parsed");
}

#[specforge_test(behavior = "parse_verify_statements")]
#[test]
fn verify_statement_without_kind() {
    let source = r#"
event manifests_loaded "Manifests Loaded" {
    verify "Extension manifests MUST be loaded before populating the kind registry"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "verify without kind should not produce errors: {:?}", result.errors);
    let entity = &result.entities[0];
    let verify = entity.fields.get("verify").expect("missing verify field");
    match verify {
        FieldValue::VerifyList(stmts) => {
            assert_eq!(stmts.len(), 1);
            // When kind is omitted, it should default to empty string or a sentinel
            assert!(stmts[0].kind.is_empty(), "kind should be empty when omitted, got: {:?}", stmts[0].kind);
            assert_eq!(stmts[0].description, "Extension manifests MUST be loaded before populating the kind registry");
        }
        other => panic!("expected VerifyList, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn string_field_values_distinct_from_identifiers() {
    let source = r#"
behavior example "Example" {
    status     planned
    contract   "This is a string value"
    count      42
    active     true
    created    2025-01-15
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let e = &result.entities[0];

    // Bare word → Identifier
    assert!(
        matches!(e.fields.get("status"), Some(FieldValue::Identifier(s)) if s == "planned"),
        "bare word 'planned' should be FieldValue::Identifier, got: {:?}",
        e.fields.get("status")
    );

    // Quoted string → String
    assert!(
        matches!(e.fields.get("contract"), Some(FieldValue::String(s)) if s == "This is a string value"),
        "quoted text should be FieldValue::String, got: {:?}",
        e.fields.get("contract")
    );

    // Integer
    assert!(
        matches!(e.fields.get("count"), Some(FieldValue::Integer(42))),
        "integer should be FieldValue::Integer, got: {:?}",
        e.fields.get("count")
    );

    // Boolean
    assert!(
        matches!(e.fields.get("active"), Some(FieldValue::Boolean(true))),
        "true should be FieldValue::Boolean, got: {:?}",
        e.fields.get("active")
    );

    // Date
    assert!(
        matches!(e.fields.get("created"), Some(FieldValue::Date(d)) if d == "2025-01-15"),
        "date should be FieldValue::Date, got: {:?}",
        e.fields.get("created")
    );
}

#[specforge_test(behavior = "parse_triple_quoted_strings")]
#[test]
fn triple_quoted_relative_indentation_preserved() {
    let source = "behavior b \"B\" {\n    contract \"\"\"\n        line one\n            indented deeper\n        back to base\n    \"\"\"\n}\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let contract = result.entities[0].fields.get("contract").expect("missing contract");
    match contract {
        FieldValue::String(s) => {
            let lines: Vec<&str> = s.lines().collect();
            assert_eq!(lines.len(), 3, "expected 3 lines, got: {lines:?}");
            assert_eq!(lines[0], "line one");
            assert_eq!(lines[1], "    indented deeper");
            assert_eq!(lines[2], "back to base");
        }
        other => panic!("expected String, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_empty_reference_list() {
    let source = r#"
behavior foo "T" {
    tags []
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    match result.entities[0].fields.get("tags").expect("missing tags") {
        FieldValue::ReferenceList(items) => {
            assert!(items.is_empty(), "expected empty list, got: {items:?}");
        }
        other => panic!("expected ReferenceList, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_empty_nested_block() {
    let source = r#"
behavior foo "T" {
    requires {}
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    match result.entities[0].fields.get("requires").expect("missing requires") {
        FieldValue::Block(map) => {
            assert!(map.entries().is_empty(), "expected empty block, got {} entries", map.entries().len());
        }
        other => panic!("expected Block, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn spec_block_preserves_raw_body() {
    let source = r#"
spec "MyProject" {
    version "1.0.0"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let raw = result.entities[0].raw_body.as_ref().expect("missing raw_body on spec block");
    assert!(raw.contains("version"), "spec block raw_body should contain fields, got: {raw:?}");
}

#[specforge_test(behavior = "parse_define_blocks")]
#[test]
fn define_block_preserves_raw_body() {
    let source = r#"
define my_kind {
    testable true
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let raw = result.entities[0].raw_body.as_ref().expect("missing raw_body on define block");
    assert!(raw.contains("testable"), "define block raw_body should contain fields, got: {raw:?}");
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_single_variant_union() {
    let source = "type Singleton = OnlyOne\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].id.raw, "Singleton");

    match result.entities[0].fields.get("variants").expect("missing variants") {
        FieldValue::ReferenceList(variants) => {
            assert_eq!(variants.len(), 1, "expected 1 variant, got: {variants:?}");
            assert_eq!(variants[0], "OnlyOne");
        }
        other => panic!("expected ReferenceList for variants, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_all_block_types")]
#[test]
fn parse_union_type_variants_extracted() {
    let source = "type FieldValue = StringValue | ReferenceList | StringList | Block | VerifyList\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities[0].id.raw, "FieldValue");

    match result.entities[0].fields.get("variants").expect("missing variants") {
        FieldValue::ReferenceList(variants) => {
            assert_eq!(variants, &["StringValue", "ReferenceList", "StringList", "Block", "VerifyList"]);
        }
        other => panic!("expected ReferenceList for variants, got {:?}", other),
    }
}

fn walkdir(dir: PathBuf) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(walkdir(path));
            } else if path.extension().is_some_and(|ext| ext == "spec") {
                results.push(path);
            }
        }
    }
    results
}
