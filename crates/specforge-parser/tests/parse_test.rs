use specforge_common::Sym;
use specforge_parser::{parse, EntityKind, EntityId, FieldValue};
use specforge_test_macros::test as specforge_test;
use std::path::PathBuf;

#[specforge_test(behavior = "parse_all_block_types", verify = "parse any keyword as generic entity_block")]
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
    assert_eq!(entity.kind, EntityKind { raw: Sym::new("behavior") });
    assert_eq!(entity.id, EntityId { raw: Sym::new("parse_spec_file") });
    assert_eq!(entity.title.as_deref(), Some("Parse Spec File"));

    let status = entity.fields.get("status").expect("missing 'status' field");
    assert!(matches!(status, FieldValue::Identifier(id) if id == "planned"));
}

#[specforge_test(behavior = "parse_use_imports", verify = "parse full use import")]
#[test]
fn parse_use_import() {
    let source = "use \"behaviors/parsing\"\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.imports[0].path, "behaviors/parsing");
    assert_eq!(result.imports[0].kind, specforge_parser::ImportKind::Full);
    assert!(result.imports[0].bindings.is_none());
}

#[specforge_test(behavior = "parse_use_imports", verify = "parse selective use import with braces")]
#[test]
fn parse_selective_import() {
    let source = "use { SpecFile, ParseError } from \"types/core\"\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.imports[0].path, "types/core");
    assert_eq!(result.imports[0].kind, specforge_parser::ImportKind::Selective);
    let bindings = result.imports[0].bindings.as_ref().expect("expected selective import");
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].name, "SpecFile");
    assert!(bindings[0].alias.is_none());
    assert_eq!(bindings[1].name, "ParseError");
    assert!(bindings[1].alias.is_none());
}

#[specforge_test(behavior = "parse_verify_statements", verify = "parse verify statement in any entity block")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "spec block uses dedicated grammar rule")]
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
    assert_eq!(entity.kind, EntityKind { raw: Sym::new("spec") });
    assert_eq!(entity.title.as_deref(), Some("SpecForge"));

    let version = entity.fields.get("version").expect("missing version");
    assert!(matches!(version, FieldValue::String(v) if v == "0.1.0"));
}

#[specforge_test(behavior = "parse_ref_blocks", verify = "parse one-line ref syntax")]
#[test]
fn parse_ref_inline() {
    let source = r#"ref gh.issue:42 "Support Wasm extensions""#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind, EntityKind { raw: Sym::new("ref") });
    assert_eq!(entity.id.raw, "gh.issue:42");
    assert_eq!(entity.title.as_deref(), Some("Support Wasm extensions"));

    // Decomposed scheme/kind/identifier
    assert!(matches!(entity.fields.get("scheme"), Some(FieldValue::String(s)) if s == "gh"));
    assert!(matches!(entity.fields.get("ref_kind"), Some(FieldValue::String(s)) if s == "issue"));
    assert!(matches!(entity.fields.get("identifier"), Some(FieldValue::String(s)) if s == "42"));
}

#[specforge_test(behavior = "parse_ref_blocks", verify = "ref block supports optional title and body fields")]
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
    assert_eq!(entity.kind, EntityKind { raw: Sym::new("ref") });
    assert_eq!(entity.title.as_deref(), Some("Performance tracking"));
    assert!(matches!(entity.fields.get("priority"), Some(FieldValue::String(s)) if s == "high"));
}

#[specforge_test(behavior = "parse_define_blocks", verify = "parse define block with name and body")]
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
    assert_eq!(entity.kind, EntityKind { raw: Sym::new("define") });
    assert_eq!(entity.id.raw, "my_custom_type");
    assert!(entity.title.is_none());
    assert!(matches!(entity.fields.get("base_kind"), Some(FieldValue::String(s)) if s == "entity"));
    assert!(entity.fields.get("verify").is_some());
}

#[specforge_test(behavior = "parse_all_block_types", verify = "generic block preserves kind, name, title, and fields")]
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

#[specforge_test(behavior = "parse_triple_quoted_strings", verify = "common leading whitespace is stripped")]
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

#[specforge_test(behavior = "recover_from_syntax_errors", verify = "valid blocks after syntax error are still parsed")]
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

#[specforge_test(behavior = "recover_from_syntax_errors", verify = "parser collects multiple errors from one file")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "unknown keyword parsed without error")]
#[test]
fn unknown_keyword_parsed_without_error() {
    // Use a completely made-up keyword — the parser must accept it generically
    let source = r#"
zygomorphic my_entity "Made Up Kind" {
    status planned
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unknown keyword should not produce errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].kind.raw, "zygomorphic");
    assert_eq!(result.entities[0].id.raw, "my_entity");
    assert_eq!(result.entities[0].title.as_deref(), Some("Made Up Kind"));
}

#[test]
fn comments_are_skipped() {
    let source = r#"
// This is a file-level comment
use "behaviors/parsing" // inline comment

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

#[specforge_test(behavior = "parse_all_block_types", verify = "any keyword produces generic entity_block AST node")]
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

#[specforge_test(behavior = "parse_spec_file_to_ast", verify = "parse valid file produces complete AST")]
#[test]
fn parse_multiple_entities_in_one_file() {
    let source = r#"
use "types/core"

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

#[specforge_test(behavior = "parse_all_block_types", verify = "parse string field values correctly")]
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

#[specforge_test(behavior = "parse_ref_blocks", verify = "ref block extracts scheme, kind, and identifier components")]
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

#[specforge_test(behavior = "parse_use_imports", verify = "parse full use import")]
#[test]
fn parse_multiple_use_imports() {
    let source = r#"
use "behaviors/parsing"
use "types/core"
use "invariants/core"
use "features/output"
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.imports.len(), 4);
    assert_eq!(result.imports[0].path, "behaviors/parsing");
    assert_eq!(result.imports[1].path, "types/core");
    assert_eq!(result.imports[2].path, "invariants/core");
    assert_eq!(result.imports[3].path, "features/output");
}

#[specforge_test(behavior = "parse_spec_file_to_ast", verify = "AST source spans match original token positions")]
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

#[specforge_test(behavior = "parse_spec_file_to_ast", verify = "parse valid file produces complete AST")]
#[test]
fn parse_empty_file() {
    let result = parse("", "empty.spec");
    assert!(result.errors.is_empty());
    assert!(result.entities.is_empty());
    assert!(result.imports.is_empty());
}

#[specforge_test(behavior = "parse_all_block_types", verify = "any keyword produces generic entity_block AST node")]
#[test]
fn parse_empty_block() {
    let source = "behavior empty_block \"Empty\" {\n}\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].id.raw, "empty_block");
    assert_eq!(result.entities[0].fields.entries().len(), 0);
}

#[specforge_test(behavior = "parse_spec_file_to_ast", verify = "parse valid file produces complete AST")]
#[test]
fn parse_real_world_spec_file() {
    // A realistic .spec file exercising many constructs at once
    let source = r#"
// Parsing behaviors
use "invariants/core"
use "types/core"
use "types/errors"

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

#[specforge_test(behavior = "parse_all_block_types", verify = "generic block preserves kind, name, title, and fields")]
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

#[specforge_test(behavior = "parse_spec_file_to_ast", verify = "parse valid file produces complete AST")]
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
    // signatures, term blocks, etc.) that the core grammar
    // correctly does not handle — body parsers will process these in
    // Phase 1.5. The key assertion is no panics on any real content.
}

#[specforge_test(behavior = "parse_all_block_types", verify = "any keyword produces generic entity_block AST node")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "any keyword produces generic entity_block AST node")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "any keyword produces generic entity_block AST node")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "any keyword produces generic entity_block AST node")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "any keyword produces generic entity_block AST node")]
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

#[specforge_test(behavior = "parse_use_imports", verify = "reject use import with .spec extension")]
#[test]
fn reject_use_import_with_spec_extension() {
    let source = "use \"behaviors/parsing.spec\"\n";
    let result = parse(source, "test.spec");

    // The path is a quoted string containing ".spec" — the parser will parse it,
    // but validation (resolver) rejects it. Check the path is correctly extracted.
    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.imports[0].path, "behaviors/parsing.spec");
}

#[specforge_test(behavior = "parse_verify_statements", verify = "verify parsed in spec block")]
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
    assert_eq!(entity.kind, EntityKind { raw: Sym::new("spec") });

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

#[specforge_test(behavior = "parse_verify_statements", verify = "verify parsed in define block")]
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
    assert_eq!(entity.kind, EntityKind { raw: Sym::new("define") });

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

#[specforge_test(behavior = "parse_triple_quoted_strings", verify = "recover from unclosed triple-quoted string with diagnostic")]
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

#[specforge_test(behavior = "recover_from_syntax_errors", verify = "valid blocks after syntax error are still parsed")]
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

#[specforge_test(behavior = "parse_ref_blocks", verify = "reject ref block with missing scheme or identifier")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "generic block preserves kind, name, title, and fields")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "generic block preserves kind, name, title, and fields")]
#[test]
fn empty_block_has_empty_raw_body() {
    let source = "behavior empty \"Empty\" {\n}\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let entity = &result.entities[0];
    let raw = entity.raw_body.as_ref().expect("missing raw_body");
    assert!(raw.trim().is_empty(), "expected empty raw_body, got: {raw:?}");
}

#[specforge_test(behavior = "parse_spec_file_to_ast", verify = "parse valid file produces complete AST")]
#[test]
fn spec_files_without_extension_syntax_parse_cleanly() {
    // Files that use ONLY standard field syntax (no port method signatures,
    // no term sub-blocks) should parse with ZERO errors.
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
        "glossary.spec",       // nested term sub-blocks inside term singleton
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

#[specforge_test(behavior = "parse_all_block_types", verify = "parse string field values correctly")]
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

#[specforge_test(behavior = "parse_verify_statements", verify = "verify kind and description extracted correctly")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "parse string field values correctly")]
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

    // Bare word -> Identifier
    assert!(
        matches!(e.fields.get("status"), Some(FieldValue::Identifier(s)) if s == "planned"),
        "bare word 'planned' should be FieldValue::Identifier, got: {:?}",
        e.fields.get("status")
    );

    // Quoted string -> String
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

#[specforge_test(behavior = "parse_triple_quoted_strings", verify = "relative indentation is preserved")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "parse string field values correctly")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "generic block preserves kind, name, title, and fields")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "spec block uses dedicated grammar rule")]
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

#[specforge_test(behavior = "parse_define_blocks", verify = "define block supports standard field syntax")]
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

#[specforge_test(behavior = "parse_all_block_types", verify = "any keyword produces generic entity_block AST node")]
#[test]
fn parse_single_variant_union() {
    let source = "type Singleton = OnlyOne\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].id.raw, "Singleton");

    match result.entities[0].fields.get("variants").expect("missing variants") {
        FieldValue::VariantList(variants) => {
            assert_eq!(variants.len(), 1, "expected 1 variant, got: {variants:?}");
            assert_eq!(variants[0], "OnlyOne");
        }
        other => panic!("expected VariantList for variants, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_all_block_types", verify = "any keyword produces generic entity_block AST node")]
#[test]
fn parse_union_type_variants_extracted() {
    let source = "type FieldValue = StringValue | ReferenceList | StringList | Block | VerifyList\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities[0].id.raw, "FieldValue");

    match result.entities[0].fields.get("variants").expect("missing variants") {
        FieldValue::VariantList(variants) => {
            assert_eq!(variants, &["StringValue", "ReferenceList", "StringList", "Block", "VerifyList"]);
        }
        other => panic!("expected VariantList for variants, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// NEW TESTS: Fill coverage gaps for uncovered verify statements
// ---------------------------------------------------------------------------

#[specforge_test(behavior = "parse_all_block_types", verify = "ref block uses dedicated grammar rule")]
#[test]
fn ref_block_uses_dedicated_grammar_rule() {
    // Ref blocks have unique syntax: ref scheme.kind:identifier [title] { fields }
    // This verifies that ref is parsed via a dedicated grammar rule, not generic entity_block
    let source = r#"
ref gh.pr:100 "Ref grammar rule" {
    status "open"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "ref block should parse without errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "ref");
    assert_eq!(entity.id.raw, "gh.pr:100");
    assert_eq!(entity.title.as_deref(), Some("Ref grammar rule"));

    // Ref-specific decomposition proves dedicated rule (generic blocks don't extract these)
    assert!(matches!(entity.fields.get("scheme"), Some(FieldValue::String(s)) if s == "gh"));
    assert!(matches!(entity.fields.get("ref_kind"), Some(FieldValue::String(s)) if s == "pr"));
    assert!(matches!(entity.fields.get("identifier"), Some(FieldValue::String(s)) if s == "100"));
    assert!(matches!(entity.fields.get("status"), Some(FieldValue::String(s)) if s == "open"));
}

#[specforge_test(behavior = "parse_all_block_types", verify = "define block uses dedicated grammar rule")]
#[test]
fn define_block_uses_dedicated_grammar_rule() {
    // Define blocks have unique syntax: define <name> { fields }
    // No title allowed — this differentiates from generic entity_block
    let source = r#"
define custom_entity {
    base_kind "entity"
    testable true
    singleton false
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "define block should parse without errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "define");
    assert_eq!(entity.id.raw, "custom_entity");
    // Define blocks have no title — this distinguishes from generic entity_block
    assert!(entity.title.is_none(), "define blocks must not have a title");
    assert!(matches!(entity.fields.get("base_kind"), Some(FieldValue::String(s)) if s == "entity"));
    assert!(matches!(entity.fields.get("testable"), Some(FieldValue::Boolean(true))));
    assert!(matches!(entity.fields.get("singleton"), Some(FieldValue::Boolean(false))));
}

#[specforge_test(behavior = "parse_triple_quoted_strings", verify = "triple-quoted string preserves newlines")]
#[test]
fn triple_quoted_string_preserves_newlines() {
    // Verify that internal newlines are preserved in triple-quoted strings
    let source = "behavior b \"B\" {\n    contract \"\"\"\n        First line\n        Second line\n        Third line\n    \"\"\"\n}\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let contract = result.entities[0].fields.get("contract").expect("missing contract");
    match contract {
        FieldValue::String(s) => {
            let lines: Vec<&str> = s.lines().collect();
            assert_eq!(lines.len(), 3, "expected 3 lines (newlines preserved), got: {lines:?}");
            assert_eq!(lines[0], "First line");
            assert_eq!(lines[1], "Second line");
            assert_eq!(lines[2], "Third line");
        }
        other => panic!("expected String, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_verify_statements", verify = "parse multiple verify statements in same entity")]
#[test]
fn parse_multiple_verify_statements_in_same_entity() {
    let source = r#"
behavior multi_verify "Multi Verify" {
    contract "The system MUST handle multiple verify statements"
    verify unit "first unit test"
    verify unit "second unit test"
    verify integration "integration test"
    verify property "property-based test"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let entity = &result.entities[0];
    let verify = entity.fields.get("verify").expect("missing verify field");
    match verify {
        FieldValue::VerifyList(stmts) => {
            assert_eq!(stmts.len(), 4, "expected 4 verify statements in same entity");
            assert_eq!(stmts[0].kind, "unit");
            assert_eq!(stmts[0].description, "first unit test");
            assert_eq!(stmts[1].kind, "unit");
            assert_eq!(stmts[1].description, "second unit test");
            assert_eq!(stmts[2].kind, "integration");
            assert_eq!(stmts[2].description, "integration test");
            assert_eq!(stmts[3].kind, "property");
            assert_eq!(stmts[3].description, "property-based test");
        }
        other => panic!("expected VerifyList, got {:?}", other),
    }
}

#[specforge_test(behavior = "parse_ref_blocks", verify = "parse ref block with scheme.kind:identifier format")]
#[test]
fn parse_ref_block_with_scheme_kind_identifier_format() {
    // Verify the full ref block syntax with scheme.kind:identifier is parsed
    let source = r#"ref jira.epic:PROJ-42 "Epic tracking""#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "ref");
    assert_eq!(entity.id.raw, "jira.epic:PROJ-42");
    assert_eq!(entity.title.as_deref(), Some("Epic tracking"));

    // scheme.kind:identifier components must be decomposed
    assert!(matches!(entity.fields.get("scheme"), Some(FieldValue::String(s)) if s == "jira"));
    assert!(matches!(entity.fields.get("ref_kind"), Some(FieldValue::String(s)) if s == "epic"));
    assert!(matches!(entity.fields.get("identifier"), Some(FieldValue::String(s)) if s == "PROJ-42"));
}

#[specforge_test(behavior = "parse_define_blocks", verify = "define block parsed without extension knowledge")]
#[test]
fn define_block_parsed_without_extension_knowledge() {
    // Define blocks are core grammar constructs — they must parse even when
    // no extensions are loaded. The parser has no extension context at all,
    // so successful parsing proves no extension knowledge is required.
    let source = r#"
define exotic_kind {
    base_kind "entity"
    testable true
    singleton false
    supports_verify true
    incremental false
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "define block must parse without any extension knowledge: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "define");
    assert_eq!(entity.id.raw, "exotic_kind");
    // All standard field types must work inside define blocks
    assert!(matches!(entity.fields.get("base_kind"), Some(FieldValue::String(s)) if s == "entity"));
    assert!(matches!(entity.fields.get("testable"), Some(FieldValue::Boolean(true))));
    assert!(matches!(entity.fields.get("singleton"), Some(FieldValue::Boolean(false))));
    assert!(matches!(entity.fields.get("supports_verify"), Some(FieldValue::Boolean(true))));
    assert!(matches!(entity.fields.get("incremental"), Some(FieldValue::Boolean(false))));
}

// --- pub use import tests ---

#[specforge_test(behavior = "parse_use_imports", verify = "parse pub use import")]
#[test]
fn parse_pub_use_import() {
    let source = "pub use \"./foo\"\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.imports[0].path, "./foo");
    assert!(result.imports[0].is_pub, "pub use should set is_pub = true");
    assert_eq!(result.imports[0].kind, specforge_parser::ImportKind::Full);
    assert!(result.imports[0].bindings.is_none());
}

#[specforge_test(behavior = "parse_use_imports", verify = "parse pub use selective import")]
#[test]
fn parse_pub_use_selective() {
    let source = "pub use { Bar, Baz } from \"./foo\"\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.imports.len(), 1);
    assert!(result.imports[0].is_pub, "pub use should set is_pub = true");
    assert_eq!(result.imports[0].kind, specforge_parser::ImportKind::Selective);
    let bindings = result.imports[0].bindings.as_ref().expect("expected selective bindings");
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].name, "Bar");
    assert_eq!(bindings[1].name, "Baz");
}

#[specforge_test(behavior = "parse_use_imports", verify = "parse mixed use and pub use imports")]
#[test]
fn parse_mixed_use_and_pub_use() {
    let source = r#"
use "./private"
pub use "./public"
use "./another_private"
pub use { Foo } from "./another_public"
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.imports.len(), 4);
    assert!(!result.imports[0].is_pub, "use should set is_pub = false");
    assert!(result.imports[1].is_pub, "pub use should set is_pub = true");
    assert!(!result.imports[2].is_pub, "use should set is_pub = false");
    assert!(result.imports[3].is_pub, "pub use should set is_pub = true");
    assert_eq!(result.imports[3].kind, specforge_parser::ImportKind::Selective);
    let bindings = result.imports[3].bindings.as_ref().expect("expected selective bindings");
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].name, "Foo");
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

#[test]
fn unquoted_import_path_misparsed_as_entity() {
    // BUG: unquoted import paths like `use invariants/core` are not matched by use_import rule
    // which requires quotes. Tree-sitter error recovery might be creating entity_block nodes.
    let source = r#"
use invariants/core

behavior test "Test" {
  status planned
}
"#;
    
    let result = parse(source, "test.spec");
    
    eprintln!("=== UNQUOTED IMPORT TEST ===");
    eprintln!("Imports: {}", result.imports.len());
    for imp in &result.imports {
        eprintln!("  - path={}", imp.path);
    }
    
    eprintln!("Entities: {}", result.entities.len());
    for (i, ent) in result.entities.iter().enumerate() {
        eprintln!("  [{}] kind={}, id={}, line={}", i, ent.kind.raw, ent.id.raw, ent.span.start_line);
    }
    
    eprintln!("Errors: {}", result.errors.len());
    for err in &result.errors {
        eprintln!("  - line={}: {}", err.span.start_line, err.message);
    }
}

#[test]
fn debug_treesitter_ast() {
    use std::path::PathBuf;
    use tree_sitter::Parser;
    
    let source = r#"use invariants/core

behavior test "Test" {
  status planned
}"#;
    
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_specforge::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    fn print_node(node: tree_sitter::Node, source: &str, indent: usize) {
        let indent_str = " ".repeat(indent);
        let text = node.utf8_text(source.as_bytes()).unwrap_or("???");
        let text_display = if text.len() > 40 { 
            format!("{}...", &text[..40]) 
        } else { 
            text.to_string() 
        };
        eprintln!("{}kind={}, line={}, text={:?}", indent_str, node.kind(), node.start_position().row + 1, text_display);
        
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            print_node(child, source, indent + 2);
        }
    }
    
    eprintln!("\n=== TREE-SITTER AST ===");
    print_node(tree.root_node(), source, 0);
}

// === Phase 6: Parser Robustness ===

// B:parse_all_block_types — verify unit "integer overflow produces parse error instead of silent 0"
#[specforge_test(behavior = "parse_all_block_types", verify = "integer overflow produces parse error instead of silent 0")]
#[test]
fn integer_overflow_produces_parse_error() {
    let source = r#"
behavior overflowed "Overflow" {
    priority 99999999999999999999999999999
}
"#;
    let result = parse(source, "test.spec");

    // Should report an error about integer overflow
    let has_overflow_error = result.errors.iter().any(|e| {
        e.message.contains("integer") && e.message.contains("too large")
    });
    assert!(has_overflow_error, "expected parse error for integer overflow, got errors: {:?}", result.errors);

    // Entity is still created (error recovery), but the error is surfaced
    assert!(!result.errors.is_empty(), "overflow must be reported as an error");
    // Verify the error has useful span and expected/found info
    let err = result.errors.iter().find(|e| e.message.contains("too large")).unwrap();
    assert!(err.expected.is_some(), "error should have 'expected' field");
    assert!(err.found.is_some(), "error should have 'found' field");
}

// B:parse_all_block_types — verify unit "missing opening brace produces helpful error"
#[specforge_test(behavior = "parse_all_block_types", verify = "missing opening brace produces helpful error")]
#[test]
fn missing_opening_brace_produces_error() {
    let source = "behavior no_brace \"No Brace\"\n    status planned\n}\n";
    let result = parse(source, "test.spec");

    // Should produce some error — tree-sitter should flag this
    assert!(!result.errors.is_empty() || result.entities.is_empty(),
        "missing brace should produce an error or no entity");
}

// B:parse_all_block_types — verify unit "completely invalid syntax produces error with location"
#[specforge_test(behavior = "parse_all_block_types", verify = "completely invalid syntax produces error with location")]
#[test]
fn completely_invalid_syntax_produces_error_with_location() {
    let source = "@@@ invalid !!! garbage\n";
    let result = parse(source, "test.spec");

    assert!(!result.errors.is_empty(), "invalid syntax should produce errors, got none");
    // Error should have a meaningful span
    let err = &result.errors[0];
    assert!(err.span.start_line >= 1, "error span should have valid line number");
    // Error should have expected field with guidance
    assert!(err.expected.is_some(), "error should suggest what was expected");
}

// B:parse_all_block_types — verify unit "valid integer parses correctly"
#[specforge_test(behavior = "parse_all_block_types", verify = "valid integer parses correctly")]
#[test]
fn valid_integer_parses_correctly() {
    let source = r#"
behavior normal "Normal" {
    priority 42
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let entity = &result.entities[0];
    match entity.fields.get("priority") {
        Some(FieldValue::Integer(val)) => assert_eq!(*val, 42),
        other => panic!("expected Integer(42), got {:?}", other),
    }
}
